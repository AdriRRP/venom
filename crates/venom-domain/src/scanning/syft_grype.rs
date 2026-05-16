use crate::{
    ArtifactKind, EvidenceFreshness, FindingProvider, FindingProviderError,
    FindingProviderErrorKind, PackageCoordinate, ProviderScanReport, ReportedFinding, ScanRequest,
    Severity, as_provider_error, validate_provider_scan_report,
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use time::OffsetDateTime;
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

/// Stable provider key for the Syft + Grype adapter path.
pub const SYFT_GRYPE_PROVIDER_KEY: &str = "syft-grype";

/// Official Syft image used by the live Docker-backed runner.
pub const OFFICIAL_SYFT_IMAGE: &str = "ghcr.io/anchore/syft:v1.44.0";

/// Official Grype image used by the live Docker-backed runner.
pub const OFFICIAL_GRYPE_IMAGE: &str = "ghcr.io/anchore/grype:v0.112.0";

/// Default timeout applied to each live provider process.
pub const DEFAULT_LIVE_COMMAND_TIMEOUT: Duration = Duration::from_mins(1);

/// Maximum stderr payload kept in one provider failure message.
pub const MAX_ERROR_TEXT_BYTES: usize = 2048;

/// Provider backed by committed Syft and Grype fixture outputs.
///
/// This is the deterministic lane for the real adapter. It exercises the same
/// mapping code as the live runner, but with fixed scanner outputs stored in
/// the repository.
pub struct FixtureSyftGrypeProvider {
    deterministic: FixtureBundle,
    live: FixtureBundle,
}

impl FixtureSyftGrypeProvider {
    #[must_use]
    pub const fn new(deterministic: FixtureBundle, live: FixtureBundle) -> Self {
        Self {
            deterministic,
            live,
        }
    }

    /// Load deterministic and live fixture bundles from committed JSON files.
    ///
    /// # Errors
    ///
    /// Returns an error when any fixture file cannot be read or parsed later by
    /// the adapter path.
    pub fn from_paths(
        deterministic_syft: impl AsRef<Path>,
        deterministic_grype: impl AsRef<Path>,
        live_syft: impl AsRef<Path>,
        live_grype: impl AsRef<Path>,
    ) -> Result<Self, FindingProviderError> {
        Ok(Self::new(
            FixtureBundle::from_paths(deterministic_syft, deterministic_grype)?,
            FixtureBundle::from_paths(live_syft, live_grype)?,
        ))
    }

    const fn bundle_for(&self, freshness: EvidenceFreshness) -> &FixtureBundle {
        match freshness {
            EvidenceFreshness::Deterministic => &self.deterministic,
            EvidenceFreshness::Live => &self.live,
        }
    }
}

impl FindingProvider for FixtureSyftGrypeProvider {
    fn provider_key(&self) -> &'static str {
        SYFT_GRYPE_PROVIDER_KEY
    }

    async fn scan<'a>(
        &'a self,
        request: &'a ScanRequest,
    ) -> Result<ProviderScanReport, FindingProviderError> {
        let bundle = self.bundle_for(request.freshness);
        let report = build_report_from_bundle(request, bundle)?;
        validate_provider_scan_report(self.provider_key(), request, &report)
            .map_err(as_provider_error)?;
        Ok(report)
    }
}

/// Provider backed by the official Docker images for Syft and Grype.
///
/// This is the live lane: it runs real scanners over an immutable container
/// image reference and maps the resulting JSON into VENOM's canonical report.
pub struct DockerSyftGrypeProvider {
    syft_image: Box<str>,
    grype_image: Box<str>,
    command_timeout: Duration,
}

impl DockerSyftGrypeProvider {
    #[must_use]
    pub fn new(syft_image: impl Into<Box<str>>, grype_image: impl Into<Box<str>>) -> Self {
        Self {
            syft_image: syft_image.into(),
            grype_image: grype_image.into(),
            command_timeout: DEFAULT_LIVE_COMMAND_TIMEOUT,
        }
    }

    #[must_use]
    pub fn official() -> Self {
        Self::new(OFFICIAL_SYFT_IMAGE, OFFICIAL_GRYPE_IMAGE)
    }

    #[must_use]
    pub const fn with_timeout(mut self, command_timeout: Duration) -> Self {
        self.command_timeout = command_timeout;
        self
    }
}

impl FindingProvider for DockerSyftGrypeProvider {
    fn provider_key(&self) -> &'static str {
        SYFT_GRYPE_PROVIDER_KEY
    }

    async fn scan<'a>(
        &'a self,
        request: &'a ScanRequest,
    ) -> Result<ProviderScanReport, FindingProviderError> {
        if request.freshness != EvidenceFreshness::Live {
            return Err(FindingProviderError::new(
                FindingProviderErrorKind::InvalidRequest,
                false,
                "docker-backed syft-grype provider only supports live freshness",
            ));
        }
        if request.artifact.kind != ArtifactKind::ContainerImage {
            return Err(FindingProviderError::new(
                FindingProviderErrorKind::InvalidRequest,
                false,
                "docker-backed syft-grype provider currently supports only container images",
            ));
        }

        let syft_stdout = run_command(
            docker_syft_command(&self.syft_image, request),
            None,
            "syft",
            self.command_timeout,
        )
        .await?;
        let grype_stdout = run_command(
            docker_grype_command(&self.grype_image),
            Some(&syft_stdout),
            "grype",
            self.command_timeout,
        )
        .await?;

        let report = build_report_from_json_bytes(request, &syft_stdout, &grype_stdout)?;
        validate_provider_scan_report(self.provider_key(), request, &report)
            .map_err(as_provider_error)?;
        Ok(report)
    }
}

/// Raw Syft + Grype fixture bundle.
#[derive(Debug, Clone)]
pub struct FixtureBundle {
    syft_json: Box<str>,
    grype_json: Box<str>,
}

impl FixtureBundle {
    /// Load one Syft JSON document and one Grype JSON report from disk.
    ///
    /// # Errors
    ///
    /// Returns an error when either file cannot be read.
    pub fn from_paths(
        syft_path: impl AsRef<Path>,
        grype_path: impl AsRef<Path>,
    ) -> Result<Self, FindingProviderError> {
        Ok(Self::from_strings(
            read_text_file(syft_path.as_ref(), "syft fixture")?,
            read_text_file(grype_path.as_ref(), "grype fixture")?,
        ))
    }

    #[must_use]
    pub fn from_strings(syft_json: String, grype_json: String) -> Self {
        Self {
            syft_json: syft_json.into_boxed_str(),
            grype_json: grype_json.into_boxed_str(),
        }
    }

    fn syft_document(&self) -> Result<SyftDocument, FindingProviderError> {
        parse_syft_document(&self.syft_json)
    }

    fn grype_report(&self) -> Result<GrypeReport, FindingProviderError> {
        parse_grype_report(&self.grype_json)
    }
}

/// Derive the canonical immutable image identity from a Syft JSON document.
///
/// This prefers repo digests when present because they are the most directly
/// usable immutable references for later live re-scans.
///
/// # Errors
///
/// Returns an error when the text is not valid Syft JSON or when the Syft
/// source does not expose an immutable image identity.
pub fn artifact_identity_from_syft_json(syft_json: &str) -> Result<Box<str>, FindingProviderError> {
    let document = parse_syft_document(syft_json)?;
    canonical_image_identity(&document)
}

fn build_report_from_bundle(
    request: &ScanRequest,
    bundle: &FixtureBundle,
) -> Result<ProviderScanReport, FindingProviderError> {
    let syft = bundle.syft_document()?;
    let grype = bundle.grype_report()?;
    build_report_from_documents(request, syft, grype)
}

fn build_report_from_json_bytes(
    request: &ScanRequest,
    syft_json: &[u8],
    grype_json: &[u8],
) -> Result<ProviderScanReport, FindingProviderError> {
    let syft = parse_syft_document_bytes(syft_json)?;
    let grype = parse_grype_report_bytes(grype_json)?;
    build_report_from_documents(request, syft, grype)
}

fn build_report_from_documents(
    request: &ScanRequest,
    syft: SyftDocument,
    grype: GrypeReport,
) -> Result<ProviderScanReport, FindingProviderError> {
    ensure_request_matches_sources(request, &syft, &grype)?;

    let observed_at = grype
        .descriptor
        .timestamp
        .as_deref()
        .map(parse_rfc3339_timestamp)
        .transpose()?
        .unwrap_or(UNIX_EPOCH);
    let findings = grype.matches.iter().map(to_reported_finding).collect();

    let mut report = ProviderScanReport::new(
        SYFT_GRYPE_PROVIDER_KEY,
        request.component_key.clone(),
        request.artifact.clone(),
        observed_at,
        request.freshness,
        findings,
    );

    if let Some(revision) = grype.descriptor.knowledge_revision() {
        report = report.with_knowledge_revision(revision);
    }

    Ok(report)
}

fn to_reported_finding(entry: &GrypeMatch) -> ReportedFinding {
    let mut finding = ReportedFinding::new(
        entry.vulnerability.id.clone(),
        to_package_coordinate(&entry.artifact),
    )
    .with_severity(normalize_severity(entry.vulnerability.severity.as_deref()))
    .with_provider_finding_key(format!(
        "grype:{}:{}",
        entry.artifact.id, entry.vulnerability.id
    ));

    if let Some(fix_version) = entry.vulnerability.fix.first_version() {
        finding = finding.with_fix_version(fix_version);
    }
    for alias in entry.vulnerability.aliases() {
        finding = finding.with_alias(alias);
    }

    finding
}

fn to_package_coordinate(artifact: &GrypeArtifact) -> PackageCoordinate {
    let coordinate = PackageCoordinate::new(artifact.name.clone(), artifact.version.clone());
    match artifact.purl.as_deref() {
        Some(purl) => coordinate.with_purl(purl),
        None => coordinate,
    }
}

fn ensure_request_matches_sources(
    request: &ScanRequest,
    syft: &SyftDocument,
    grype: &GrypeReport,
) -> Result<(), FindingProviderError> {
    let identity = request.artifact.identity.as_ref();
    let syft_matches = syft
        .identity_candidates()
        .iter()
        .any(|value| value == identity);
    let grype_matches = grype
        .identity_candidates()
        .iter()
        .any(|value| value == identity);

    if !syft_matches {
        return Err(FindingProviderError::new(
            FindingProviderErrorKind::CorruptResponse,
            false,
            format!("syft source did not match requested artifact identity `{identity}`"),
        ));
    }
    if !grype_matches {
        return Err(FindingProviderError::new(
            FindingProviderErrorKind::CorruptResponse,
            false,
            format!("grype source did not match requested artifact identity `{identity}`"),
        ));
    }

    Ok(())
}

fn canonical_image_identity(syft: &SyftDocument) -> Result<Box<str>, FindingProviderError> {
    syft.identity_candidates()
        .into_iter()
        .next()
        .map(Into::into)
        .ok_or_else(|| {
            FindingProviderError::new(
                FindingProviderErrorKind::CorruptResponse,
                false,
                "syft source did not expose a canonical immutable image identity",
            )
        })
}

fn docker_syft_command(image: &str, request: &ScanRequest) -> Command {
    let mut command = Command::new("docker");
    command.args([
        "run",
        "--rm",
        image,
        request.artifact.identity.as_ref(),
        "-o",
        "syft-json",
    ]);
    command
}

fn docker_grype_command(image: &str) -> Command {
    let mut command = Command::new("docker");
    command.args(["run", "--rm", "-i", image, "-o", "json"]);
    command
}

async fn run_command(
    mut command: Command,
    stdin_bytes: Option<&[u8]>,
    tool_name: &'static str,
    command_timeout: Duration,
) -> Result<Vec<u8>, FindingProviderError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command.kill_on_drop(true);
    if stdin_bytes.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn().map_err(|error| {
        FindingProviderError::new(
            FindingProviderErrorKind::Unavailable,
            true,
            format!("failed to spawn {tool_name}: {error}"),
        )
    })?;

    if let Some(bytes) = stdin_bytes {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            FindingProviderError::new(
                FindingProviderErrorKind::Unavailable,
                true,
                format!("{tool_name} stdin was not available"),
            )
        })?;
        stdin.write_all(bytes).await.map_err(|error| {
            FindingProviderError::new(
                FindingProviderErrorKind::Unavailable,
                true,
                format!("failed to write stdin for {tool_name}: {error}"),
            )
        })?;
        drop(stdin);
    }

    let output = match timeout(command_timeout, child.wait_with_output()).await {
        Ok(result) => result.map_err(|error| {
            FindingProviderError::new(
                FindingProviderErrorKind::Unavailable,
                true,
                format!("failed while waiting for {tool_name}: {error}"),
            )
        })?,
        Err(_) => {
            return Err(FindingProviderError::new(
                FindingProviderErrorKind::Unavailable,
                true,
                format!(
                    "{tool_name} exceeded the live execution timeout of {}s",
                    command_timeout.as_secs()
                ),
            ));
        }
    };

    if output.status.success() {
        return Ok(output.stdout);
    }

    let stderr = bounded_error_text(&output.stderr);
    Err(FindingProviderError::new(
        FindingProviderErrorKind::Unavailable,
        true,
        format!(
            "{tool_name} exited with status {}: {}",
            output.status,
            stderr.trim()
        ),
    ))
}

fn bounded_error_text(bytes: &[u8]) -> String {
    let limit = bytes.len().min(MAX_ERROR_TEXT_BYTES);
    let truncated = &bytes[..limit];
    let mut text = String::from_utf8_lossy(truncated).trim().to_owned();
    if bytes.len() > MAX_ERROR_TEXT_BYTES {
        text.push_str("…[truncated]");
    }
    text
}

fn read_text_file(path: &Path, label: &str) -> Result<String, FindingProviderError> {
    fs::read_to_string(path).map_err(|error| {
        FindingProviderError::new(
            FindingProviderErrorKind::Unavailable,
            false,
            format!("failed to read {label} at {}: {error}", path.display()),
        )
    })
}

fn parse_syft_document(text: &str) -> Result<SyftDocument, FindingProviderError> {
    parse_syft_document_bytes(text.as_bytes())
}

fn parse_syft_document_bytes(bytes: &[u8]) -> Result<SyftDocument, FindingProviderError> {
    serde_json::from_slice(bytes).map_err(|error| {
        FindingProviderError::new(
            FindingProviderErrorKind::CorruptResponse,
            false,
            format!("failed to parse syft json: {error}"),
        )
    })
}

fn parse_grype_report(text: &str) -> Result<GrypeReport, FindingProviderError> {
    parse_grype_report_bytes(text.as_bytes())
}

fn parse_grype_report_bytes(bytes: &[u8]) -> Result<GrypeReport, FindingProviderError> {
    serde_json::from_slice(bytes).map_err(|error| {
        FindingProviderError::new(
            FindingProviderErrorKind::CorruptResponse,
            false,
            format!("failed to parse grype json: {error}"),
        )
    })
}

fn parse_rfc3339_timestamp(value: &str) -> Result<SystemTime, FindingProviderError> {
    OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map(SystemTime::from)
        .map_err(|error| {
            FindingProviderError::new(
                FindingProviderErrorKind::CorruptResponse,
                false,
                format!("failed to parse RFC3339 timestamp `{value}`: {error}"),
            )
        })
}

#[allow(clippy::missing_const_for_fn)]
fn normalize_severity(value: Option<&str>) -> Severity {
    match value {
        Some(value) if value.eq_ignore_ascii_case("none") => Severity::None,
        Some(value) if value.eq_ignore_ascii_case("low") => Severity::Low,
        Some(value) if value.eq_ignore_ascii_case("medium") => Severity::Medium,
        Some(value) if value.eq_ignore_ascii_case("high") => Severity::High,
        Some(value) if value.eq_ignore_ascii_case("critical") => Severity::Critical,
        _ => Severity::Unknown,
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SyftDocument {
    source: SyftSource,
}

impl SyftDocument {
    fn identity_candidates(&self) -> Vec<String> {
        let mut values = Vec::new();
        if let Some(repo_digest) = self.source.metadata.repo_digests.first() {
            values.push(repo_digest.to_string());
        }
        if let (Some(name), Some(manifest_digest)) = (
            self.source.name.as_deref(),
            self.source.metadata.manifest_digest.as_deref(),
        ) {
            values.push(format!("{name}@{manifest_digest}"));
        }
        values
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SyftSource {
    #[serde(default)]
    name: Option<Box<str>>,
    metadata: SyftSourceMetadata,
}

#[derive(Debug, Clone, Deserialize)]
struct SyftSourceMetadata {
    #[serde(default, rename = "repoDigests")]
    repo_digests: Vec<Box<str>>,
    #[serde(default, rename = "manifestDigest")]
    manifest_digest: Option<Box<str>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeReport {
    matches: Vec<GrypeMatch>,
    source: GrypeSource,
    descriptor: GrypeDescriptor,
}

impl GrypeReport {
    fn identity_candidates(&self) -> Vec<String> {
        let mut values = Vec::new();
        if let Some(repo_digest) = self.source.target.repo_digests.first() {
            values.push(repo_digest.to_string());
        }
        if let Some(manifest_digest) = self.source.target.manifest_digest.as_deref() {
            values.push(format!(
                "{}@{manifest_digest}",
                self.source.target.image_name()
            ));
        }
        values
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeSource {
    target: GrypeSourceTarget,
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeSourceTarget {
    #[serde(default, rename = "userInput")]
    user_input: Option<Box<str>>,
    #[serde(default, rename = "repoDigests")]
    repo_digests: Vec<Box<str>>,
    #[serde(default, rename = "manifestDigest")]
    manifest_digest: Option<Box<str>>,
}

impl GrypeSourceTarget {
    fn image_name(&self) -> &str {
        self.user_input
            .as_deref()
            .unwrap_or("unknown-image")
            .split('@')
            .next()
            .unwrap_or("unknown-image")
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeDescriptor {
    #[serde(default)]
    timestamp: Option<Box<str>>,
    db: GrypeDescriptorDb,
}

impl GrypeDescriptor {
    fn knowledge_revision(&self) -> Option<String> {
        let built = self.db.status.built.as_deref()?;
        let schema = self
            .db
            .status
            .schema_version
            .as_deref()
            .unwrap_or("unknown");
        Some(format!("grype-db:{schema}:{built}"))
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeDescriptorDb {
    status: GrypeDbStatus,
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeDbStatus {
    #[serde(default, rename = "schemaVersion")]
    schema_version: Option<Box<str>>,
    #[serde(default)]
    built: Option<Box<str>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeMatch {
    vulnerability: GrypeVulnerability,
    artifact: GrypeArtifact,
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeVulnerability {
    id: Box<str>,
    #[serde(default)]
    severity: Option<Box<str>>,
    #[serde(default)]
    fix: GrypeFix,
    #[serde(default, rename = "relatedVulnerabilities")]
    related_vulnerabilities: Vec<GrypeRelatedVulnerability>,
    #[serde(default)]
    advisories: Vec<GrypeAdvisory>,
}

impl GrypeVulnerability {
    fn aliases(&self) -> Vec<&str> {
        let mut values = BTreeSet::new();

        for advisory in &self.advisories {
            if advisory.id != self.id {
                values.insert(advisory.id.as_ref());
            }
        }
        for related in &self.related_vulnerabilities {
            if related.id != self.id {
                values.insert(related.id.as_ref());
            }
        }

        values.into_iter().collect()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct GrypeFix {
    #[serde(default)]
    versions: Vec<Box<str>>,
}

impl GrypeFix {
    fn first_version(&self) -> Option<&str> {
        self.versions.first().map(Box::as_ref)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeRelatedVulnerability {
    id: Box<str>,
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeAdvisory {
    id: Box<str>,
}

#[derive(Debug, Clone, Deserialize)]
struct GrypeArtifact {
    id: Box<str>,
    name: Box<str>,
    version: Box<str>,
    #[serde(default)]
    purl: Option<Box<str>>,
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_LIVE_COMMAND_TIMEOUT, DockerSyftGrypeProvider, FixtureSyftGrypeProvider,
        MAX_ERROR_TEXT_BYTES, OFFICIAL_GRYPE_IMAGE, OFFICIAL_SYFT_IMAGE,
        artifact_identity_from_syft_json, bounded_error_text, run_command,
    };
    use crate::{
        ArtifactKind, ArtifactRef, EvidenceFreshness, FindingProvider, FindingProviderErrorKind,
        validate_provider_scan_report,
    };
    use std::{fs, path::PathBuf, process::Stdio, time::Duration};
    use tokio::process::Command;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/contracts/syft-grype")
            .join(name)
    }

    fn load_text(name: &str) -> String {
        fs::read_to_string(fixture_path(name)).expect("fixture must be readable")
    }

    fn fixture_identity() -> Box<str> {
        artifact_identity_from_syft_json(&load_text("syft-alpine-3.21.json"))
            .expect("fixture syft json must expose a canonical identity")
    }

    #[tokio::test]
    async fn fixture_provider_satisfies_the_canonical_contract() {
        let provider = FixtureSyftGrypeProvider::from_paths(
            fixture_path("syft-alpine-3.21.json"),
            fixture_path("grype-alpine-3.21.json"),
            fixture_path("syft-alpine-3.21.json"),
            fixture_path("grype-alpine-3.21.json"),
        )
        .expect("fixture provider must load");

        for freshness in [EvidenceFreshness::Deterministic, EvidenceFreshness::Live] {
            let request = crate::ScanRequest::new(
                "component:payments-api",
                ArtifactRef::new(ArtifactKind::ContainerImage, fixture_identity()),
                freshness,
            );

            let report = provider
                .scan(&request)
                .await
                .expect("fixture provider must return a report");

            validate_provider_scan_report(provider.provider_key(), &request, &report)
                .expect("real fixture report must satisfy the contract");
            assert!(!report.findings.is_empty());
            assert!(
                report
                    .knowledge_revision
                    .as_deref()
                    .is_some_and(|value| value.starts_with("grype-db:"))
            );
        }
    }

    #[test]
    fn official_images_are_pinned() {
        assert_eq!(OFFICIAL_SYFT_IMAGE, "ghcr.io/anchore/syft:v1.44.0");
        assert_eq!(OFFICIAL_GRYPE_IMAGE, "ghcr.io/anchore/grype:v0.112.0");
        assert_eq!(DEFAULT_LIVE_COMMAND_TIMEOUT, Duration::from_mins(1));
    }

    #[tokio::test]
    async fn docker_provider_rejects_deterministic_requests() {
        let provider = DockerSyftGrypeProvider::official();
        let request = crate::ScanRequest::new(
            "component:payments-api",
            ArtifactRef::new(ArtifactKind::ContainerImage, fixture_identity()),
            EvidenceFreshness::Deterministic,
        );

        let error = provider
            .scan(&request)
            .await
            .expect_err("deterministic live scan must be rejected");

        assert_eq!(error.kind, FindingProviderErrorKind::InvalidRequest);
    }

    #[test]
    fn provider_error_text_is_bounded() {
        let bytes = vec![b'x'; MAX_ERROR_TEXT_BYTES + 128];
        let text = bounded_error_text(&bytes);

        assert!(text.starts_with('x'));
        assert!(text.ends_with("…[truncated]"));
        assert!(text.len() <= MAX_ERROR_TEXT_BYTES + "…[truncated]".len());
    }

    #[tokio::test]
    async fn run_command_times_out_explicitly() {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("sleep 1")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let error = run_command(command, None, "timeout-test", Duration::from_millis(10))
            .await
            .expect_err("slow command must time out");

        assert_eq!(error.kind, FindingProviderErrorKind::Unavailable);
        assert!(error.retryable);
        assert!(
            error
                .message
                .contains("exceeded the live execution timeout")
        );
    }
}
