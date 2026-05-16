use crate::postgres_backend::PostgresBackend;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use venom_domain::{
    ActiveFindingsQuery, ArtifactKind, ArtifactRef, ComponentRegistration, DurableScanRuntime,
    DurableState, EvidenceFreshness, FindingProvider, FindingProviderError,
    FindingProviderErrorKind, IntegrationEventPublishError, IntegrationEventPublisher,
    PackageCoordinate, PendingIntegrationEvent, ProviderScanReport, PublishIntegrationEventsResult,
    ReportedFinding, RunNextScanResult, ScanCommandStatus, ScanPlanner, ScanRequest, Severity,
};

#[derive(Debug)]
pub enum AppServiceError {
    InvalidRequest(String),
    NotFound(String),
    State(String),
}

impl core::fmt::Display for AppServiceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::NotFound(message) | Self::State(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for AppServiceError {}

pub struct AppService {
    backend: AppBackend,
}

enum AppBackend {
    Local(LocalBackend),
    Postgres(PostgresBackend),
}

struct LocalBackend {
    state: DurableState,
    runtime: DurableScanRuntime,
}

impl AppService {
    /// Open the application service over one local durable state path.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the durable state or durable runtime cannot be opened.
    pub fn open_local(
        state_path: impl Into<PathBuf>,
        runtime_path: impl Into<PathBuf>,
    ) -> Result<Self, AppServiceError> {
        let state = DurableState::open(state_path)
            .map_err(|error| AppServiceError::State(error.to_string()))?;
        let runtime = DurableScanRuntime::open(runtime_path)
            .map_err(|error| AppServiceError::State(error.to_string()))?;
        Ok(Self {
            backend: AppBackend::Local(LocalBackend { state, runtime }),
        })
    }

    /// Open the application service over a Postgres durable backend.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the Postgres durable backend cannot be opened.
    pub async fn open_postgres(database_url: &str, schema: &str) -> Result<Self, AppServiceError> {
        let backend = PostgresBackend::open(database_url, schema)
            .await
            .map_err(AppServiceError::State)?;
        Ok(Self {
            backend: AppBackend::Postgres(backend),
        })
    }

    /// Register one managed component through the application boundary.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the durable state write fails.
    pub async fn register_component(
        &mut self,
        request: ComponentRegistrationRequest,
    ) -> Result<RegisterComponentResponse, AppServiceError> {
        let registration = ComponentRegistration::new(request.component_key, request.name);
        let result = match &mut self.backend {
            AppBackend::Local(local) => local
                .state
                .register_component(registration)
                .map_err(|error| AppServiceError::State(error.to_string()))?,
            AppBackend::Postgres(postgres) => postgres
                .register_component(registration)
                .await
                .map_err(AppServiceError::State)?,
        };

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
    pub async fn bind_artifact(
        &mut self,
        component_key: &str,
        request: BindArtifactRequest,
    ) -> Result<BindArtifactResponse, AppServiceError> {
        let artifact = ArtifactRef::new(
            parse_artifact_kind(&request.artifact_kind)?,
            request.artifact_identity,
        );
        let result = match &mut self.backend {
            AppBackend::Local(local) => local
                .state
                .bind_artifact(component_key, artifact)
                .map_err(|error| AppServiceError::State(error.to_string()))?,
            AppBackend::Postgres(postgres) => postgres
                .bind_artifact(component_key, artifact)
                .await
                .map_err(AppServiceError::State)?,
        };

        Ok(BindArtifactResponse {
            change: result.change.as_str().to_owned(),
            bound_artifacts: result.bound_artifacts,
        })
    }

    /// Configure the runtime provider that one managed component will use for scan execution.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the provider key is unsupported or the durable write fails.
    pub async fn configure_provider(
        &mut self,
        component_key: &str,
        request: ConfigureProviderRequest,
    ) -> Result<ConfigureProviderResponse, AppServiceError> {
        let provider_key = resolve_supported_provider_key(&request.provider_key)?;
        let result = match &mut self.backend {
            AppBackend::Local(local) => local
                .state
                .configure_provider(component_key, provider_key)
                .map_err(|error| AppServiceError::State(error.to_string()))?,
            AppBackend::Postgres(postgres) => postgres
                .configure_provider(component_key, provider_key)
                .await
                .map_err(AppServiceError::State)?,
        };

        Ok(ConfigureProviderResponse {
            change: result.change.as_str().to_owned(),
            provider_key: result.provider_key.map(Into::into),
        })
    }

    /// Record one canonical provider report through the application boundary.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the request is invalid or the durable state write fails.
    pub async fn record_provider_report(
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

        let result = match &mut self.backend {
            AppBackend::Local(local) => local
                .state
                .record_scan_report(&report)
                .map_err(|error| AppServiceError::State(error.to_string()))?,
            AppBackend::Postgres(postgres) => postgres
                .record_scan_report(&report)
                .await
                .map_err(AppServiceError::State)?,
        };

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
        let query = build_active_findings_query(&request, artifact)?;
        let page = match &self.backend {
            AppBackend::Local(local) => local.state.read_model().query_active_findings(&query),
            AppBackend::Postgres(postgres) => postgres.query_active_findings(&query),
        };
        let findings = page
            .findings
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
            min_severity: request.min_severity,
            package_name: request.package_name,
            total_active_findings: page.total,
            returned: page.returned,
            offset: page.offset,
            limit: page.limit,
            active_findings: findings,
        })
    }

    /// Create and durably enqueue one canonical scan request for managed ownership.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the request is invalid, ownership is unmanaged,
    /// or the durable runtime cannot append the command.
    pub async fn request_scan(
        &mut self,
        request: RequestScanCommand,
    ) -> Result<RequestScanResponse, AppServiceError> {
        let artifact = ArtifactRef::new(
            parse_artifact_kind(&request.artifact_kind)?,
            request.artifact_identity.clone(),
        );
        let freshness = parse_freshness(&request.freshness)?;
        let command_id = match &mut self.backend {
            AppBackend::Local(local) => {
                let scan_request = ScanPlanner::new(local.state.ingestion().inventory())
                    .plan(&request.component_key, artifact, freshness)
                    .map_err(|error| AppServiceError::InvalidRequest(error.as_str().to_owned()))?;
                local
                    .runtime
                    .enqueue(scan_request)
                    .map_err(|error| AppServiceError::State(error.to_string()))?
                    .command_id
            }
            AppBackend::Postgres(postgres) => postgres
                .request_scan(&request.component_key, artifact, freshness)
                .await
                .map_err(AppServiceError::State)?,
        };

        Ok(RequestScanResponse {
            command_id: command_id.into(),
            status: ScanCommandStatus::Pending.as_str().to_owned(),
            component_key: request.component_key,
            artifact_kind: request.artifact_kind,
            artifact_identity: request.artifact_identity,
            freshness: request.freshness,
        })
    }

    /// Query the durable status of one scan command.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError::NotFound`] when the command is unknown.
    pub fn scan_command_status(
        &self,
        command_id: &str,
    ) -> Result<ScanCommandStatusResponse, AppServiceError> {
        let status = match &self.backend {
            AppBackend::Local(local) => local.runtime.command_status(command_id),
            AppBackend::Postgres(postgres) => postgres.command_status(command_id),
        }
        .ok_or_else(|| AppServiceError::NotFound(format!("unknown scan command: {command_id}")))?;

        Ok(ScanCommandStatusResponse {
            command_id: command_id.to_owned(),
            status: status.as_str().to_owned(),
        })
    }

    /// Drain pending scan commands through one bounded worker loop.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the provider input or the worker limit is invalid,
    /// or when the durable runtime/state fails.
    pub async fn run_worker_until_idle(
        &mut self,
        request: DrainWorkerCommand,
    ) -> Result<DrainWorkerResponse, AppServiceError> {
        let max_commands = request.max_commands.ok_or_else(|| {
            AppServiceError::InvalidRequest("max_commands is required".to_owned())
        })?;
        if max_commands == 0 {
            return Err(AppServiceError::InvalidRequest(
                "max_commands must be greater than zero".to_owned(),
            ));
        }

        let mut processed = 0_usize;
        let mut completed = 0_usize;
        let mut failed = 0_usize;
        let mut last_command_id = None;
        let mut last_command_status = None;
        let mut last_error_code = None;
        let mut last_retryable = None;

        while processed < max_commands {
            let Some(provider_key) = self.next_pending_provider_key()? else {
                break;
            };
            let provider = ApiExecutionProvider::new(provider_key, request.provider.clone())?;
            let outcome = match &mut self.backend {
                AppBackend::Local(local) => local
                    .runtime
                    .run_next(&mut local.state, &provider)
                    .await
                    .map_err(|error| AppServiceError::State(error.to_string()))?,
                AppBackend::Postgres(postgres) => postgres
                    .run_next(&provider)
                    .await
                    .map_err(AppServiceError::State)?,
            };

            match outcome {
                RunNextScanResult::Idle => break,
                RunNextScanResult::Completed(result) => {
                    processed += 1;
                    completed += 1;
                    last_command_id = Some(result.command_id.into());
                    last_command_status = Some(ScanCommandStatus::Completed.as_str().to_owned());
                    last_error_code = None;
                    last_retryable = None;
                }
                RunNextScanResult::Failed(result) => {
                    processed += 1;
                    failed += 1;
                    last_command_id = Some(result.command_id.into());
                    last_command_status = Some(ScanCommandStatus::Failed.as_str().to_owned());
                    last_error_code = Some(result.error_code.into());
                    last_retryable = Some(result.retryable);
                }
            }
        }

        let pending_remaining = match &self.backend {
            AppBackend::Local(local) => local.runtime.pending_commands(),
            AppBackend::Postgres(postgres) => postgres.pending_commands(),
        };

        let outcome = if processed == 0 {
            "idle"
        } else if pending_remaining == 0 {
            "drained"
        } else {
            "limited"
        };

        Ok(DrainWorkerResponse {
            outcome: outcome.to_owned(),
            processed,
            completed,
            failed,
            pending_remaining,
            last_command_id,
            last_command_status,
            last_error_code,
            last_retryable,
        })
    }

    /// Publish pending integration events through one bounded worker loop.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the request is invalid or publication outcome
    /// persistence fails.
    pub async fn publish_integration_events_until_idle(
        &mut self,
        request: DrainIntegrationWorkerCommand,
    ) -> Result<DrainIntegrationWorkerResponse, AppServiceError> {
        let max_events = request
            .max_events
            .ok_or_else(|| AppServiceError::InvalidRequest("max_events is required".to_owned()))?;
        if max_events == 0 {
            return Err(AppServiceError::InvalidRequest(
                "max_events must be greater than zero".to_owned(),
            ));
        }

        let publisher = ApiIntegrationPublisher::new(request)?;
        let attempted_events = self
            .pending_integration_events_snapshot()
            .into_iter()
            .take(max_events)
            .collect::<Vec<_>>();

        let result = match &mut self.backend {
            AppBackend::Local(local) => {
                publish_pending_local_integration_events(local, max_events, &publisher)
                    .await
                    .map_err(AppServiceError::State)?
            }
            AppBackend::Postgres(postgres) => postgres
                .publish_pending_integration_events(max_events, &publisher)
                .await
                .map_err(AppServiceError::State)?,
        };

        let last_attempted = attempted_events.get(result.attempted.saturating_sub(1));
        let outcome = if result.attempted == 0 {
            "idle"
        } else if result.pending_remaining == 0 {
            "drained"
        } else {
            "limited"
        };

        Ok(DrainIntegrationWorkerResponse {
            outcome: outcome.to_owned(),
            attempted: result.attempted,
            published: result.published,
            pending_remaining: result.pending_remaining,
            last_event_id: last_attempted.map(|event| event.event_id.to_string()),
            last_event_kind: last_attempted.map(|event| event.event.kind_name().to_owned()),
            last_error: result
                .last_failure
                .as_ref()
                .map(|failure| failure.message.to_string()),
            last_retryable: result
                .last_failure
                .as_ref()
                .map(|failure| failure.retryable),
        })
    }

    /// Execute the next queued scan through an injected canonical provider input.
    ///
    /// # Errors
    ///
    /// Returns [`AppServiceError`] when the provider input is invalid or the durable runtime/state fails.
    pub async fn run_next_scan(
        &mut self,
        request: RunNextScanCommand,
    ) -> Result<RunNextScanResponse, AppServiceError> {
        let Some(provider_key) = self.next_pending_provider_key()? else {
            return Ok(RunNextScanResponse {
                outcome: "idle".to_owned(),
                command_id: None,
                provider_key: None,
                findings_reported: None,
                discovered: None,
                repeated: None,
                withdrawn: None,
                active: None,
                error_code: None,
                retryable: None,
            });
        };
        let provider = ApiExecutionProvider::new(provider_key, request)?;
        let outcome =
            match &mut self.backend {
                AppBackend::Local(local) => local
                    .runtime
                    .run_next(&mut local.state, &provider)
                    .await
                    .map_err(|error| AppServiceError::State(error.to_string()))?,
                AppBackend::Postgres(postgres) => postgres
                    .run_next(&provider)
                    .await
                    .map_err(AppServiceError::State)?,
            };

        Ok(match outcome {
            RunNextScanResult::Idle => RunNextScanResponse {
                outcome: "idle".to_owned(),
                command_id: None,
                provider_key: None,
                findings_reported: None,
                discovered: None,
                repeated: None,
                withdrawn: None,
                active: None,
                error_code: None,
                retryable: None,
            },
            RunNextScanResult::Completed(result) => RunNextScanResponse {
                outcome: "completed".to_owned(),
                command_id: Some(result.command_id.into()),
                provider_key: Some(result.provider_key.into()),
                findings_reported: Some(result.findings_reported),
                discovered: Some(result.change_set.discovered),
                repeated: Some(result.change_set.repeated),
                withdrawn: Some(result.change_set.withdrawn),
                active: Some(result.change_set.active),
                error_code: None,
                retryable: None,
            },
            RunNextScanResult::Failed(result) => RunNextScanResponse {
                outcome: "failed".to_owned(),
                command_id: Some(result.command_id.into()),
                provider_key: None,
                findings_reported: None,
                discovered: None,
                repeated: None,
                withdrawn: None,
                active: None,
                error_code: Some(result.error_code.into()),
                retryable: Some(result.retryable),
            },
        })
    }

    fn next_pending_provider_key(&self) -> Result<Option<&'static str>, AppServiceError> {
        let Some(component_key) = self.next_pending_component_key() else {
            return Ok(None);
        };
        let Some(provider_key) = self.configured_provider(component_key) else {
            return Err(AppServiceError::State(format!(
                "missing provider runtime configuration for component: {component_key}"
            )));
        };
        resolve_supported_provider_key(provider_key).map(Some)
    }

    fn next_pending_component_key(&self) -> Option<&str> {
        match &self.backend {
            AppBackend::Local(local) => local.runtime.next_pending_component_key(),
            AppBackend::Postgres(postgres) => postgres.next_pending_component_key(),
        }
    }

    fn configured_provider(&self, component_key: &str) -> Option<&str> {
        match &self.backend {
            AppBackend::Local(local) => local
                .state
                .ingestion()
                .inventory()
                .configured_provider(component_key),
            AppBackend::Postgres(postgres) => postgres.configured_provider(component_key),
        }
    }

    fn pending_integration_events_snapshot(&self) -> Vec<PendingIntegrationEvent> {
        match &self.backend {
            AppBackend::Local(local) => local
                .state
                .pending_integration_events()
                .iter()
                .chain(local.runtime.pending_integration_events().iter())
                .cloned()
                .collect(),
            AppBackend::Postgres(postgres) => postgres.pending_integration_events().to_vec(),
        }
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
pub struct ConfigureProviderRequest {
    pub provider_key: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigureProviderResponse {
    pub change: String,
    pub provider_key: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
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
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ActiveFindingsResponse {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub total_active_findings: usize,
    pub returned: usize,
    pub offset: usize,
    pub limit: usize,
    pub active_findings: Vec<ActiveFindingItem>,
}

#[derive(Debug, Serialize)]
pub struct ActiveFindingItem {
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub severity: String,
}

#[derive(Debug, Deserialize)]
pub struct RequestScanCommand {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub freshness: String,
}

#[derive(Debug, Serialize)]
pub struct RequestScanResponse {
    pub command_id: String,
    pub status: String,
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub freshness: String,
}

#[derive(Debug, Serialize)]
pub struct ScanCommandStatusResponse {
    pub command_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunNextScanCommand {
    pub knowledge_revision: Option<String>,
    pub findings: Option<Vec<ProviderReportFindingRequest>>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub retryable: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RunNextScanResponse {
    pub outcome: String,
    pub command_id: Option<String>,
    pub provider_key: Option<String>,
    pub findings_reported: Option<usize>,
    pub discovered: Option<usize>,
    pub repeated: Option<usize>,
    pub withdrawn: Option<usize>,
    pub active: Option<usize>,
    pub error_code: Option<String>,
    pub retryable: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DrainWorkerCommand {
    pub max_commands: Option<usize>,
    #[serde(flatten)]
    pub provider: RunNextScanCommand,
}

#[derive(Debug, Serialize)]
pub struct DrainWorkerResponse {
    pub outcome: String,
    pub processed: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending_remaining: usize,
    pub last_command_id: Option<String>,
    pub last_command_status: Option<String>,
    pub last_error_code: Option<String>,
    pub last_retryable: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DrainIntegrationWorkerCommand {
    pub max_events: Option<usize>,
    pub publisher_key: Option<String>,
    pub error_message: Option<String>,
    pub retryable: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DrainIntegrationWorkerResponse {
    pub outcome: String,
    pub attempted: usize,
    pub published: usize,
    pub pending_remaining: usize,
    pub last_event_id: Option<String>,
    pub last_event_kind: Option<String>,
    pub last_error: Option<String>,
    pub last_retryable: Option<bool>,
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

fn build_active_findings_query(
    request: &ActiveFindingsRequest,
    artifact: ArtifactRef,
) -> Result<ActiveFindingsQuery, AppServiceError> {
    let mut query = ActiveFindingsQuery::new(request.component_key.clone(), artifact);
    if let Some(min_severity) = request.min_severity.as_deref() {
        query = query.with_min_severity(parse_severity(min_severity)?);
    }
    if let Some(package_name) = request.package_name.as_deref() {
        query = query.with_package_name(package_name);
    }
    if let Some(offset) = request.offset {
        query = query.with_offset(offset);
    }
    if let Some(limit) = request.limit {
        query = query.with_limit(limit);
    }
    Ok(query)
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

#[derive(Debug, Clone)]
struct ApiExecutionProvider {
    provider_key: &'static str,
    mode: ApiExecutionMode,
}

#[derive(Debug, Clone)]
enum ApiExecutionMode {
    Success {
        findings: Vec<ReportedFinding>,
        knowledge_revision: Option<Box<str>>,
    },
    Failure(FindingProviderError),
}

impl ApiExecutionProvider {
    fn new(
        provider_key: &'static str,
        request: RunNextScanCommand,
    ) -> Result<Self, AppServiceError> {
        let RunNextScanCommand {
            knowledge_revision,
            findings,
            error_kind,
            error_message,
            retryable,
        } = request;

        if let Some(error_kind) = error_kind {
            let message = error_message.unwrap_or_else(|| "provider execution failed".to_owned());
            return Ok(Self {
                provider_key,
                mode: ApiExecutionMode::Failure(FindingProviderError::new(
                    parse_error_kind(&error_kind)?,
                    retryable.unwrap_or(false),
                    message,
                )),
            });
        }

        let findings = findings
            .unwrap_or_default()
            .into_iter()
            .map(ProviderReportFindingRequest::into_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            provider_key,
            mode: ApiExecutionMode::Success {
                findings,
                knowledge_revision: knowledge_revision.map(String::into_boxed_str),
            },
        })
    }
}

impl FindingProvider for ApiExecutionProvider {
    fn provider_key(&self) -> &'static str {
        self.provider_key
    }

    async fn scan<'a>(
        &'a self,
        request: &'a ScanRequest,
    ) -> Result<ProviderScanReport, FindingProviderError> {
        match &self.mode {
            ApiExecutionMode::Success {
                findings,
                knowledge_revision,
            } => {
                let mut report = ProviderScanReport::new(
                    self.provider_key,
                    request.component_key.clone(),
                    request.artifact.clone(),
                    SystemTime::now(),
                    request.freshness,
                    findings.clone(),
                );
                report.knowledge_revision.clone_from(knowledge_revision);
                Ok(report)
            }
            ApiExecutionMode::Failure(error) => Err(error.clone()),
        }
    }
}

const API_WORKER_PROVIDER_KEY: &str = "fixture-provider";
const API_INTEGRATION_PUBLISHER_KEY: &str = "fixture-publisher";

fn resolve_supported_provider_key(value: &str) -> Result<&'static str, AppServiceError> {
    match value {
        API_WORKER_PROVIDER_KEY => Ok(API_WORKER_PROVIDER_KEY),
        _ => Err(AppServiceError::InvalidRequest(format!(
            "unsupported provider key: {value}"
        ))),
    }
}

fn parse_error_kind(value: &str) -> Result<FindingProviderErrorKind, AppServiceError> {
    match value {
        "invalid-request" => Ok(FindingProviderErrorKind::InvalidRequest),
        "unavailable" => Ok(FindingProviderErrorKind::Unavailable),
        "unauthorized" => Ok(FindingProviderErrorKind::Unauthorized),
        "corrupt-response" => Ok(FindingProviderErrorKind::CorruptResponse),
        "rate-limited" => Ok(FindingProviderErrorKind::RateLimited),
        _ => Err(AppServiceError::InvalidRequest(format!(
            "unsupported provider error kind: {value}"
        ))),
    }
}

#[derive(Debug, Clone)]
struct ApiIntegrationPublisher {
    mode: ApiIntegrationPublisherMode,
}

#[derive(Debug, Clone)]
enum ApiIntegrationPublisherMode {
    Success,
    Failure(IntegrationEventPublishError),
}

impl ApiIntegrationPublisher {
    fn new(request: DrainIntegrationWorkerCommand) -> Result<Self, AppServiceError> {
        let publisher_key = request
            .publisher_key
            .unwrap_or_else(|| API_INTEGRATION_PUBLISHER_KEY.to_owned());
        resolve_supported_publisher_key(&publisher_key)?;

        Ok(if let Some(message) = request.error_message {
            Self {
                mode: ApiIntegrationPublisherMode::Failure(IntegrationEventPublishError::new(
                    request.retryable.unwrap_or(false),
                    message,
                )),
            }
        } else {
            Self {
                mode: ApiIntegrationPublisherMode::Success,
            }
        })
    }
}

impl IntegrationEventPublisher for ApiIntegrationPublisher {
    fn publisher_key(&self) -> &'static str {
        API_INTEGRATION_PUBLISHER_KEY
    }

    async fn publish<'a>(
        &'a self,
        _event: &'a PendingIntegrationEvent,
    ) -> Result<(), IntegrationEventPublishError> {
        match &self.mode {
            ApiIntegrationPublisherMode::Success => Ok(()),
            ApiIntegrationPublisherMode::Failure(error) => Err(error.clone()),
        }
    }
}

fn resolve_supported_publisher_key(value: &str) -> Result<&'static str, AppServiceError> {
    match value {
        API_INTEGRATION_PUBLISHER_KEY => Ok(API_INTEGRATION_PUBLISHER_KEY),
        _ => Err(AppServiceError::InvalidRequest(format!(
            "unsupported publisher key: {value}"
        ))),
    }
}

async fn publish_pending_local_integration_events(
    local: &mut LocalBackend,
    max_events: usize,
    publisher: &(impl IntegrationEventPublisher + Sync),
) -> Result<PublishIntegrationEventsResult, String> {
    let mut result = PublishIntegrationEventsResult {
        attempted: 0,
        published: 0,
        pending_remaining: local.state.pending_integration_events().len()
            + local.runtime.pending_integration_events().len(),
        last_failure: None,
    };
    if max_events == 0 {
        return Ok(result);
    }

    let state_result = local
        .state
        .publish_pending_integration_events(max_events, publisher)
        .await
        .map_err(|error| error.to_string())?;
    merge_publish_results(&mut result, state_result);

    if result.last_failure.is_some() || result.attempted >= max_events {
        result.pending_remaining = local.state.pending_integration_events().len()
            + local.runtime.pending_integration_events().len();
        return Ok(result);
    }

    let runtime_budget = max_events - result.attempted;
    let runtime_result = local
        .runtime
        .publish_pending_integration_events(runtime_budget, publisher)
        .await
        .map_err(|error| error.to_string())?;
    merge_publish_results(&mut result, runtime_result);
    result.pending_remaining = local.state.pending_integration_events().len()
        + local.runtime.pending_integration_events().len();
    Ok(result)
}

fn merge_publish_results(
    aggregate: &mut PublishIntegrationEventsResult,
    update: PublishIntegrationEventsResult,
) {
    aggregate.attempted += update.attempted;
    aggregate.published += update.published;
    aggregate.pending_remaining = update.pending_remaining;
    if update.last_failure.is_some() {
        aggregate.last_failure = update.last_failure;
    }
}
