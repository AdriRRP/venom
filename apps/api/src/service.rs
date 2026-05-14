use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use venom_domain::{
    ArtifactKind, ArtifactRef, ComponentRegistration, DurableState, EvidenceFreshness,
    PackageCoordinate, ProviderScanReport, ReportedFinding, Severity,
};

#[derive(Debug)]
pub enum AppServiceError {
    InvalidRequest(String),
    State(String),
}

impl core::fmt::Display for AppServiceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::State(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AppServiceError {}

pub struct AppService {
    state: DurableState,
}

impl AppService {
    /// Open the application service over one local durable state path.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the durable state cannot be opened.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, AppServiceError> {
        let state =
            DurableState::open(path).map_err(|error| AppServiceError::State(error.to_string()))?;
        Ok(Self { state })
    }

    /// Register one managed component through the application boundary.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the durable state write fails.
    pub fn register_component(
        &mut self,
        request: ComponentRegistrationRequest,
    ) -> Result<RegisterComponentResponse, AppServiceError> {
        let result = self
            .state
            .register_component(ComponentRegistration::new(
                request.component_key,
                request.name,
            ))
            .map_err(|error| AppServiceError::State(error.to_string()))?;

        Ok(RegisterComponentResponse {
            change: result.change.as_str().to_owned(),
            managed_components: result.managed_components,
        })
    }

    /// Bind one immutable artifact to one managed component.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the request is invalid or the durable state write fails.
    pub fn bind_artifact(
        &mut self,
        component_key: &str,
        request: BindArtifactRequest,
    ) -> Result<BindArtifactResponse, AppServiceError> {
        let artifact = ArtifactRef::new(
            parse_artifact_kind(&request.artifact_kind)?,
            request.artifact_identity,
        );
        let result = self
            .state
            .bind_artifact(component_key, artifact)
            .map_err(|error| AppServiceError::State(error.to_string()))?;

        Ok(BindArtifactResponse {
            change: result.change.as_str().to_owned(),
            bound_artifacts: result.bound_artifacts,
        })
    }

    /// Record one canonical provider report through the application boundary.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the request is invalid or the durable state write fails.
    pub fn record_provider_report(
        &mut self,
        request: ProviderScanReportRequest,
    ) -> Result<RecordProviderReportResponse, AppServiceError> {
        let mut report = ProviderScanReport::new(
            request.provider_key,
            request.component_key,
            ArtifactRef::new(
                parse_artifact_kind(&request.artifact_kind)?,
                request.artifact_identity,
            ),
            SystemTime::now(),
            parse_freshness(&request.freshness)?,
            request
                .findings
                .into_iter()
                .map(ProviderReportFindingRequest::into_domain)
                .collect::<Result<Vec<_>, _>>()?,
        );
        report.knowledge_revision = request.knowledge_revision.map(String::into_boxed_str);

        let result = self
            .state
            .record_scan_report(&report)
            .map_err(|error| AppServiceError::State(error.to_string()))?;

        Ok(RecordProviderReportResponse {
            discovered: result.discovered,
            repeated: result.repeated,
            withdrawn: result.withdrawn,
            active: result.active,
        })
    }

    /// Query the currently active findings for one managed component and artifact.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the request contains an unsupported artifact kind.
    pub fn list_active_findings(
        &self,
        request: ActiveFindingsRequest,
    ) -> Result<ActiveFindingsResponse, AppServiceError> {
        let artifact = ArtifactRef::new(
            parse_artifact_kind(&request.artifact_kind)?,
            request.artifact_identity.clone(),
        );
        let findings = self
            .state
            .read_model()
            .active_findings(&request.component_key, &artifact)
            .into_iter()
            .map(|finding| ActiveFindingItem {
                vulnerability_id: finding.vulnerability_id.into(),
                package_name: finding.package.name.into(),
                package_version: finding.package.version.into(),
                severity: severity_name(finding.severity).to_owned(),
            })
            .collect::<Vec<_>>();

        Ok(ActiveFindingsResponse {
            component_key: request.component_key,
            artifact_kind: request.artifact_kind,
            artifact_identity: request.artifact_identity,
            active_findings: findings,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ComponentRegistrationRequest {
    pub component_key: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterComponentResponse {
    pub change: String,
    pub managed_components: usize,
}

#[derive(Debug, Deserialize)]
pub struct BindArtifactRequest {
    pub artifact_kind: String,
    pub artifact_identity: String,
}

#[derive(Debug, Serialize)]
pub struct BindArtifactResponse {
    pub change: String,
    pub bound_artifacts: usize,
}

#[derive(Debug, Deserialize)]
pub struct ProviderScanReportRequest {
    pub provider_key: String,
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub freshness: String,
    pub knowledge_revision: Option<String>,
    pub findings: Vec<ProviderReportFindingRequest>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderReportFindingRequest {
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub severity: String,
}

impl ProviderReportFindingRequest {
    fn into_domain(self) -> Result<ReportedFinding, AppServiceError> {
        Ok(ReportedFinding::new(
            self.vulnerability_id,
            PackageCoordinate::new(self.package_name, self.package_version),
        )
        .with_severity(parse_severity(&self.severity)?))
    }
}

#[derive(Debug, Serialize)]
pub struct RecordProviderReportResponse {
    pub discovered: usize,
    pub repeated: usize,
    pub withdrawn: usize,
    pub active: usize,
}

#[derive(Debug)]
pub struct ActiveFindingsRequest {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
}

#[derive(Debug, Serialize)]
pub struct ActiveFindingsResponse {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub active_findings: Vec<ActiveFindingItem>,
}

#[derive(Debug, Serialize)]
pub struct ActiveFindingItem {
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub severity: String,
}

fn parse_artifact_kind(value: &str) -> Result<ArtifactKind, AppServiceError> {
    match value {
        "container-image" => Ok(ArtifactKind::ContainerImage),
        "sbom-document" => Ok(ArtifactKind::SbomDocument),
        _ => Err(AppServiceError::InvalidRequest(format!(
            "unsupported artifact kind: {value}"
        ))),
    }
}

fn parse_freshness(value: &str) -> Result<EvidenceFreshness, AppServiceError> {
    match value {
        "deterministic" => Ok(EvidenceFreshness::Deterministic),
        "live" => Ok(EvidenceFreshness::Live),
        _ => Err(AppServiceError::InvalidRequest(format!(
            "unsupported freshness: {value}"
        ))),
    }
}

fn parse_severity(value: &str) -> Result<Severity, AppServiceError> {
    match value {
        "unknown" => Ok(Severity::Unknown),
        "none" => Ok(Severity::None),
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        _ => Err(AppServiceError::InvalidRequest(format!(
            "unsupported severity: {value}"
        ))),
    }
}

const fn severity_name(value: Severity) -> &'static str {
    match value {
        Severity::Unknown => "unknown",
        Severity::None => "none",
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
        Severity::Critical => "critical",
    }
}
