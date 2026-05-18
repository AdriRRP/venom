use core::future::Future;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Kind of immutable artifact a provider can observe.
///
/// VENOM treats the artifact as the stable scan subject. Mutable tags or other
/// moving references should be resolved before this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// A container image identified by an immutable digest.
    ContainerImage,
    /// A software bill of materials identified by an immutable digest.
    SbomDocument,
}

/// Canonical identity of the exact thing that was scanned.
///
/// This is intentionally small: kind plus immutable identity. The core should
/// not need provider-specific subject metadata to reason about finding
/// discovery, repetition, or withdrawal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// The class of artifact that was scanned.
    pub kind: ArtifactKind,
    /// Immutable identifier such as `image@sha256:...` or `sbom:sha256:...`.
    pub identity: Box<str>,
}

impl ArtifactRef {
    #[must_use]
    pub fn new(kind: ArtifactKind, identity: impl Into<Box<str>>) -> Self {
        Self {
            kind,
            identity: identity.into(),
        }
    }
}

/// Freshness mode requested from a provider scan.
///
/// This does not describe the age of a finding. It describes what sort of
/// evidence the adapter should use when producing a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceFreshness {
    /// Reproducible evidence such as frozen fixtures, fixed scanner versions,
    /// or a pinned vulnerability database revision.
    Deterministic,
    /// Fresh evidence from the current scanner and vulnerability knowledge.
    Live,
}

/// Request for one complete provider observation over one immutable artifact.
///
/// A scan request asks for a full snapshot, not for incremental delta events.
/// VENOM derives lifecycle semantics by comparing successive snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanRequest {
    /// Canonical component identity inside VENOM.
    pub component_key: Box<str>,
    /// Exact immutable artifact to scan.
    pub artifact: ArtifactRef,
    /// Desired reproducibility vs freshness mode for the adapter.
    pub freshness: EvidenceFreshness,
}

impl ScanRequest {
    #[must_use]
    pub fn new(
        component_key: impl Into<Box<str>>,
        artifact: ArtifactRef,
        freshness: EvidenceFreshness,
    ) -> Self {
        Self {
            component_key: component_key.into(),
            artifact,
            freshness,
        }
    }
}

/// Canonical package identity inside a reported finding.
///
/// Providers disagree on package metadata shape, but package name, version, and
/// optional PURL are the minimum useful common denominator for correlation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PackageCoordinate {
    /// Package name reported by the provider after canonicalization.
    pub name: Box<str>,
    /// Package version reported by the provider after canonicalization.
    pub version: Box<str>,
    /// Optional Package URL when the provider can supply one.
    pub purl: Option<Box<str>>,
}

impl PackageCoordinate {
    #[must_use]
    pub fn new(name: impl Into<Box<str>>, version: impl Into<Box<str>>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            purl: None,
        }
    }

    #[must_use]
    pub fn with_purl(mut self, purl: impl Into<Box<str>>) -> Self {
        self.purl = Some(purl.into());
        self
    }
}

/// Canonical severity bucket accepted by the VENOM provider boundary.
///
/// This keeps the core independent from provider-specific severity labels while
/// still preserving a useful normalized signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// The provider did not give a usable severity.
    Unknown,
    /// The provider explicitly reported no severity or no impact.
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// One canonical vulnerability observation inside a provider scan report.
///
/// This is still provider-facing data, but already normalized enough for VENOM
/// to compare snapshots and build domain behavior on top of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedFinding {
    /// Canonical vulnerability identifier such as a CVE or GHSA id.
    pub vulnerability_id: Box<str>,
    /// Provider-local finding identifier when the provider exposes one.
    pub provider_finding_key: Option<Box<str>>,
    /// Affected package coordinate.
    pub package: PackageCoordinate,
    /// Provider-reported fixed version when available.
    pub fix_version: Option<Box<str>>,
    /// Normalized severity.
    pub severity: Severity,
    /// Additional advisory identifiers that refer to the same vulnerability.
    pub aliases: Vec<Box<str>>,
}

impl ReportedFinding {
    #[must_use]
    pub fn new(vulnerability_id: impl Into<Box<str>>, package: PackageCoordinate) -> Self {
        Self {
            vulnerability_id: vulnerability_id.into(),
            provider_finding_key: None,
            package,
            fix_version: None,
            severity: Severity::Unknown,
            aliases: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_provider_finding_key(mut self, key: impl Into<Box<str>>) -> Self {
        self.provider_finding_key = Some(key.into());
        self
    }

    #[must_use]
    pub fn with_fix_version(mut self, version: impl Into<Box<str>>) -> Self {
        self.fix_version = Some(version.into());
        self
    }

    #[must_use]
    pub const fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    #[must_use]
    pub fn with_alias(mut self, alias: impl Into<Box<str>>) -> Self {
        self.aliases.push(alias.into());
        self
    }
}

/// One complete provider snapshot for one component and one immutable artifact.
///
/// This is the canonical boundary object between provider adapters and the core
/// domain. The report must be self-contained enough for VENOM to compare it
/// with earlier reports and derive what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderScanReport {
    /// Stable provider key such as `syft-grype` or `trivy`.
    pub provider_key: Box<str>,
    /// Canonical component identity inside VENOM.
    pub component_key: Box<str>,
    /// Exact immutable artifact the provider observed.
    pub artifact: ArtifactRef,
    /// Time at which the provider observed or produced this snapshot.
    pub observed_at: SystemTime,
    /// Reproducibility vs freshness mode used to obtain this snapshot.
    pub freshness: EvidenceFreshness,
    /// Optional revision of the provider knowledge base used for the scan.
    pub knowledge_revision: Option<Box<str>>,
    /// Full set of findings observed for this component and artifact.
    pub findings: Vec<ReportedFinding>,
}

impl ProviderScanReport {
    #[must_use]
    pub fn new(
        provider_key: impl Into<Box<str>>,
        component_key: impl Into<Box<str>>,
        artifact: ArtifactRef,
        observed_at: SystemTime,
        freshness: EvidenceFreshness,
        findings: Vec<ReportedFinding>,
    ) -> Self {
        Self {
            provider_key: provider_key.into(),
            component_key: component_key.into(),
            artifact,
            observed_at,
            freshness,
            knowledge_revision: None,
            findings,
        }
    }

    #[must_use]
    pub fn with_knowledge_revision(mut self, revision: impl Into<Box<str>>) -> Self {
        self.knowledge_revision = Some(revision.into());
        self
    }
}

/// Normalized failure class returned by a finding provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingProviderErrorKind {
    /// The request shape is invalid for this provider.
    InvalidRequest,
    /// The provider or required infrastructure is temporarily unavailable.
    Unavailable,
    /// Credentials are missing, invalid, or insufficient.
    Unauthorized,
    /// The provider responded, but the payload could not be trusted or parsed.
    CorruptResponse,
    /// The provider rejected the request because rate limits were exceeded.
    RateLimited,
}

/// Canonical error returned by the provider boundary.
///
/// The `retryable` flag is an adapter decision to help the caller distinguish
/// transient failures from hard failures without depending on provider-specific
/// error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingProviderError {
    /// Broad error class.
    pub kind: FindingProviderErrorKind,
    /// Whether retrying the same request later may succeed.
    pub retryable: bool,
    /// Human-readable explanation suitable for logs and diagnostics.
    pub message: Box<str>,
}

impl FindingProviderError {
    #[must_use]
    pub fn new(
        kind: FindingProviderErrorKind,
        retryable: bool,
        message: impl Into<Box<str>>,
    ) -> Self {
        Self {
            kind,
            retryable,
            message: message.into(),
        }
    }
}

/// Provider boundary for one complete observation over one immutable artifact.
///
/// The provider returns a full snapshot for the requested artifact. VENOM owns
/// the meaning of discovered, repeated, changed, and withdrawn findings by
/// comparing canonical snapshots over time.
pub trait FindingProvider {
    /// Stable identifier of the adapter implementation.
    fn provider_key(&self) -> &'static str;

    /// Produce one canonical snapshot for the requested component and artifact.
    ///
    /// Implementations may call external scanners, parse fixtures, or read
    /// cached SBOMs, but the return value must always be a full snapshot for
    /// the exact requested artifact.
    fn scan<'a>(
        &'a self,
        request: &'a ScanRequest,
    ) -> impl Future<Output = Result<ProviderScanReport, FindingProviderError>> + Send + 'a;
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactKind, ArtifactRef, EvidenceFreshness, PackageCoordinate, ProviderScanReport,
        ReportedFinding, Severity,
    };
    use std::time::SystemTime;

    #[test]
    fn scan_request_artifacts_can_use_immutable_image_identities() {
        let artifact = ArtifactRef::new(
            ArtifactKind::ContainerImage,
            "registry.example/app@sha256:123",
        );

        assert_eq!(artifact.kind, ArtifactKind::ContainerImage);
        assert_eq!(&*artifact.identity, "registry.example/app@sha256:123");
    }

    #[test]
    fn provider_scan_report_keeps_freshness_and_knowledge_revision() {
        let finding = ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0").with_purl("pkg:apk/openssl@3.0.0"),
        )
        .with_fix_version("3.0.1")
        .with_severity(Severity::High)
        .with_alias("GHSA-1234");

        let report = ProviderScanReport::new(
            "syft-grype",
            "component:payments-api",
            ArtifactRef::new(ArtifactKind::SbomDocument, "sbom:sha256:abc"),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![finding],
        )
        .with_knowledge_revision("grype-db:2026-05-14");

        assert_eq!(report.provider_key.as_ref(), "syft-grype");
        assert_eq!(report.freshness, EvidenceFreshness::Deterministic);
        assert_eq!(
            report.knowledge_revision.as_deref(),
            Some("grype-db:2026-05-14")
        );
        assert_eq!(report.findings.len(), 1);
    }
}
