use crate::infra::http_integration_publisher::{HTTP_EVENT_PUBLISHER_KEY, HttpEventPublisher};
use crate::infra::postgres_backend::{DrainDueCollectionScansResult, PostgresStore};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use venom_domain::durable_state::DurableState;
use venom_domain::findings::{
    ActiveFindingsQuery, ArtifactKind, ArtifactRef, EvidenceFreshness, FindingProvider,
    FindingProviderError, FindingProviderErrorKind, FindingReadModel, PackageCoordinate,
    ProviderScanReport, ReportedFinding, ScanRequest, Severity,
};
use venom_domain::integration::{
    IntegrationEventPublishError, IntegrationEventPublisher, IntegrationRuntimeConfig,
    PendingIntegrationEvent, PublishIntegrationEventsResult,
};
use venom_domain::inventory::{CollectionRegistration, ComponentInventory, ComponentRegistration};
use venom_domain::scanning::{
    CollectionScanScheduler, RunNextScanResult, ScanCommandQueue, ScanCommandStatus, ScanPlanner,
};

#[derive(Debug)]
pub enum ApiApplicationError {
    InvalidRequest(String),
    NotFound(String),
    State(String),
}

impl core::fmt::Display for ApiApplicationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::NotFound(message) | Self::State(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for ApiApplicationError {}

#[derive(Debug, Clone)]
pub struct ApiReadSnapshot {
    inventory: Arc<ComponentInventory>,
    read_model: Arc<FindingReadModel>,
}

impl ApiReadSnapshot {
    #[must_use]
    pub fn new(inventory: ComponentInventory, read_model: FindingReadModel) -> Self {
        Self {
            inventory: Arc::new(inventory),
            read_model: Arc::new(read_model),
        }
    }

    #[must_use]
    pub fn with_inventory(&self, inventory: ComponentInventory) -> Self {
        Self {
            inventory: Arc::new(inventory),
            read_model: Arc::clone(&self.read_model),
        }
    }

    #[must_use]
    pub fn with_read_model(&self, read_model: FindingReadModel) -> Self {
        Self {
            inventory: Arc::clone(&self.inventory),
            read_model: Arc::new(read_model),
        }
    }

    /// Query the currently active findings for one managed component and artifact.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request contains an unsupported artifact kind.
    pub fn list_active_findings(
        &self,
        request: ActiveFindingsRequest,
    ) -> Result<ActiveFindingsResponse, ApiApplicationError> {
        let artifact = ArtifactRef::new(
            parse_artifact_kind(&request.artifact_kind)?,
            request.artifact_identity.clone(),
        );
        let query = build_active_findings_query(&request, artifact)?;
        let page = self.read_model.query_active_findings(&query);
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

    /// Query the operator-facing collection board with schedule and due state.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the current system time cannot be read.
    pub fn list_collections(&self) -> Result<ListCollectionsResponse, ApiApplicationError> {
        let now_unix_ms = current_unix_millis()?;
        let collections = self
            .inventory
            .collection_operations_summaries(now_unix_ms)
            .into_iter()
            .map(|collection| CollectionSummary {
                collection_key: collection.collection_key.into(),
                name: collection.name.into(),
                members: collection.members,
                scan_schedule: collection
                    .scan_schedule
                    .map(CollectionScanScheduleItem::from),
                due_now: collection.due_now,
            })
            .collect::<Vec<_>>();
        let managed_collections = collections.len();
        Ok(ListCollectionsResponse {
            managed_collections,
            collections,
        })
    }

    /// Query one managed collection detail by key.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError::NotFound`] when the collection is unknown.
    pub fn collection_detail(
        &self,
        collection_key: &str,
    ) -> Result<CollectionDetailResponse, ApiApplicationError> {
        let collection = self
            .inventory
            .collections()
            .into_iter()
            .find(|collection| collection.collection_key.as_ref() == collection_key)
            .ok_or_else(|| {
                ApiApplicationError::NotFound(format!("unknown collection: {collection_key}"))
            })?;

        Ok(CollectionDetailResponse {
            collection_key: collection.collection_key.into(),
            name: collection.name.into(),
            scan_schedule: collection
                .scan_schedule
                .map(CollectionScanScheduleItem::from),
            members: collection
                .component_keys
                .into_iter()
                .map(|component_key| CollectionMemberItem {
                    component_key: component_key.into(),
                })
                .collect(),
        })
    }
}

pub struct ApiApplication {
    backend: ApiStore,
}

enum ApiStore {
    Local(LocalStore),
    Postgres(PostgresStore),
}

struct LocalStore {
    state: DurableState,
    runtime: ScanCommandQueue,
}

impl ApiApplication {
    /// Open the application service over one local durable state path.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state or durable runtime cannot be opened.
    pub fn open_local(
        state_path: impl Into<PathBuf>,
        runtime_path: impl Into<PathBuf>,
    ) -> Result<Self, ApiApplicationError> {
        let state = DurableState::open(state_path)
            .map_err(|error| ApiApplicationError::State(error.to_string()))?;
        let runtime = ScanCommandQueue::open(runtime_path)
            .map_err(|error| ApiApplicationError::State(error.to_string()))?;
        Ok(Self {
            backend: ApiStore::Local(LocalStore { state, runtime }),
        })
    }

    /// Open the application service over a Postgres durable backend.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the Postgres durable backend cannot be opened.
    pub async fn open_postgres(
        database_url: &str,
        schema: &str,
    ) -> Result<Self, ApiApplicationError> {
        let backend = PostgresStore::open(database_url, schema)
            .await
            .map_err(ApiApplicationError::State)?;
        Ok(Self {
            backend: ApiStore::Postgres(backend),
        })
    }

    #[must_use]
    pub fn read_snapshot(&self) -> ApiReadSnapshot {
        match &self.backend {
            ApiStore::Local(local) => ApiReadSnapshot::new(
                local.state.ingestion().inventory().clone(),
                local.state.read_model().clone(),
            ),
            ApiStore::Postgres(postgres) => ApiReadSnapshot::new(
                postgres.inventory_snapshot(),
                postgres.read_model_snapshot(),
            ),
        }
    }

    #[must_use]
    pub fn inventory_snapshot(&self) -> ComponentInventory {
        match &self.backend {
            ApiStore::Local(local) => local.state.ingestion().inventory().clone(),
            ApiStore::Postgres(postgres) => postgres.inventory_snapshot(),
        }
    }

    #[must_use]
    pub fn read_model_snapshot(&self) -> FindingReadModel {
        match &self.backend {
            ApiStore::Local(local) => local.state.read_model().clone(),
            ApiStore::Postgres(postgres) => postgres.read_model_snapshot(),
        }
    }

    /// Query the durable status of one scan command.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError::NotFound`] when the command is unknown.
    pub fn scan_command_status(
        &self,
        command_id: &str,
    ) -> Result<ScanCommandStatusResponse, ApiApplicationError> {
        let status = match &self.backend {
            ApiStore::Local(local) => local.runtime.command_status(command_id),
            ApiStore::Postgres(postgres) => postgres.command_status(command_id),
        }
        .ok_or_else(|| {
            ApiApplicationError::NotFound(format!("unknown scan command: {command_id}"))
        })?;

        Ok(ScanCommandStatusResponse {
            command_id: command_id.to_owned(),
            status: status.as_str().to_owned(),
        })
    }

    /// Register one managed component through the application boundary.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn register_component(
        &mut self,
        request: ComponentRegistrationRequest,
    ) -> Result<RegisterComponentResponse, ApiApplicationError> {
        let registration = ComponentRegistration::new(request.component_key, request.name);
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .register_component(registration)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .register_component(registration)
                .await
                .map_err(ApiApplicationError::State)?,
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
    /// Returns [`ApiApplicationError`] when the request is invalid or the durable state write fails.
    pub async fn bind_artifact(
        &mut self,
        component_key: &str,
        request: BindArtifactRequest,
    ) -> Result<BindArtifactResponse, ApiApplicationError> {
        let artifact = ArtifactRef::new(
            parse_artifact_kind(&request.artifact_kind)?,
            request.artifact_identity,
        );
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .bind_artifact(component_key, artifact)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .bind_artifact(component_key, artifact)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(BindArtifactResponse {
            change: result.change.as_str().to_owned(),
            bound_artifacts: result.bound_artifacts,
        })
    }

    /// Create one closed release collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn register_collection(
        &mut self,
        request: CollectionRegistrationRequest,
    ) -> Result<RegisterCollectionResponse, ApiApplicationError> {
        let registration = CollectionRegistration::new(request.collection_key, request.name);
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .register_collection(registration)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .register_collection(registration)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(RegisterCollectionResponse {
            change: result.change.as_str().to_owned(),
            managed_collections: result.managed_collections,
        })
    }

    /// Add one managed component to one closed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn add_component_to_collection(
        &mut self,
        collection_key: &str,
        request: CollectionMembershipRequest,
    ) -> Result<CollectionMembershipResponse, ApiApplicationError> {
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .add_component_to_collection(collection_key, &request.component_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .add_component_to_collection(collection_key, &request.component_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(CollectionMembershipResponse {
            change: result.change.as_str().to_owned(),
            members: result.members,
        })
    }

    /// Remove one managed component from one closed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn remove_component_from_collection(
        &mut self,
        collection_key: &str,
        component_key: &str,
    ) -> Result<CollectionMembershipResponse, ApiApplicationError> {
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .remove_component_from_collection(collection_key, component_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .remove_component_from_collection(collection_key, component_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(CollectionMembershipResponse {
            change: result.change.as_str().to_owned(),
            members: result.members,
        })
    }

    /// Configure one periodic scan schedule for one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid or the durable write fails.
    pub async fn configure_collection_scan_schedule(
        &mut self,
        collection_key: &str,
        request: ConfigureCollectionScanScheduleRequest,
    ) -> Result<ConfigureCollectionScanScheduleResponse, ApiApplicationError> {
        let freshness = parse_freshness(&request.freshness)?;
        if request.cadence_minutes == 0 {
            return Err(ApiApplicationError::InvalidRequest(
                "cadence_minutes must be greater than zero".to_owned(),
            ));
        }
        let next_due_at_unix_ms = current_unix_millis()?;

        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .configure_collection_scan_schedule(
                    collection_key,
                    request.cadence_minutes,
                    freshness,
                    next_due_at_unix_ms,
                )
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .configure_collection_scan_schedule(
                    collection_key,
                    request.cadence_minutes,
                    freshness,
                    next_due_at_unix_ms,
                )
                .await
                .map_err(ApiApplicationError::State)?,
        };

        let Some(schedule) = result.schedule else {
            return Err(ApiApplicationError::InvalidRequest(format!(
                "unknown collection: {collection_key}"
            )));
        };

        Ok(ConfigureCollectionScanScheduleResponse {
            change: result.change.as_str().to_owned(),
            collection_key: collection_key.to_owned(),
            cadence_minutes: schedule.cadence_minutes,
            freshness: freshness_name(schedule.freshness).to_owned(),
            next_due_at_unix_ms: schedule.next_due_at_unix_ms,
        })
    }

    /// Configure the runtime provider that one managed component will use for scan execution.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the provider key is unsupported or the durable write fails.
    pub async fn configure_provider(
        &mut self,
        component_key: &str,
        request: ConfigureProviderRequest,
    ) -> Result<ConfigureProviderResponse, ApiApplicationError> {
        let provider_key = resolve_supported_provider_key(&request.provider_key)?;
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .configure_provider(component_key, provider_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .configure_provider(component_key, provider_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(ConfigureProviderResponse {
            change: result.change.as_str().to_owned(),
            provider_key: result.provider_key.map(Into::into),
        })
    }

    /// Configure the system integration publication runtime.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid or the durable write fails.
    pub async fn configure_integration_runtime(
        &mut self,
        request: ConfigureIntegrationRuntimeRequest,
    ) -> Result<ConfigureIntegrationRuntimeResponse, ApiApplicationError> {
        let config = parse_integration_runtime_config(request)?;
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .configure_integration_runtime(config)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .configure_integration_runtime(config)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(ConfigureIntegrationRuntimeResponse::from(
            result.change.as_str(),
            &result.config,
        ))
    }

    /// Record one canonical provider report through the application boundary.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid or the durable state write fails.
    pub async fn record_provider_report(
        &mut self,
        request: ProviderScanReportRequest,
    ) -> Result<RecordProviderReportResponse, ApiApplicationError> {
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
            ApiStore::Local(local) => local
                .state
                .record_scan_report(&report)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .record_scan_report(&report)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(RecordProviderReportResponse {
            discovered: result.discovered,
            repeated: result.repeated,
            withdrawn: result.withdrawn,
            active: result.active,
        })
    }

    /// Create and durably enqueue one canonical scan request for managed ownership.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, ownership is unmanaged,
    /// or the durable runtime cannot append the command.
    pub async fn request_scan(
        &mut self,
        request: RequestScanCommand,
    ) -> Result<RequestScanResponse, ApiApplicationError> {
        let artifact = ArtifactRef::new(
            parse_artifact_kind(&request.artifact_kind)?,
            request.artifact_identity.clone(),
        );
        let freshness = parse_freshness(&request.freshness)?;
        let command_id = match &mut self.backend {
            ApiStore::Local(local) => {
                let scan_request = ScanPlanner::new(local.state.ingestion().inventory())
                    .plan(&request.component_key, artifact, freshness)
                    .map_err(|error| {
                        ApiApplicationError::InvalidRequest(error.as_str().to_owned())
                    })?;
                local
                    .runtime
                    .enqueue(scan_request)
                    .map_err(|error| ApiApplicationError::State(error.to_string()))?
                    .command_id
            }
            ApiStore::Postgres(postgres) => postgres
                .request_scan(&request.component_key, artifact, freshness)
                .await
                .map_err(ApiApplicationError::State)?,
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

    /// Create and durably enqueue one canonical scan batch for one closed release collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the collection is unmanaged,
    /// or the durable runtime cannot append the commands.
    pub async fn request_collection_scan(
        &mut self,
        collection_key: &str,
        request: RequestCollectionScanCommand,
    ) -> Result<RequestCollectionScanResponse, ApiApplicationError> {
        let freshness = parse_freshness(&request.freshness)?;
        let command_ids = match &mut self.backend {
            ApiStore::Local(local) => {
                let batch = ScanPlanner::new(local.state.ingestion().inventory())
                    .plan_collection(collection_key, freshness)
                    .map_err(|error| {
                        ApiApplicationError::InvalidRequest(error.as_str().to_owned())
                    })?;
                let mut command_ids = Vec::with_capacity(batch.requests.len());
                for scan_request in batch.requests {
                    let command = local
                        .runtime
                        .enqueue(scan_request)
                        .map_err(|error| ApiApplicationError::State(error.to_string()))?;
                    command_ids.push(command.command_id.into());
                }
                command_ids
            }
            ApiStore::Postgres(postgres) => postgres
                .request_collection_scan(collection_key, freshness)
                .await
                .map_err(ApiApplicationError::State)?
                .into_iter()
                .map(Into::into)
                .collect(),
        };

        Ok(RequestCollectionScanResponse {
            collection_key: collection_key.to_owned(),
            freshness: request.freshness,
            enqueued: command_ids.len(),
            command_ids,
        })
    }

    /// Materialize due collection scan schedules into canonical durable scan commands.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid or the durable state fails.
    pub async fn run_collection_scan_worker_until_idle(
        &mut self,
        request: DrainCollectionScanWorkerCommand,
    ) -> Result<DrainCollectionScanWorkerResponse, ApiApplicationError> {
        let max_collections = request.max_collections.ok_or_else(|| {
            ApiApplicationError::InvalidRequest("max_collections is required".to_owned())
        })?;
        if max_collections == 0 {
            return Err(ApiApplicationError::InvalidRequest(
                "max_collections must be greater than zero".to_owned(),
            ));
        }

        let now_unix_ms = current_unix_millis()?;
        match &mut self.backend {
            ApiStore::Local(local) => {
                let mut inventory = local.state.ingestion().inventory().clone();
                let due_scans = CollectionScanScheduler::new(&mut inventory)
                    .collect_due(now_unix_ms, max_collections);

                for due_scan in &due_scans {
                    local
                        .state
                        .record_collection_scan_materialization(
                            due_scan.collection_key.as_ref(),
                            due_scan.next_due_at_unix_ms,
                            now_unix_ms,
                            u32::try_from(due_scan.requests.len()).map_err(|_| {
                                ApiApplicationError::State(
                                    "collection scheduler command count overflow".to_owned(),
                                )
                            })?,
                        )
                        .map_err(|error| ApiApplicationError::State(error.to_string()))?;
                }

                let processed_collections = due_scans.len();
                let mut enqueued_commands = 0_usize;
                let mut last_collection_key = None;
                for due_scan in due_scans {
                    enqueued_commands += due_scan.requests.len();
                    last_collection_key = Some(due_scan.collection_key.to_string());
                    for scan_request in due_scan.requests {
                        let _ = local
                            .runtime
                            .enqueue(scan_request)
                            .map_err(|error| ApiApplicationError::State(error.to_string()))?;
                    }
                }

                let pending_due_remaining =
                    inventory.due_collection_keys(now_unix_ms, usize::MAX).len();
                let outcome = if processed_collections == 0 {
                    "idle"
                } else if pending_due_remaining == 0 {
                    "drained"
                } else {
                    "limited"
                };

                Ok(DrainCollectionScanWorkerResponse {
                    outcome: outcome.to_owned(),
                    processed_collections,
                    enqueued_commands,
                    pending_due_remaining,
                    last_collection_key,
                })
            }
            ApiStore::Postgres(postgres) => {
                let result = postgres
                    .drain_due_collection_scans(max_collections, now_unix_ms)
                    .await
                    .map_err(ApiApplicationError::State)?;
                Ok(DrainCollectionScanWorkerResponse::from(result))
            }
        }
    }

    /// Drain pending scan commands through one bounded worker loop.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the provider input or the worker limit is invalid,
    /// or when the durable runtime/state fails.
    pub async fn run_worker_until_idle(
        &mut self,
        request: DrainWorkerCommand,
    ) -> Result<DrainWorkerResponse, ApiApplicationError> {
        let max_commands = request.max_commands.ok_or_else(|| {
            ApiApplicationError::InvalidRequest("max_commands is required".to_owned())
        })?;
        if max_commands == 0 {
            return Err(ApiApplicationError::InvalidRequest(
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
                ApiStore::Local(local) => local
                    .runtime
                    .run_next(&mut local.state, &provider)
                    .await
                    .map_err(|error| ApiApplicationError::State(error.to_string()))?,
                ApiStore::Postgres(postgres) => postgres
                    .run_next(&provider)
                    .await
                    .map_err(ApiApplicationError::State)?,
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
            ApiStore::Local(local) => local.runtime.pending_commands(),
            ApiStore::Postgres(postgres) => postgres.pending_commands(),
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
    /// Returns [`ApiApplicationError`] when the request is invalid or publication outcome
    /// persistence fails.
    pub async fn publish_integration_events_until_idle(
        &mut self,
        request: DrainIntegrationWorkerCommand,
    ) -> Result<DrainIntegrationWorkerResponse, ApiApplicationError> {
        let max_events = request.max_events.ok_or_else(|| {
            ApiApplicationError::InvalidRequest("max_events is required".to_owned())
        })?;
        if max_events == 0 {
            return Err(ApiApplicationError::InvalidRequest(
                "max_events must be greater than zero".to_owned(),
            ));
        }

        let config = self.integration_runtime_config().cloned().ok_or_else(|| {
            ApiApplicationError::State("missing integration runtime configuration".to_owned())
        })?;
        let publisher = ApiIntegrationPublisher::new(&config, request)?;
        let attempted_events = self
            .pending_integration_events_snapshot()
            .into_iter()
            .take(max_events)
            .collect::<Vec<_>>();

        let result = match &mut self.backend {
            ApiStore::Local(local) => {
                publish_pending_local_integration_events(local, max_events, &publisher)
                    .await
                    .map_err(ApiApplicationError::State)?
            }
            ApiStore::Postgres(postgres) => postgres
                .publish_pending_integration_events(max_events, &publisher)
                .await
                .map_err(ApiApplicationError::State)?,
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
    /// Returns [`ApiApplicationError`] when the provider input is invalid or the durable runtime/state fails.
    pub async fn run_next_scan(
        &mut self,
        request: RunNextScanCommand,
    ) -> Result<RunNextScanResponse, ApiApplicationError> {
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
        let outcome = match &mut self.backend {
            ApiStore::Local(local) => local
                .runtime
                .run_next(&mut local.state, &provider)
                .await
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .run_next(&provider)
                .await
                .map_err(ApiApplicationError::State)?,
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

    fn next_pending_provider_key(&self) -> Result<Option<&'static str>, ApiApplicationError> {
        let Some(component_key) = self.next_pending_component_key() else {
            return Ok(None);
        };
        let Some(provider_key) = self.configured_provider(component_key) else {
            return Err(ApiApplicationError::State(format!(
                "missing provider runtime configuration for component: {component_key}"
            )));
        };
        resolve_supported_provider_key(provider_key).map(Some)
    }

    fn next_pending_component_key(&self) -> Option<&str> {
        match &self.backend {
            ApiStore::Local(local) => local.runtime.next_pending_component_key(),
            ApiStore::Postgres(postgres) => postgres.next_pending_component_key(),
        }
    }

    fn configured_provider(&self, component_key: &str) -> Option<&str> {
        match &self.backend {
            ApiStore::Local(local) => local
                .state
                .ingestion()
                .inventory()
                .configured_provider(component_key),
            ApiStore::Postgres(postgres) => postgres.configured_provider(component_key),
        }
    }

    fn pending_integration_events_snapshot(&self) -> Vec<PendingIntegrationEvent> {
        match &self.backend {
            ApiStore::Local(local) => local
                .state
                .pending_integration_events()
                .iter()
                .chain(local.runtime.pending_integration_events().iter())
                .cloned()
                .collect(),
            ApiStore::Postgres(postgres) => postgres.pending_integration_events().to_vec(),
        }
    }

    const fn integration_runtime_config(&self) -> Option<&IntegrationRuntimeConfig> {
        match &self.backend {
            ApiStore::Local(local) => local.state.integration_runtime_config(),
            ApiStore::Postgres(postgres) => postgres.integration_runtime_config(),
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
pub struct CollectionRegistrationRequest {
    pub collection_key: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterCollectionResponse {
    pub change: String,
    pub managed_collections: usize,
}

#[derive(Debug, Deserialize)]
pub struct CollectionMembershipRequest {
    pub component_key: String,
}

#[derive(Debug, Serialize)]
pub struct CollectionMembershipResponse {
    pub change: String,
    pub members: usize,
}

#[derive(Debug, Serialize)]
pub struct ListCollectionsResponse {
    pub managed_collections: usize,
    pub collections: Vec<CollectionSummary>,
}

#[derive(Debug, Serialize)]
pub struct CollectionSummary {
    pub collection_key: String,
    pub name: String,
    pub members: usize,
    pub scan_schedule: Option<CollectionScanScheduleItem>,
    pub due_now: bool,
}

#[derive(Debug, Serialize)]
pub struct CollectionDetailResponse {
    pub collection_key: String,
    pub name: String,
    pub scan_schedule: Option<CollectionScanScheduleItem>,
    pub members: Vec<CollectionMemberItem>,
}

#[derive(Debug, Serialize)]
pub struct CollectionMemberItem {
    pub component_key: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CollectionScanScheduleItem {
    pub cadence_minutes: u32,
    pub freshness: &'static str,
    pub next_due_at_unix_ms: u64,
    pub last_materialized_at_unix_ms: Option<u64>,
    pub last_enqueued_commands: Option<u32>,
}

impl From<venom_domain::CollectionScanSchedule> for CollectionScanScheduleItem {
    fn from(value: venom_domain::CollectionScanSchedule) -> Self {
        Self {
            cadence_minutes: value.cadence_minutes,
            freshness: freshness_name(value.freshness),
            next_due_at_unix_ms: value.next_due_at_unix_ms,
            last_materialized_at_unix_ms: value.last_materialized_at_unix_ms,
            last_enqueued_commands: value.last_enqueued_commands,
        }
    }
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
pub struct ConfigureCollectionScanScheduleRequest {
    pub cadence_minutes: u32,
    pub freshness: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigureCollectionScanScheduleResponse {
    pub change: String,
    pub collection_key: String,
    pub cadence_minutes: u32,
    pub freshness: String,
    pub next_due_at_unix_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureIntegrationRuntimeRequest {
    pub publisher_key: String,
    pub endpoint_url: Option<String>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ConfigureIntegrationRuntimeResponse {
    pub change: String,
    pub publisher_key: String,
    pub endpoint_url: Option<String>,
    pub timeout_ms: Option<u32>,
}

impl ConfigureIntegrationRuntimeResponse {
    fn from(change: &str, config: &IntegrationRuntimeConfig) -> Self {
        Self {
            change: change.to_owned(),
            publisher_key: config.publisher_key().to_owned(),
            endpoint_url: config.endpoint_url().map(ToOwned::to_owned),
            timeout_ms: config.timeout_ms(),
        }
    }
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
    fn into_domain(self) -> Result<ReportedFinding, ApiApplicationError> {
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

#[derive(Debug, Deserialize)]
pub struct RequestCollectionScanCommand {
    pub freshness: String,
}

#[derive(Debug, Serialize)]
pub struct RequestCollectionScanResponse {
    pub collection_key: String,
    pub freshness: String,
    pub enqueued: usize,
    pub command_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DrainCollectionScanWorkerCommand {
    pub max_collections: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct DrainCollectionScanWorkerResponse {
    pub outcome: String,
    pub processed_collections: usize,
    pub enqueued_commands: usize,
    pub pending_due_remaining: usize,
    pub last_collection_key: Option<String>,
}

impl From<DrainDueCollectionScansResult> for DrainCollectionScanWorkerResponse {
    fn from(value: DrainDueCollectionScansResult) -> Self {
        Self {
            outcome: value.outcome.into(),
            processed_collections: value.processed_collections,
            enqueued_commands: value.enqueued_commands,
            pending_due_remaining: value.pending_due_remaining,
            last_collection_key: value.last_collection_key.map(Into::into),
        }
    }
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

fn parse_artifact_kind(value: &str) -> Result<ArtifactKind, ApiApplicationError> {
    match value {
        "container-image" => Ok(ArtifactKind::ContainerImage),
        "sbom-document" => Ok(ArtifactKind::SbomDocument),
        _ => Err(ApiApplicationError::InvalidRequest(format!(
            "unsupported artifact kind: {value}"
        ))),
    }
}

fn parse_freshness(value: &str) -> Result<EvidenceFreshness, ApiApplicationError> {
    match value {
        "deterministic" => Ok(EvidenceFreshness::Deterministic),
        "live" => Ok(EvidenceFreshness::Live),
        _ => Err(ApiApplicationError::InvalidRequest(format!(
            "unsupported freshness: {value}"
        ))),
    }
}

const fn freshness_name(value: EvidenceFreshness) -> &'static str {
    match value {
        EvidenceFreshness::Deterministic => "deterministic",
        EvidenceFreshness::Live => "live",
    }
}

fn current_unix_millis() -> Result<u64, ApiApplicationError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ApiApplicationError::State(error.to_string()))?;
    u64::try_from(duration.as_millis())
        .map_err(|_| ApiApplicationError::State("current unix millis overflow".to_owned()))
}

fn parse_severity(value: &str) -> Result<Severity, ApiApplicationError> {
    match value {
        "unknown" => Ok(Severity::Unknown),
        "none" => Ok(Severity::None),
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        _ => Err(ApiApplicationError::InvalidRequest(format!(
            "unsupported severity: {value}"
        ))),
    }
}

fn parse_integration_runtime_config(
    request: ConfigureIntegrationRuntimeRequest,
) -> Result<IntegrationRuntimeConfig, ApiApplicationError> {
    match request.publisher_key.as_str() {
        API_INTEGRATION_PUBLISHER_KEY => {
            if request.endpoint_url.is_some() || request.timeout_ms.is_some() {
                return Err(ApiApplicationError::InvalidRequest(
                    "fixture publisher does not accept endpoint_url or timeout_ms".to_owned(),
                ));
            }
            Ok(IntegrationRuntimeConfig::Fixture)
        }
        HTTP_EVENT_PUBLISHER_KEY => {
            let endpoint_url = request.endpoint_url.ok_or_else(|| {
                ApiApplicationError::InvalidRequest(
                    "http publisher requires endpoint_url".to_owned(),
                )
            })?;
            let timeout_ms = request.timeout_ms.unwrap_or(3_000);
            if timeout_ms == 0 {
                return Err(ApiApplicationError::InvalidRequest(
                    "http publisher timeout_ms must be greater than zero".to_owned(),
                ));
            }
            Ok(IntegrationRuntimeConfig::Http {
                endpoint_url: endpoint_url.into_boxed_str(),
                timeout_ms,
            })
        }
        value => Err(ApiApplicationError::InvalidRequest(format!(
            "unsupported publisher key: {value}"
        ))),
    }
}

fn build_active_findings_query(
    request: &ActiveFindingsRequest,
    artifact: ArtifactRef,
) -> Result<ActiveFindingsQuery, ApiApplicationError> {
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
    ) -> Result<Self, ApiApplicationError> {
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

fn resolve_supported_provider_key(value: &str) -> Result<&'static str, ApiApplicationError> {
    match value {
        API_WORKER_PROVIDER_KEY => Ok(API_WORKER_PROVIDER_KEY),
        _ => Err(ApiApplicationError::InvalidRequest(format!(
            "unsupported provider key: {value}"
        ))),
    }
}

fn parse_error_kind(value: &str) -> Result<FindingProviderErrorKind, ApiApplicationError> {
    match value {
        "invalid-request" => Ok(FindingProviderErrorKind::InvalidRequest),
        "unavailable" => Ok(FindingProviderErrorKind::Unavailable),
        "unauthorized" => Ok(FindingProviderErrorKind::Unauthorized),
        "corrupt-response" => Ok(FindingProviderErrorKind::CorruptResponse),
        "rate-limited" => Ok(FindingProviderErrorKind::RateLimited),
        _ => Err(ApiApplicationError::InvalidRequest(format!(
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
    Http(HttpEventPublisher),
}

impl ApiIntegrationPublisher {
    fn new(
        config: &IntegrationRuntimeConfig,
        request: DrainIntegrationWorkerCommand,
    ) -> Result<Self, ApiApplicationError> {
        Ok(match config {
            IntegrationRuntimeConfig::Fixture => {
                if let Some(message) = request.error_message {
                    Self {
                        mode: ApiIntegrationPublisherMode::Failure(
                            IntegrationEventPublishError::new(
                                request.retryable.unwrap_or(false),
                                message,
                            ),
                        ),
                    }
                } else {
                    Self {
                        mode: ApiIntegrationPublisherMode::Success,
                    }
                }
            }
            IntegrationRuntimeConfig::Http {
                endpoint_url,
                timeout_ms,
            } => {
                if request.error_message.is_some() || request.retryable.is_some() {
                    return Err(ApiApplicationError::InvalidRequest(
                        "http publisher does not accept fixture failure controls".to_owned(),
                    ));
                }
                Self {
                    mode: ApiIntegrationPublisherMode::Http(
                        HttpEventPublisher::new(endpoint_url.clone(), *timeout_ms)
                            .map_err(ApiApplicationError::State)?,
                    ),
                }
            }
        })
    }
}

impl IntegrationEventPublisher for ApiIntegrationPublisher {
    fn publisher_key(&self) -> &'static str {
        match &self.mode {
            ApiIntegrationPublisherMode::Success | ApiIntegrationPublisherMode::Failure(_) => {
                API_INTEGRATION_PUBLISHER_KEY
            }
            ApiIntegrationPublisherMode::Http(publisher) => publisher.publisher_key(),
        }
    }

    async fn publish<'a>(
        &'a self,
        event: &'a PendingIntegrationEvent,
    ) -> Result<(), IntegrationEventPublishError> {
        match &self.mode {
            ApiIntegrationPublisherMode::Success => Ok(()),
            ApiIntegrationPublisherMode::Failure(error) => Err(error.clone()),
            ApiIntegrationPublisherMode::Http(publisher) => publisher.publish(event).await,
        }
    }
}

async fn publish_pending_local_integration_events(
    local: &mut LocalStore,
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
