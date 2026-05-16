use sqlx::{PgPool, postgres::PgPoolOptions, types::Json};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use venom_domain::{
    ActiveFindingsPage, ActiveFindingsQuery, ArtifactKind, ArtifactRef, BindArtifactChange,
    BindArtifactResult, CompletedScanCommand, ComponentRegistration, ConfigureProviderChange,
    ConfigureProviderResult, EvidenceFreshness, FailedScanCommand, FindingChangeSet,
    FindingIngestion, FindingProvider, FindingProviderError, FindingProviderErrorKind,
    FindingReadModel, IntegrationEventPublicationFailure, IntegrationEventPublisher,
    PendingIntegrationEvent, ProviderScanReport, PublishIntegrationEventsResult,
    RegisterComponentChange, RegisterComponentResult, ReportedFinding, RunNextScanResult,
    ScanCommandStatus, ScanPlanner, ScanRequest, as_provider_error, validate_provider_scan_report,
};

#[derive(Debug)]
pub struct PostgresBackend {
    pool: PgPool,
    names: TableNames,
    ingestion: FindingIngestion,
    read_model: FindingReadModel,
    commands: BTreeMap<Box<str>, ScanCommandRecord>,
    order: Vec<Box<str>>,
    pending_integration_events: Vec<PendingIntegrationEvent>,
}

impl PostgresBackend {
    /// Open or create the Postgres durable backend and rebuild in-memory state.
    ///
    /// # Errors
    ///
    /// Returns an error string when Postgres cannot be reached, initialized, or replayed.
    pub async fn open(database_url: &str, schema: &str) -> Result<Self, String> {
        let names = TableNames::new(schema)?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|error| format!("postgres connect failed: {error}"))?;

        let mut backend = Self {
            pool,
            names,
            ingestion: FindingIngestion::new(),
            read_model: FindingReadModel::new(),
            commands: BTreeMap::new(),
            order: Vec::new(),
            pending_integration_events: Vec::new(),
        };
        backend.init_schema().await?;
        backend.rebuild().await?;
        Ok(backend)
    }

    /// Durably register one managed component in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn register_component(
        &mut self,
        registration: ComponentRegistration,
    ) -> Result<RegisterComponentResult, String> {
        let mut candidate = self.ingestion.clone();
        let result = candidate.inventory_mut().register(registration.clone());
        if result.change == RegisterComponentChange::Registered {
            sqlx::query(&format!(
                "INSERT INTO {} (component_key, name) VALUES ($1, $2)",
                self.names.components
            ))
            .bind(registration.component_key.as_ref())
            .bind(registration.name.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres component insert failed: {error}"))?;
            self.ingestion = candidate;
        }
        Ok(result)
    }

    /// Durably bind one immutable artifact to one managed component in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn bind_artifact(
        &mut self,
        component_key: &str,
        artifact: ArtifactRef,
    ) -> Result<BindArtifactResult, String> {
        let mut candidate = self.ingestion.clone();
        let result = candidate
            .inventory_mut()
            .bind_artifact(component_key, artifact.clone());
        if result.change == BindArtifactChange::Bound {
            sqlx::query(&format!(
                "INSERT INTO {} (component_key, artifact_kind, artifact_identity) VALUES ($1, $2, $3)",
                self.names.artifact_bindings
            ))
            .bind(component_key)
            .bind(artifact_kind_name(artifact.kind))
            .bind(artifact.identity.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres artifact binding insert failed: {error}"))?;
            self.ingestion = candidate;
        }
        Ok(result)
    }

    /// Durably configure one provider runtime for a managed component in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn configure_provider(
        &mut self,
        component_key: &str,
        provider_key: &str,
    ) -> Result<ConfigureProviderResult, String> {
        let mut candidate = self.ingestion.clone();
        let result = candidate
            .inventory_mut()
            .configure_provider(component_key, provider_key);
        if result.change == ConfigureProviderChange::Configured {
            sqlx::query(&format!(
                concat!(
                    "INSERT INTO {} (component_key, provider_key) VALUES ($1, $2) ",
                    "ON CONFLICT (component_key) DO UPDATE SET provider_key = EXCLUDED.provider_key, updated_at = NOW()"
                ),
                self.names.provider_runtime_configs
            ))
            .bind(component_key)
            .bind(provider_key)
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres provider config upsert failed: {error}"))?;
            self.ingestion = candidate;
        }
        Ok(result)
    }

    /// Durably record one canonical provider report in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the report is invalid for managed ownership or the durable write fails.
    pub async fn record_scan_report(
        &mut self,
        report: &ProviderScanReport,
    ) -> Result<FindingChangeSet, String> {
        let mut candidate_ingestion = self.ingestion.clone();
        let mut candidate_read_model = self.read_model.clone();
        let change_set = candidate_ingestion
            .record_scan_report(report)
            .map_err(|error| format!("provider report cannot be applied: {}", error.as_str()))?;
        candidate_read_model.record_scan_report(report);
        let pending_integration_event = PendingIntegrationEvent::finding_changes_observed(
            report.component_key.clone(),
            report.artifact.clone(),
            report.provider_key.clone(),
            report.freshness,
            report.observed_at,
            change_set.clone(),
        );

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| format!("postgres transaction begin failed: {error}"))?;

        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(provider_key, component_key, artifact_kind, artifact_identity, observed_at_micros, freshness, knowledge_revision, findings) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
            ),
            self.names.provider_reports
        ))
        .bind(report.provider_key.as_ref())
        .bind(report.component_key.as_ref())
        .bind(artifact_kind_name(report.artifact.kind))
        .bind(report.artifact.identity.as_ref())
        .bind(system_time_to_micros(report.observed_at)?)
        .bind(freshness_name(report.freshness))
        .bind(report.knowledge_revision.as_deref())
        .bind(Json(report.findings.clone()))
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("postgres provider report insert failed: {error}"))?;

        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(event_id, event_kind, payload, publication_status) VALUES ($1, $2, $3, $4)"
            ),
            self.names.integration_outbox
        ))
        .bind(pending_integration_event.event_id.as_ref())
        .bind(pending_integration_event.event.kind_name())
        .bind(Json(pending_integration_event.clone()))
        .bind("pending")
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("postgres integration outbox insert failed: {error}"))?;

        transaction
            .commit()
            .await
            .map_err(|error| format!("postgres transaction commit failed: {error}"))?;

        self.ingestion = candidate_ingestion;
        self.read_model = candidate_read_model;
        self.pending_integration_events
            .push(pending_integration_event);
        Ok(change_set)
    }

    #[must_use]
    pub fn active_findings(
        &self,
        component_key: &str,
        artifact: &ArtifactRef,
    ) -> Vec<ReportedFinding> {
        self.read_model.active_findings(component_key, artifact)
    }

    #[must_use]
    pub fn query_active_findings(&self, query: &ActiveFindingsQuery) -> ActiveFindingsPage {
        self.read_model.query_active_findings(query)
    }

    #[must_use]
    pub fn pending_commands(&self) -> usize {
        self.commands
            .values()
            .filter(|command| command.status == ScanCommandStatus::Pending)
            .count()
    }

    #[must_use]
    pub fn next_pending_component_key(&self) -> Option<&str> {
        self.order.iter().find_map(|command_id| {
            self.commands.get(command_id.as_ref()).and_then(|record| {
                (record.status == ScanCommandStatus::Pending)
                    .then_some(record.request.component_key.as_ref())
            })
        })
    }

    #[must_use]
    pub fn configured_provider(&self, component_key: &str) -> Option<&str> {
        self.ingestion
            .inventory()
            .configured_provider(component_key)
    }

    /// Durably enqueue one canonical scan request in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the ownership is unmanaged or the durable write fails.
    pub async fn request_scan(
        &mut self,
        component_key: &str,
        artifact: ArtifactRef,
        freshness: EvidenceFreshness,
    ) -> Result<Box<str>, String> {
        let request = ScanPlanner::new(self.ingestion.inventory())
            .plan(component_key, artifact, freshness)
            .map_err(|error| error.as_str().to_owned())?;
        let command_id = next_command_id();

        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(command_id, component_key, artifact_kind, artifact_identity, freshness, status) ",
                "VALUES ($1, $2, $3, $4, $5, $6)"
            ),
            self.names.scan_commands
        ))
        .bind(command_id.as_ref())
        .bind(request.component_key.as_ref())
        .bind(artifact_kind_name(request.artifact.kind))
        .bind(request.artifact.identity.as_ref())
        .bind(freshness_name(request.freshness))
        .bind(scan_command_status_name(ScanCommandStatus::Pending))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres scan command insert failed: {error}"))?;

        self.order.push(command_id.clone());
        self.commands.insert(
            command_id.clone(),
            ScanCommandRecord {
                request,
                status: ScanCommandStatus::Pending,
            },
        );
        Ok(command_id)
    }

    #[must_use]
    pub fn command_status(&self, command_id: &str) -> Option<ScanCommandStatus> {
        self.commands.get(command_id).map(|record| record.status)
    }

    #[must_use]
    pub fn pending_integration_events(&self) -> &[PendingIntegrationEvent] {
        &self.pending_integration_events
    }

    /// Publish a bounded batch of pending integration events.
    ///
    /// # Errors
    ///
    /// Returns an error string when publication outcome persistence fails.
    pub async fn publish_pending_integration_events(
        &mut self,
        max_events: usize,
        publisher: &(impl IntegrationEventPublisher + Sync),
    ) -> Result<PublishIntegrationEventsResult, String> {
        let mut result = PublishIntegrationEventsResult {
            attempted: 0,
            published: 0,
            pending_remaining: self.pending_integration_events.len(),
            last_failure: None,
        };
        if max_events == 0 {
            return Ok(result);
        }

        let batch = self
            .pending_integration_events
            .iter()
            .take(max_events)
            .cloned()
            .collect::<Vec<_>>();

        for event in batch {
            result.attempted += 1;
            let attempted_at_micros = system_time_to_micros(SystemTime::now())?;
            match publisher.publish(&event).await {
                Ok(()) => {
                    sqlx::query(&format!(
                        concat!(
                            "UPDATE {} ",
                            "SET publication_status = 'published', last_error = NULL, ",
                            "last_attempted_at_micros = $2, published_at_micros = $3, attempt_count = attempt_count + 1 ",
                            "WHERE event_id = $1"
                        ),
                        self.names.integration_outbox
                    ))
                    .bind(event.event_id.as_ref())
                    .bind(attempted_at_micros)
                    .bind(attempted_at_micros)
                    .execute(&self.pool)
                    .await
                    .map_err(|error| format!("postgres integration outbox publish update failed: {error}"))?;
                    self.remove_pending_integration_event(event.event_id.as_ref());
                    result.published += 1;
                }
                Err(error) => {
                    sqlx::query(&format!(
                        concat!(
                            "UPDATE {} ",
                            "SET publication_status = 'pending', last_error = $2, ",
                            "last_attempted_at_micros = $3, attempt_count = attempt_count + 1 ",
                            "WHERE event_id = $1"
                        ),
                        self.names.integration_outbox
                    ))
                    .bind(event.event_id.as_ref())
                    .bind(error.message.as_ref())
                    .bind(attempted_at_micros)
                    .execute(&self.pool)
                    .await
                    .map_err(|sql_error| {
                        format!("postgres integration outbox failure update failed: {sql_error}")
                    })?;
                    result.last_failure = Some(IntegrationEventPublicationFailure {
                        event_id: event.event_id,
                        retryable: error.retryable,
                        message: error.message,
                    });
                    break;
                }
            }
        }

        result.pending_remaining = self.pending_integration_events.len();
        Ok(result)
    }

    /// Execute the oldest pending durable scan command through one provider.
    ///
    /// # Errors
    ///
    /// Returns an error string when Postgres cannot durably persist the command result.
    pub async fn run_next(
        &mut self,
        provider: &(impl FindingProvider + Sync),
    ) -> Result<RunNextScanResult, String> {
        let Some(command_id) = self
            .order
            .iter()
            .find(|command_id| {
                self.command_status(command_id.as_ref()) == Some(ScanCommandStatus::Pending)
            })
            .cloned()
        else {
            return Ok(RunNextScanResult::Idle);
        };

        let Some(request) = self
            .commands
            .get(command_id.as_ref())
            .map(|record| record.request.clone())
        else {
            return Err("pending scan command missing from postgres runtime".to_owned());
        };

        match provider.scan(&request).await {
            Ok(report) => {
                self.complete_scan_command(command_id, request, provider.provider_key(), report)
                    .await
            }
            Err(error) => self.fail_scan_command(command_id, error).await,
        }
    }

    async fn complete_scan_command(
        &mut self,
        command_id: Box<str>,
        request: ScanRequest,
        provider_key: &'static str,
        report: ProviderScanReport,
    ) -> Result<RunNextScanResult, String> {
        if let Err(violation) = validate_provider_scan_report(provider_key, &request, &report) {
            return self
                .fail_scan_command(command_id, as_provider_error(violation))
                .await;
        }

        let mut candidate_ingestion = self.ingestion.clone();
        let mut candidate_read_model = self.read_model.clone();
        let change_set = candidate_ingestion
            .record_scan_report(&report)
            .map_err(|error| format!("provider report cannot be applied: {}", error.as_str()))?;
        candidate_read_model.record_scan_report(&report);
        let findings_reported = report.findings.len();
        let pending_integration_event = PendingIntegrationEvent::scan_command_completed(
            command_id.as_ref(),
            request.component_key.clone(),
            request.artifact.clone(),
            report.provider_key.clone(),
            request.freshness,
            findings_reported,
            change_set.clone(),
        );

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| format!("postgres transaction begin failed: {error}"))?;

        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(provider_key, component_key, artifact_kind, artifact_identity, observed_at_micros, freshness, knowledge_revision, findings) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
            ),
            self.names.provider_reports
        ))
        .bind(report.provider_key.as_ref())
        .bind(report.component_key.as_ref())
        .bind(artifact_kind_name(report.artifact.kind))
        .bind(report.artifact.identity.as_ref())
        .bind(system_time_to_micros(report.observed_at)?)
        .bind(freshness_name(report.freshness))
        .bind(report.knowledge_revision.as_deref())
        .bind(Json(report.findings.clone()))
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("postgres provider report insert failed: {error}"))?;

        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(event_id, event_kind, payload, publication_status) VALUES ($1, $2, $3, $4)"
            ),
            self.names.integration_outbox
        ))
        .bind(pending_integration_event.event_id.as_ref())
        .bind(pending_integration_event.event.kind_name())
        .bind(Json(pending_integration_event.clone()))
        .bind("pending")
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("postgres integration outbox insert failed: {error}"))?;

        sqlx::query(&format!(
            concat!(
                "UPDATE {} ",
                "SET status = $2, updated_at = NOW() ",
                "WHERE command_id = $1"
            ),
            self.names.scan_commands
        ))
        .bind(command_id.as_ref())
        .bind(scan_command_status_name(ScanCommandStatus::Completed))
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("postgres scan command completion failed: {error}"))?;

        transaction
            .commit()
            .await
            .map_err(|error| format!("postgres transaction commit failed: {error}"))?;

        self.ingestion = candidate_ingestion;
        self.read_model = candidate_read_model;
        self.pending_integration_events
            .push(pending_integration_event);
        let Some(command) = self.commands.get_mut(command_id.as_ref()) else {
            return Err("completed scan command missing from postgres runtime".to_owned());
        };
        command.status = ScanCommandStatus::Completed;

        Ok(RunNextScanResult::Completed(CompletedScanCommand {
            command_id,
            provider_key: report.provider_key,
            findings_reported,
            change_set,
        }))
    }

    async fn fail_scan_command(
        &mut self,
        command_id: Box<str>,
        error: FindingProviderError,
    ) -> Result<RunNextScanResult, String> {
        sqlx::query(&format!(
            concat!(
                "UPDATE {} ",
                "SET status = $2, error_code = $3, retryable = $4, detail = $5, updated_at = NOW() ",
                "WHERE command_id = $1"
            ),
            self.names.scan_commands
        ))
        .bind(command_id.as_ref())
        .bind(scan_command_status_name(ScanCommandStatus::Failed))
        .bind(provider_error_code(error.kind))
        .bind(error.retryable)
        .bind(error.message.as_ref())
        .execute(&self.pool)
        .await
        .map_err(|sql_error| format!("postgres scan command failure update failed: {sql_error}"))?;

        let Some(command) = self.commands.get_mut(command_id.as_ref()) else {
            return Err("failed scan command missing from postgres runtime".to_owned());
        };
        command.status = ScanCommandStatus::Failed;

        Ok(RunNextScanResult::Failed(FailedScanCommand {
            command_id,
            error_code: provider_error_code(error.kind).into(),
            retryable: error.retryable,
            detail: error.message,
        }))
    }

    async fn init_schema(&self) -> Result<(), String> {
        sqlx::query(&format!(
            "CREATE SCHEMA IF NOT EXISTS {}",
            self.names.schema
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres schema create failed: {error}"))?;

        self.create_components_table().await?;
        self.create_artifact_bindings_table().await?;
        self.create_provider_runtime_configs_table().await?;
        self.create_provider_reports_table().await?;
        self.create_scan_commands_table().await?;
        self.create_integration_outbox_table().await?;

        Ok(())
    }

    async fn create_components_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "component_key TEXT PRIMARY KEY, ",
                "name TEXT NOT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.components
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres components table create failed: {error}"))?;
        Ok(())
    }

    async fn create_artifact_bindings_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "artifact_kind TEXT NOT NULL, ",
                "artifact_identity TEXT NOT NULL, ",
                "component_key TEXT NOT NULL REFERENCES {}(component_key) ON DELETE CASCADE, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "PRIMARY KEY (artifact_kind, artifact_identity)",
                ")"
            ),
            self.names.artifact_bindings, self.names.components
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres artifact bindings table create failed: {error}"))?;
        Ok(())
    }

    async fn create_provider_runtime_configs_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "component_key TEXT PRIMARY KEY REFERENCES {}(component_key) ON DELETE CASCADE, ",
                "provider_key TEXT NOT NULL, ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.provider_runtime_configs, self.names.components
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres provider runtime configs table create failed: {error}")
        })?;
        Ok(())
    }

    async fn create_provider_reports_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "id BIGSERIAL PRIMARY KEY, ",
                "provider_key TEXT NOT NULL, ",
                "component_key TEXT NOT NULL REFERENCES {}(component_key) ON DELETE CASCADE, ",
                "artifact_kind TEXT NOT NULL, ",
                "artifact_identity TEXT NOT NULL, ",
                "observed_at_micros BIGINT NOT NULL, ",
                "freshness TEXT NOT NULL, ",
                "knowledge_revision TEXT NULL, ",
                "findings JSONB NOT NULL",
                ")"
            ),
            self.names.provider_reports, self.names.components
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres provider reports table create failed: {error}"))?;
        Ok(())
    }

    async fn create_scan_commands_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "order_id BIGSERIAL PRIMARY KEY, ",
                "command_id TEXT NOT NULL UNIQUE, ",
                "component_key TEXT NOT NULL, ",
                "artifact_kind TEXT NOT NULL, ",
                "artifact_identity TEXT NOT NULL, ",
                "freshness TEXT NOT NULL, ",
                "status TEXT NOT NULL, ",
                "error_code TEXT NULL, ",
                "retryable BOOLEAN NULL, ",
                "detail TEXT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.scan_commands
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres scan commands table create failed: {error}"))?;
        Ok(())
    }

    async fn create_integration_outbox_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "order_id BIGSERIAL PRIMARY KEY, ",
                "event_id TEXT NOT NULL UNIQUE, ",
                "event_kind TEXT NOT NULL, ",
                "payload JSONB NOT NULL, ",
                "publication_status TEXT NOT NULL, ",
                "attempt_count BIGINT NOT NULL DEFAULT 0, ",
                "last_error TEXT NULL, ",
                "last_attempted_at_micros BIGINT NULL, ",
                "published_at_micros BIGINT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.integration_outbox
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres integration outbox table create failed: {error}"))?;
        Ok(())
    }

    async fn rebuild(&mut self) -> Result<(), String> {
        self.ingestion = FindingIngestion::new();
        self.read_model = FindingReadModel::new();
        self.commands.clear();
        self.order.clear();
        self.pending_integration_events.clear();

        self.load_components().await?;
        self.load_artifact_bindings().await?;
        self.load_provider_runtime_configs().await?;
        self.load_provider_reports().await?;
        self.load_scan_commands().await?;
        self.load_pending_integration_events().await?;

        Ok(())
    }

    async fn load_components(&mut self) -> Result<(), String> {
        let components = sqlx::query_as::<_, (String, String)>(&format!(
            "SELECT component_key, name FROM {} ORDER BY created_at, component_key",
            self.names.components
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres components load failed: {error}"))?;
        for (component_key, name) in components {
            let result = self
                .ingestion
                .inventory_mut()
                .register(ComponentRegistration::new(component_key, name));
            if result.change == RegisterComponentChange::Rejected {
                return Err("postgres components contain conflicting registration".to_owned());
            }
        }

        Ok(())
    }

    async fn load_artifact_bindings(&mut self) -> Result<(), String> {
        let bindings = sqlx::query_as::<_, (String, String, String)>(&format!(
            concat!(
                "SELECT component_key, artifact_kind, artifact_identity ",
                "FROM {} ORDER BY created_at, component_key, artifact_kind, artifact_identity"
            ),
            self.names.artifact_bindings
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres artifact bindings load failed: {error}"))?;
        for (component_key, artifact_kind, artifact_identity) in bindings {
            let result = self.ingestion.inventory_mut().bind_artifact(
                component_key.as_ref(),
                ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
            );
            if result.change == BindArtifactChange::Rejected {
                return Err("postgres artifact bindings contain conflicting ownership".to_owned());
            }
        }

        Ok(())
    }

    async fn load_provider_reports(&mut self) -> Result<(), String> {
        let reports = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                i64,
                String,
                Option<String>,
                Json<Vec<ReportedFinding>>,
            ),
        >(&format!(
            concat!(
                "SELECT provider_key, component_key, artifact_kind, artifact_identity, ",
                "observed_at_micros, freshness, knowledge_revision, findings ",
                "FROM {} ORDER BY id"
            ),
            self.names.provider_reports
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres provider reports load failed: {error}"))?;
        for (
            provider_key,
            component_key,
            artifact_kind,
            artifact_identity,
            observed_at_micros,
            freshness,
            knowledge_revision,
            findings,
        ) in reports
        {
            let report = ProviderScanReport {
                provider_key: provider_key.into_boxed_str(),
                component_key: component_key.into_boxed_str(),
                artifact: ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
                observed_at: micros_to_system_time(observed_at_micros)?,
                freshness: parse_freshness(&freshness)?,
                knowledge_revision: knowledge_revision.map(String::into_boxed_str),
                findings: findings.0,
            };
            self.ingestion
                .record_scan_report(&report)
                .map_err(|error| {
                    format!("postgres provider report replay failed: {}", error.as_str())
                })?;
            self.read_model.record_scan_report(&report);
        }

        Ok(())
    }

    async fn load_provider_runtime_configs(&mut self) -> Result<(), String> {
        let configs = sqlx::query_as::<_, (String, String)>(&format!(
            "SELECT component_key, provider_key FROM {} ORDER BY component_key",
            self.names.provider_runtime_configs
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres provider runtime configs load failed: {error}"))?;
        for (component_key, provider_key) in configs {
            let result = self
                .ingestion
                .inventory_mut()
                .configure_provider(&component_key, provider_key);
            if result.change == ConfigureProviderChange::Rejected {
                return Err(
                    "postgres provider runtime config references unknown component".to_owned(),
                );
            }
        }

        Ok(())
    }

    async fn load_scan_commands(&mut self) -> Result<(), String> {
        let commands = sqlx::query_as::<_, (String, String, String, String, String, String)>(&format!(
            concat!(
                "SELECT command_id, component_key, artifact_kind, artifact_identity, freshness, status ",
                "FROM {} ORDER BY order_id"
            ),
            self.names.scan_commands
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres scan commands load failed: {error}"))?;
        for (command_id, component_key, artifact_kind, artifact_identity, freshness, status) in
            commands
        {
            let command_id = command_id.into_boxed_str();
            let request = ScanRequest::new(
                component_key,
                ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
                parse_freshness(&freshness)?,
            );
            self.order.push(command_id.clone());
            self.commands.insert(
                command_id,
                ScanCommandRecord {
                    request,
                    status: parse_scan_command_status(&status)?,
                },
            );
        }

        Ok(())
    }

    async fn load_pending_integration_events(&mut self) -> Result<(), String> {
        let events = sqlx::query_as::<_, (Json<PendingIntegrationEvent>,)>(&format!(
            concat!(
                "SELECT payload FROM {} ",
                "WHERE publication_status = 'pending' ORDER BY order_id"
            ),
            self.names.integration_outbox
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres integration outbox load failed: {error}"))?;
        self.pending_integration_events = events.into_iter().map(|(payload,)| payload.0).collect();
        Ok(())
    }

    fn remove_pending_integration_event(&mut self, event_id: &str) {
        if let Some(index) = self
            .pending_integration_events
            .iter()
            .position(|event| event.event_id.as_ref() == event_id)
        {
            self.pending_integration_events.remove(index);
        }
    }
}

#[derive(Debug, Clone)]
struct ScanCommandRecord {
    request: ScanRequest,
    status: ScanCommandStatus,
}

#[derive(Debug)]
struct TableNames {
    schema: Box<str>,
    components: Box<str>,
    artifact_bindings: Box<str>,
    provider_runtime_configs: Box<str>,
    provider_reports: Box<str>,
    scan_commands: Box<str>,
    integration_outbox: Box<str>,
}

impl TableNames {
    fn new(schema: &str) -> Result<Self, String> {
        let schema = validate_schema_name(schema)?;
        Ok(Self {
            components: format!("{schema}.components").into_boxed_str(),
            artifact_bindings: format!("{schema}.artifact_bindings").into_boxed_str(),
            provider_runtime_configs: format!("{schema}.provider_runtime_configs").into_boxed_str(),
            provider_reports: format!("{schema}.provider_reports").into_boxed_str(),
            scan_commands: format!("{schema}.scan_commands").into_boxed_str(),
            integration_outbox: format!("{schema}.integration_outbox").into_boxed_str(),
            schema,
        })
    }
}

fn validate_schema_name(schema: &str) -> Result<Box<str>, String> {
    let mut chars = schema.chars();
    match chars.next() {
        Some(value) if value.is_ascii_alphabetic() || value == '_' => {}
        _ => return Err("invalid postgres schema name".to_owned()),
    }
    if !chars.all(|value| value.is_ascii_alphanumeric() || value == '_') {
        return Err("invalid postgres schema name".to_owned());
    }
    Ok(schema.to_owned().into_boxed_str())
}

const fn artifact_kind_name(value: ArtifactKind) -> &'static str {
    match value {
        ArtifactKind::ContainerImage => "container-image",
        ArtifactKind::SbomDocument => "sbom-document",
    }
}

fn parse_artifact_kind(value: &str) -> Result<ArtifactKind, String> {
    match value {
        "container-image" => Ok(ArtifactKind::ContainerImage),
        "sbom-document" => Ok(ArtifactKind::SbomDocument),
        other => Err(format!("unsupported artifact kind: {other}")),
    }
}

const fn freshness_name(value: EvidenceFreshness) -> &'static str {
    match value {
        EvidenceFreshness::Deterministic => "deterministic",
        EvidenceFreshness::Live => "live",
    }
}

fn parse_freshness(value: &str) -> Result<EvidenceFreshness, String> {
    match value {
        "deterministic" => Ok(EvidenceFreshness::Deterministic),
        "live" => Ok(EvidenceFreshness::Live),
        other => Err(format!("unsupported freshness: {other}")),
    }
}

const fn scan_command_status_name(value: ScanCommandStatus) -> &'static str {
    match value {
        ScanCommandStatus::Pending => "pending",
        ScanCommandStatus::Completed => "completed",
        ScanCommandStatus::Failed => "failed",
    }
}

fn parse_scan_command_status(value: &str) -> Result<ScanCommandStatus, String> {
    match value {
        "pending" => Ok(ScanCommandStatus::Pending),
        "completed" => Ok(ScanCommandStatus::Completed),
        "failed" => Ok(ScanCommandStatus::Failed),
        other => Err(format!("unsupported scan command status: {other}")),
    }
}

const fn provider_error_code(value: FindingProviderErrorKind) -> &'static str {
    match value {
        FindingProviderErrorKind::InvalidRequest => "invalid-request",
        FindingProviderErrorKind::Unavailable => "unavailable",
        FindingProviderErrorKind::Unauthorized => "unauthorized",
        FindingProviderErrorKind::CorruptResponse => "corrupt-response",
        FindingProviderErrorKind::RateLimited => "rate-limited",
    }
}

fn next_command_id() -> Box<str> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    format!("scan-command-{nanos}").into_boxed_str()
}

fn system_time_to_micros(value: SystemTime) -> Result<i64, String> {
    let duration = value
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system time must be after unix epoch: {error}"))?;
    i64::try_from(duration.as_micros()).map_err(|_| "observed_at micros overflow".to_owned())
}

fn micros_to_system_time(value: i64) -> Result<SystemTime, String> {
    let micros =
        u64::try_from(value).map_err(|_| "observed_at micros must be positive".to_owned())?;
    Ok(UNIX_EPOCH + Duration::from_micros(micros))
}

#[cfg(test)]
mod tests {
    use super::PostgresBackend;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use venom_domain::{
        ArtifactKind, ArtifactRef, ComponentRegistration, EvidenceFreshness, FindingProvider,
        FindingProviderError, IntegrationEvent, IntegrationEventPublishError,
        IntegrationEventPublisher, PackageCoordinate, PendingIntegrationEvent, ProviderScanReport,
        ReportedFinding, RunNextScanResult, ScanCommandStatus,
    };

    fn postgres_test_url() -> Option<String> {
        std::env::var("VENOM_TEST_POSTGRES_URL").ok()
    }

    fn temp_schema(name: &str) -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("venom_{name}_{nanos}_{counter}")
    }

    fn artifact() -> ArtifactRef {
        ArtifactRef::new(
            ArtifactKind::ContainerImage,
            "registry.example/payments@sha256:111",
        )
    }

    #[derive(Debug, Clone)]
    struct FixtureProvider;

    impl FindingProvider for FixtureProvider {
        fn provider_key(&self) -> &'static str {
            "fixture-provider"
        }

        async fn scan<'a>(
            &'a self,
            request: &'a venom_domain::ScanRequest,
        ) -> Result<ProviderScanReport, FindingProviderError> {
            Ok(ProviderScanReport::new(
                "fixture-provider",
                request.component_key.clone(),
                request.artifact.clone(),
                SystemTime::UNIX_EPOCH,
                request.freshness,
                vec![ReportedFinding::new(
                    "CVE-2026-0001",
                    PackageCoordinate::new("openssl", "3.0.0"),
                )],
            )
            .with_knowledge_revision("fixture-db:2026-05-16"))
        }
    }

    #[derive(Debug, Clone)]
    struct SuccessPublisher;

    impl IntegrationEventPublisher for SuccessPublisher {
        fn publisher_key(&self) -> &'static str {
            "fixture-publisher"
        }

        async fn publish<'a>(
            &'a self,
            _event: &'a PendingIntegrationEvent,
        ) -> Result<(), IntegrationEventPublishError> {
            Ok(())
        }
    }

    #[derive(Debug, Clone)]
    struct FailingPublisher;

    impl IntegrationEventPublisher for FailingPublisher {
        fn publisher_key(&self) -> &'static str {
            "fixture-publisher"
        }

        async fn publish<'a>(
            &'a self,
            _event: &'a PendingIntegrationEvent,
        ) -> Result<(), IntegrationEventPublishError> {
            Err(IntegrationEventPublishError::new(
                true,
                "publisher unavailable",
            ))
        }
    }

    #[tokio::test]
    async fn postgres_record_scan_report_appends_pending_integration_event() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("outbox_report");
        let mut backend = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should open");
        let _ = backend
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .await
            .expect("registration should persist");
        let _ = backend
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");
        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )],
        )
        .with_knowledge_revision("fixture-db:2026-05-16");

        let _ = backend
            .record_scan_report(&report)
            .await
            .expect("provider report should persist");
        assert_eq!(backend.pending_integration_events().len(), 1);
        assert!(matches!(
            backend.pending_integration_events()[0].event,
            IntegrationEvent::FindingChangesObserved { .. }
        ));

        let reopened = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(reopened.pending_integration_events().len(), 1);
    }

    #[tokio::test]
    async fn postgres_completed_scan_command_appends_pending_integration_event() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("outbox_command");
        let mut backend = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should open");
        let _ = backend
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .await
            .expect("registration should persist");
        let _ = backend
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");
        let _ = backend
            .configure_provider("component:payments-api", "fixture-provider")
            .await
            .expect("provider config should persist");
        let command_id = backend
            .request_scan(
                "component:payments-api",
                artifact(),
                EvidenceFreshness::Deterministic,
            )
            .await
            .expect("scan request should persist");

        let outcome = backend
            .run_next(&FixtureProvider)
            .await
            .expect("scan command should complete");
        assert!(matches!(outcome, RunNextScanResult::Completed(_)));
        assert_eq!(backend.pending_integration_events().len(), 1);
        assert!(matches!(
            backend.pending_integration_events()[0].event,
            IntegrationEvent::ScanCommandCompleted { .. }
        ));

        let reopened = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(
            reopened.command_status(command_id.as_ref()),
            Some(ScanCommandStatus::Completed)
        );
        assert_eq!(reopened.pending_integration_events().len(), 1);
    }

    #[tokio::test]
    async fn postgres_successful_publication_removes_pending_integration_event() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish_success");
        let mut backend = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should open");
        let _ = backend
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .await
            .expect("registration should persist");
        let _ = backend
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");
        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )],
        )
        .with_knowledge_revision("fixture-db:2026-05-16");
        let _ = backend
            .record_scan_report(&report)
            .await
            .expect("provider report should persist");

        let result = backend
            .publish_pending_integration_events(1, &SuccessPublisher)
            .await
            .expect("publication should persist");
        assert_eq!(result.published, 1);
        assert_eq!(backend.pending_integration_events().len(), 0);

        let reopened = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(reopened.pending_integration_events().len(), 0);
    }

    #[tokio::test]
    async fn postgres_failed_publication_keeps_pending_integration_event() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish_failure");
        let mut backend = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should open");
        let _ = backend
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .await
            .expect("registration should persist");
        let _ = backend
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");
        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )],
        )
        .with_knowledge_revision("fixture-db:2026-05-16");
        let _ = backend
            .record_scan_report(&report)
            .await
            .expect("provider report should persist");

        let result = backend
            .publish_pending_integration_events(1, &FailingPublisher)
            .await
            .expect("failed publication outcome should persist");
        assert_eq!(result.published, 0);
        assert_eq!(backend.pending_integration_events().len(), 1);

        let reopened = PostgresBackend::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(reopened.pending_integration_events().len(), 1);
    }
}
