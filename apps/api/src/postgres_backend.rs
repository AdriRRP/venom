use sqlx::{FromRow, PgPool, postgres::PgPoolOptions, types::Json};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use venom_domain::{
    ArtifactKind, ArtifactRef, BindArtifactChange, BindArtifactResult, CompletedScanCommand,
    ComponentRegistration, EvidenceFreshness, FailedScanCommand, FindingChangeSet,
    FindingIngestion, FindingProvider, FindingProviderError, FindingProviderErrorKind,
    FindingReadModel, ProviderScanReport, RegisterComponentChange, RegisterComponentResult,
    ReportedFinding, RunNextScanResult, ScanCommandStatus, ScanPlanner, ScanRequest,
    as_provider_error, validate_provider_scan_report,
};

#[derive(Debug)]
pub struct PostgresBackend {
    pool: PgPool,
    names: TableNames,
    ingestion: FindingIngestion,
    read_model: FindingReadModel,
    commands: BTreeMap<Box<str>, ScanCommandRecord>,
    order: Vec<Box<str>>,
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
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres provider report insert failed: {error}"))?;

        self.ingestion = candidate_ingestion;
        self.read_model = candidate_read_model;
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

    async fn rebuild(&mut self) -> Result<(), String> {
        self.ingestion = FindingIngestion::new();
        self.read_model = FindingReadModel::new();
        self.commands.clear();
        self.order.clear();

        self.load_components().await?;
        self.load_artifact_bindings().await?;
        self.load_provider_reports().await?;
        self.load_scan_commands().await?;

        Ok(())
    }

    async fn load_components(&mut self) -> Result<(), String> {
        let components = sqlx::query_as::<_, ComponentRow>(&format!(
            "SELECT component_key, name FROM {} ORDER BY created_at, component_key",
            self.names.components
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres components load failed: {error}"))?;
        for row in components {
            let result = self
                .ingestion
                .inventory_mut()
                .register(ComponentRegistration::new(row.component_key, row.name));
            if result.change == RegisterComponentChange::Rejected {
                return Err("postgres components contain conflicting registration".to_owned());
            }
        }

        Ok(())
    }

    async fn load_artifact_bindings(&mut self) -> Result<(), String> {
        let bindings = sqlx::query_as::<_, ArtifactBindingRow>(&format!(
            concat!(
                "SELECT component_key, artifact_kind, artifact_identity ",
                "FROM {} ORDER BY created_at, component_key, artifact_kind, artifact_identity"
            ),
            self.names.artifact_bindings
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres artifact bindings load failed: {error}"))?;
        for row in bindings {
            let result = self.ingestion.inventory_mut().bind_artifact(
                row.component_key.as_ref(),
                ArtifactRef::new(
                    parse_artifact_kind(&row.artifact_kind)?,
                    row.artifact_identity,
                ),
            );
            if result.change == BindArtifactChange::Rejected {
                return Err("postgres artifact bindings contain conflicting ownership".to_owned());
            }
        }

        Ok(())
    }

    async fn load_provider_reports(&mut self) -> Result<(), String> {
        let reports = sqlx::query_as::<_, ProviderReportRow>(&format!(
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
        for row in reports {
            let report = ProviderScanReport {
                provider_key: row.provider_key.into_boxed_str(),
                component_key: row.component_key.into_boxed_str(),
                artifact: ArtifactRef::new(
                    parse_artifact_kind(&row.artifact_kind)?,
                    row.artifact_identity,
                ),
                observed_at: micros_to_system_time(row.observed_at_micros)?,
                freshness: parse_freshness(&row.freshness)?,
                knowledge_revision: row.knowledge_revision.map(String::into_boxed_str),
                findings: row.findings.0,
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

    async fn load_scan_commands(&mut self) -> Result<(), String> {
        let commands = sqlx::query_as::<_, ScanCommandRow>(&format!(
            concat!(
                "SELECT command_id, component_key, artifact_kind, artifact_identity, freshness, status ",
                "FROM {} ORDER BY order_id"
            ),
            self.names.scan_commands
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres scan commands load failed: {error}"))?;
        for row in commands {
            let command_id = row.command_id.into_boxed_str();
            let request = ScanRequest::new(
                row.component_key,
                ArtifactRef::new(
                    parse_artifact_kind(&row.artifact_kind)?,
                    row.artifact_identity,
                ),
                parse_freshness(&row.freshness)?,
            );
            self.order.push(command_id.clone());
            self.commands.insert(
                command_id,
                ScanCommandRecord {
                    request,
                    status: parse_scan_command_status(&row.status)?,
                },
            );
        }

        Ok(())
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
    provider_reports: Box<str>,
    scan_commands: Box<str>,
}

impl TableNames {
    fn new(schema: &str) -> Result<Self, String> {
        let schema = validate_schema_name(schema)?;
        Ok(Self {
            components: format!("{schema}.components").into_boxed_str(),
            artifact_bindings: format!("{schema}.artifact_bindings").into_boxed_str(),
            provider_reports: format!("{schema}.provider_reports").into_boxed_str(),
            scan_commands: format!("{schema}.scan_commands").into_boxed_str(),
            schema,
        })
    }
}

#[derive(Debug, FromRow)]
struct ComponentRow {
    component_key: String,
    name: String,
}

#[derive(Debug, FromRow)]
struct ArtifactBindingRow {
    component_key: String,
    artifact_kind: String,
    artifact_identity: String,
}

#[derive(Debug, FromRow)]
struct ProviderReportRow {
    provider_key: String,
    component_key: String,
    artifact_kind: String,
    artifact_identity: String,
    observed_at_micros: i64,
    freshness: String,
    knowledge_revision: Option<String>,
    findings: Json<Vec<ReportedFinding>>,
}

#[derive(Debug, FromRow)]
struct ScanCommandRow {
    command_id: String,
    component_key: String,
    artifact_kind: String,
    artifact_identity: String,
    freshness: String,
    status: String,
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
