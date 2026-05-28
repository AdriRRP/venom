use crate::app::read_cursor::{EventSourceCursor, RowSourceCursor};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction, postgres::PgPoolOptions, types::Json};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use venom_domain::findings::finding_provider_contract::{
    as_provider_error, validate_provider_scan_report,
};
use venom_domain::findings::{
    AcceptRiskChange, AcceptRiskResult, ArtifactKind, ArtifactRef, BulkAcceptRiskResult,
    BulkGovernanceQuery, BulkReopenFindingResult, BulkSuppressFindingResult, EvidenceFreshness,
    FindingChangeSet, FindingDecision, FindingGovernance, FindingIngestion, FindingProvider,
    FindingProviderError, FindingProviderErrorKind, FindingReadModel, FindingRef,
    ProviderScanReport, ReopenFindingChange, ReopenFindingResult, ReportedFinding, RiskAcceptance,
    ScanRequest, SuppressFindingChange, SuppressFindingResult, Suppression,
};
use venom_domain::integration::{
    ConfigureIntegrationRuntimeChange, ConfigureIntegrationRuntimeResult,
    IntegrationEventPublicationFailure, IntegrationEventPublishError, IntegrationEventPublisher,
    IntegrationRuntimeConfig, PendingIntegrationEvent, PublishIntegrationEventsResult,
};
use venom_domain::inventory::{
    AssignCollectionContextProfileChange, AssignCollectionContextProfileResult,
    AssignComponentTagChange, AssignComponentTagResult, AssignContextProfileChange,
    AssignContextProfileResult, AssignTagContextProfileChange, AssignTagContextProfileResult,
    BindArtifactChange, BindArtifactResult, CollectionRegistration, CollectionSource,
    CollectionSourceMode, ComponentInventory, ComponentListCollectionSource, ComponentRegistration,
    ComponentTagRegistration, ConfigureCollectionScanScheduleChange,
    ConfigureCollectionScanScheduleResult, ConfigureCollectionSourceChange,
    ConfigureCollectionSourceResult, ConfigureProviderChange, ConfigureProviderResult,
    ContextProfileRegistration, MaterializeCollectionSourceChange,
    MaterializeCollectionSourceResult, RegisterCollectionChange, RegisterCollectionResult,
    RegisterComponentChange, RegisterComponentResult, RegisterComponentTagChange,
    RegisterComponentTagResult, RegisterContextProfileChange, RegisterContextProfileResult,
};
use venom_domain::operations::system_event_trace::SystemEventQueryIndex;
use venom_domain::operations::{
    MAX_SYSTEM_EVENTS_LIMIT, SystemEvent, SystemEventKind, SystemEventRecentWindows,
    SystemEventWindowTotals,
};
use venom_domain::scanning::{
    CollectionScanScheduler, CompletedScanCommand, DueCollectionScan, FailedScanCommand,
    RunNextScanResult, ScanCommandStatus, ScanPlanner,
};

#[derive(Debug)]
pub struct PostgresStore {
    pool: PgPool,
    names: TableNames,
    observed_change_watermark: Arc<AtomicU64>,
    ingestion: FindingIngestion,
    governance: FindingGovernance,
    read_model: Arc<FindingReadModel>,
    inventory_snapshot_cache: Arc<ComponentInventory>,
    read_model_snapshot_cache: Arc<FindingReadModel>,
    integration_runtime_config: Option<IntegrationRuntimeConfig>,
    provider_report_row_high_watermark: u64,
    governance_journal_high_watermark: u64,
    commands: Arc<CommandRecordMap>,
    order: Arc<CommandOrder>,
    pending_integration_events: Arc<PendingIntegrationEventList>,
    pending_integration_source_cursor: RowSourceCursor,
    system_event_index_snapshot_cache: Arc<SystemEventQueryIndex>,
    system_event_source_cursor: EventSourceCursor,
    command_statuses_snapshot_cache: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
    command_status_source_cursor: RowSourceCursor,
}

#[derive(Debug, Clone)]
pub struct PostgresRemoteChangeProbe {
    pool: PgPool,
    change_watermark_table: Box<str>,
    observed_change_watermark: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
pub struct PostgresReadSnapshotLoader {
    pool: PgPool,
    names: TableNames,
    change_watermark_table: Box<str>,
}

#[derive(Debug, Clone)]
pub struct PostgresReadSnapshotBase {
    inventory: Arc<ComponentInventory>,
    read_model: Arc<FindingReadModel>,
    read_model_source_watermark: u64,
    governance_source_watermark: u64,
    system_event_index: Arc<SystemEventQueryIndex>,
    system_event_source_cursor: EventSourceCursor,
    command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
    command_status_source_cursor: RowSourceCursor,
}

#[derive(Debug)]
pub struct LoadedPostgresReadSnapshot {
    pub inventory: Arc<ComponentInventory>,
    pub read_model: Arc<FindingReadModel>,
    pub read_model_source_watermark: u64,
    pub governance_source_watermark: u64,
    pub system_event_index: Arc<SystemEventQueryIndex>,
    pub system_event_source_cursor: EventSourceCursor,
    pub command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
    pub command_status_source_cursor: RowSourceCursor,
    pub change_watermark: u64,
}

#[derive(Debug, Clone)]
struct TailRefreshCursors {
    command_status: RowSourceCursor,
    system_event: EventSourceCursor,
}

impl PostgresReadSnapshotBase {
    #[must_use]
    pub const fn new(
        inventory: Arc<ComponentInventory>,
        read_model: Arc<FindingReadModel>,
        read_model_source_watermark: u64,
        governance_source_watermark: u64,
        system_event_index: Arc<SystemEventQueryIndex>,
        system_event_source_cursor: EventSourceCursor,
        command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
        command_status_source_cursor: RowSourceCursor,
    ) -> Self {
        Self {
            inventory,
            read_model,
            read_model_source_watermark,
            governance_source_watermark,
            system_event_index,
            system_event_source_cursor,
            command_statuses,
            command_status_source_cursor,
        }
    }
}

const CHANGE_LANE_INVENTORY: i32 = 1;
const CHANGE_LANE_COMPONENT_BINDINGS: i32 = 1 << 1;
const CHANGE_LANE_COLLECTIONS: i32 = 1 << 2;
const CHANGE_LANE_READ_MODEL: i32 = 1 << 3;
const CHANGE_LANE_GOVERNANCE: i32 = 1 << 4;
const CHANGE_LANE_COMMAND_STATUSES: i32 = 1 << 5;
const CHANGE_LANE_SYSTEM_EVENTS: i32 = 1 << 6;
const CHANGE_LANE_INTEGRATION_OUTBOX: i32 = 1 << 7;
const CHANGE_LANE_INTEGRATION_RUNTIME: i32 = 1 << 8;
const CHANGE_LANE_COLLECTION_SCHEDULES: i32 = 1 << 9;
const CHANGE_LANE_PROVIDER_RUNTIME_CONFIGS: i32 = 1 << 10;
const POSTGRES_POOL_MAX_CONNECTIONS: u32 = 1;
const CHANGE_LANE_ALL: i32 = CHANGE_LANE_INVENTORY
    | CHANGE_LANE_COMPONENT_BINDINGS
    | CHANGE_LANE_COLLECTIONS
    | CHANGE_LANE_READ_MODEL
    | CHANGE_LANE_GOVERNANCE
    | CHANGE_LANE_COMMAND_STATUSES
    | CHANGE_LANE_SYSTEM_EVENTS
    | CHANGE_LANE_INTEGRATION_OUTBOX
    | CHANGE_LANE_INTEGRATION_RUNTIME
    | CHANGE_LANE_COLLECTION_SCHEDULES
    | CHANGE_LANE_PROVIDER_RUNTIME_CONFIGS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrainDueCollectionScansResult {
    pub outcome: Box<str>,
    pub processed_collections: usize,
    pub enqueued_commands: usize,
    pub pending_due_remaining: usize,
    pub last_collection_key: Option<Box<str>>,
    pub partial_progress: bool,
    pub last_error: Option<Box<str>>,
}

type DueCollectionScanRows = Vec<(Box<str>, venom_domain::CollectionScanSchedule)>;
type CommandRecordMap = BTreeMap<Box<str>, ScanCommandRecord>;
type CommandOrder = Vec<Box<str>>;
type PendingIntegrationEventList = Vec<PendingIntegrationEvent>;
type SystemEventRow = (
    String,
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<bool>,
    Option<String>,
);
type SystemEventWindowRow = (
    String,
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<bool>,
    Option<String>,
    i64,
    i64,
);
type SystemEventDeltaRow = (
    String,
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<bool>,
    Option<String>,
    i64,
);
type CommandStatusDeltaRow = (String, String, i64);
type GovernanceJournalRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
);

#[derive(Default)]
struct SystemEventTotals {
    total: usize,
    scheduler_total: usize,
    command_total: usize,
    governance_total: usize,
    publication_total: usize,
}

impl PostgresStore {
    /// Connect one shared Postgres pool for VENOM durable operations.
    ///
    /// # Errors
    ///
    /// Returns an error string when Postgres cannot be reached.
    pub async fn connect_pool(database_url: &str) -> Result<PgPool, String> {
        PgPoolOptions::new()
            .max_connections(POSTGRES_POOL_MAX_CONNECTIONS)
            .connect(database_url)
            .await
            .map_err(|error| format!("postgres connect failed: {error}"))
    }

    /// Open or create the Postgres durable backend and rebuild in-memory state.
    ///
    /// # Errors
    ///
    /// Returns an error string when Postgres cannot be reached, initialized, or replayed.
    #[cfg(test)]
    pub async fn open(database_url: &str, schema: &str) -> Result<Self, String> {
        let pool = Self::connect_pool(database_url).await?;
        Self::open_with_pool(pool, schema).await
    }

    /// Open or create the Postgres durable backend over one shared pool.
    ///
    /// # Errors
    ///
    /// Returns an error string when Postgres cannot be initialized or replayed.
    pub async fn open_with_pool(pool: PgPool, schema: &str) -> Result<Self, String> {
        let names = TableNames::new(schema)?;
        let mut backend = Self {
            pool,
            names,
            observed_change_watermark: Arc::new(AtomicU64::new(0)),
            ingestion: FindingIngestion::new(),
            governance: FindingGovernance::new(),
            read_model: Arc::new(FindingReadModel::new()),
            inventory_snapshot_cache: Arc::new(ComponentInventory::default()),
            read_model_snapshot_cache: Arc::new(FindingReadModel::new()),
            integration_runtime_config: None,
            provider_report_row_high_watermark: 0,
            governance_journal_high_watermark: 0,
            commands: Arc::new(BTreeMap::new()),
            order: Arc::new(Vec::new()),
            pending_integration_events: Arc::new(Vec::new()),
            pending_integration_source_cursor: RowSourceCursor::default(),
            system_event_index_snapshot_cache: Arc::new(SystemEventQueryIndex::new()),
            system_event_source_cursor: EventSourceCursor::default(),
            command_statuses_snapshot_cache: Arc::new(BTreeMap::new()),
            command_status_source_cursor: RowSourceCursor::default(),
        };
        backend.init_schema().await?;
        backend.rebuild().await?;
        Ok(backend)
    }

    #[must_use]
    pub fn fork_from(base: &Self) -> Self {
        Self {
            pool: base.pool.clone(),
            names: base.names.clone(),
            observed_change_watermark: Arc::new(AtomicU64::new(base.observed_change_watermark())),
            ingestion: base.ingestion.clone(),
            governance: base.governance.clone(),
            read_model: Arc::clone(&base.read_model),
            inventory_snapshot_cache: Arc::clone(&base.inventory_snapshot_cache),
            read_model_snapshot_cache: Arc::clone(&base.read_model_snapshot_cache),
            integration_runtime_config: base.integration_runtime_config.clone(),
            provider_report_row_high_watermark: base.provider_report_row_high_watermark,
            governance_journal_high_watermark: base.governance_journal_high_watermark,
            commands: Arc::clone(&base.commands),
            order: Arc::clone(&base.order),
            pending_integration_events: Arc::clone(&base.pending_integration_events),
            pending_integration_source_cursor: base.pending_integration_source_cursor.clone(),
            system_event_index_snapshot_cache: Arc::clone(&base.system_event_index_snapshot_cache),
            system_event_source_cursor: base.system_event_source_cursor.clone(),
            command_statuses_snapshot_cache: Arc::clone(&base.command_statuses_snapshot_cache),
            command_status_source_cursor: base.command_status_source_cursor.clone(),
        }
    }

    fn detached(pool: PgPool, names: TableNames) -> Self {
        Self {
            pool,
            names,
            observed_change_watermark: Arc::new(AtomicU64::new(0)),
            ingestion: FindingIngestion::new(),
            governance: FindingGovernance::new(),
            read_model: Arc::new(FindingReadModel::new()),
            inventory_snapshot_cache: Arc::new(ComponentInventory::default()),
            read_model_snapshot_cache: Arc::new(FindingReadModel::new()),
            integration_runtime_config: None,
            provider_report_row_high_watermark: 0,
            governance_journal_high_watermark: 0,
            commands: Arc::new(BTreeMap::new()),
            order: Arc::new(Vec::new()),
            pending_integration_events: Arc::new(Vec::new()),
            pending_integration_source_cursor: RowSourceCursor::default(),
            system_event_index_snapshot_cache: Arc::new(SystemEventQueryIndex::new()),
            system_event_source_cursor: EventSourceCursor::default(),
            command_statuses_snapshot_cache: Arc::new(BTreeMap::new()),
            command_status_source_cursor: RowSourceCursor::default(),
        }
    }

    /// Refresh one Postgres-backed in-memory view when another writer advanced the durable store.
    ///
    /// # Errors
    ///
    /// Returns an error string when the schema-local change watermark cannot be
    /// be read or rebuild fails.
    pub async fn refresh_from_remote_if_stale(&mut self) -> Result<bool, String> {
        let current_change_watermark = self.current_change_watermark().await?;
        if self.observed_change_watermark() == current_change_watermark {
            return Ok(false);
        }
        let lane_mask = self
            .changed_lane_mask(self.observed_change_watermark(), current_change_watermark)
            .await?;
        if lane_mask & CHANGE_LANE_INVENTORY != 0 {
            self.refresh_inventory_core_from_remote().await?;
        }
        if lane_mask & CHANGE_LANE_COMPONENT_BINDINGS != 0 {
            self.refresh_component_bindings_from_remote().await?;
        }
        if lane_mask & CHANGE_LANE_COLLECTIONS != 0 {
            self.refresh_collections_from_remote().await?;
        }
        if lane_mask & CHANGE_LANE_COLLECTION_SCHEDULES != 0 {
            self.refresh_collection_scan_schedules_from_remote().await?;
        }
        if lane_mask & CHANGE_LANE_PROVIDER_RUNTIME_CONFIGS != 0 {
            self.refresh_provider_runtime_configs_from_remote().await?;
        }
        if lane_mask & (CHANGE_LANE_READ_MODEL | CHANGE_LANE_GOVERNANCE) != 0 {
            self.refresh_read_model_from_remote(lane_mask).await?;
        }
        if lane_mask & CHANGE_LANE_COMMAND_STATUSES != 0 {
            self.refresh_command_statuses_from_remote().await?;
        }
        if lane_mask & CHANGE_LANE_INTEGRATION_OUTBOX != 0 {
            self.refresh_pending_integration_events_from_remote()
                .await?;
        }
        if lane_mask & CHANGE_LANE_INTEGRATION_RUNTIME != 0 {
            self.refresh_integration_runtime_config_from_remote()
                .await?;
        }
        if lane_mask & CHANGE_LANE_SYSTEM_EVENTS != 0 {
            self.refresh_system_events_from_remote().await?;
        }
        self.set_observed_change_watermark(current_change_watermark);
        Ok(true)
    }

    /// Mark the latest Postgres WAL watermark as already observed by this instance.
    ///
    /// # Errors
    ///
    /// Returns an error string when the schema-local change watermark cannot be
    /// read.
    pub async fn mark_remote_change_observed(&self) -> Result<(), String> {
        self.set_observed_change_watermark(self.current_change_watermark().await?);
        Ok(())
    }

    #[must_use]
    pub fn remote_change_probe(&self) -> PostgresRemoteChangeProbe {
        PostgresRemoteChangeProbe {
            pool: self.pool.clone(),
            change_watermark_table: self.names.change_watermark.clone(),
            observed_change_watermark: Arc::clone(&self.observed_change_watermark),
        }
    }

    #[must_use]
    pub fn read_snapshot_loader(&self) -> PostgresReadSnapshotLoader {
        PostgresReadSnapshotLoader {
            pool: self.pool.clone(),
            names: self.names.clone(),
            change_watermark_table: self.names.change_watermark.clone(),
        }
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
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register(registration.clone());
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
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_inventory_and_release_board_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably register one reusable execution-context profile in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn register_context_profile(
        &mut self,
        registration: ContextProfileRegistration,
    ) -> Result<RegisterContextProfileResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register_context_profile(registration.clone());
        if result.change == RegisterContextProfileChange::Registered {
            sqlx::query(&format!(
                concat!(
                    "INSERT INTO {} (profile_key, name, internet_exposed, production, mission_critical, vpn_restricted, non_privileged_user) ",
                    "VALUES ($1, $2, $3, $4, $5, $6, $7)"
                ),
                self.names.context_profiles
            ))
            .bind(registration.profile_key.as_ref())
            .bind(registration.name.as_ref())
            .bind(registration.internet_exposed)
            .bind(registration.production)
            .bind(registration.mission_critical)
            .bind(registration.vpn_restricted)
            .bind(registration.non_privileged_user)
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres context profile insert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_inventory_and_release_board_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably register one managed component tag in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn register_component_tag(
        &mut self,
        registration: ComponentTagRegistration,
    ) -> Result<RegisterComponentTagResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register_component_tag(registration.clone());
        if result.change == RegisterComponentTagChange::Registered {
            sqlx::query(&format!(
                "INSERT INTO {} (tag_key, name) VALUES ($1, $2)",
                self.names.component_tags
            ))
            .bind(registration.tag_key.as_ref())
            .bind(registration.name.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres component tag insert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably create one closed release collection in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn register_collection(
        &mut self,
        registration: CollectionRegistration,
    ) -> Result<RegisterCollectionResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register_collection(registration.clone());
        if result.change == RegisterCollectionChange::Created {
            sqlx::query(&format!(
                "INSERT INTO {} (collection_key, name) VALUES ($1, $2)",
                self.names.collections
            ))
            .bind(registration.collection_key.as_ref())
            .bind(registration.name.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres collection insert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
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
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.bind_artifact(component_key, artifact.clone());
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
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably add one managed component to one collection in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn add_component_to_collection(
        &mut self,
        collection_key: &str,
        component_key: &str,
    ) -> Result<venom_domain::AddCollectionComponentResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.add_component_to_collection(collection_key, component_key);
        if result.change == venom_domain::AddCollectionComponentChange::Added {
            sqlx::query(&format!(
                concat!("INSERT INTO {} (collection_key, component_key) VALUES ($1, $2)"),
                self.names.collection_memberships
            ))
            .bind(collection_key)
            .bind(component_key)
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres collection membership insert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably add one managed component to one managed component tag in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn assign_component_tag(
        &mut self,
        tag_key: &str,
        component_key: &str,
    ) -> Result<AssignComponentTagResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.assign_component_tag(tag_key, component_key);
        if result.change == AssignComponentTagChange::Assigned {
            sqlx::query(&format!(
                "INSERT INTO {} (tag_key, component_key) VALUES ($1, $2)",
                self.names.component_tag_memberships
            ))
            .bind(tag_key)
            .bind(component_key)
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres component tag membership insert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably remove one managed component from one collection in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn remove_component_from_collection(
        &mut self,
        collection_key: &str,
        component_key: &str,
    ) -> Result<venom_domain::RemoveCollectionComponentResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result =
            candidate_inventory.remove_component_from_collection(collection_key, component_key);
        if result.change == venom_domain::RemoveCollectionComponentChange::Removed {
            sqlx::query(&format!(
                "DELETE FROM {} WHERE collection_key = $1 AND component_key = $2",
                self.names.collection_memberships
            ))
            .bind(collection_key)
            .bind(component_key)
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres collection membership delete failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably configure one declared source for one managed collection in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn configure_collection_source(
        &mut self,
        collection_key: &str,
        source: CollectionSource,
    ) -> Result<ConfigureCollectionSourceResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result =
            candidate_inventory.configure_collection_source(collection_key, source.clone());
        if result.change == ConfigureCollectionSourceChange::Configured {
            sqlx::query(&format!(
                concat!(
                    "INSERT INTO {} (collection_key, source_kind, mode, component_keys) VALUES ($1, $2, $3, $4) ",
                    "ON CONFLICT (collection_key) DO UPDATE SET source_kind = EXCLUDED.source_kind, mode = EXCLUDED.mode, component_keys = EXCLUDED.component_keys, updated_at = NOW()"
                ),
                self.names.collection_sources
            ))
            .bind(collection_key)
            .bind(collection_source_kind_name(&source))
            .bind(collection_source_mode_name(source.mode()))
            .bind(Json(
                source
                    .component_keys()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            ))
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres collection source upsert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably materialize one declared source into collection membership in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn materialize_collection_source(
        &mut self,
        collection_key: &str,
    ) -> Result<MaterializeCollectionSourceResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.materialize_collection_source(collection_key);
        if result.change == MaterializeCollectionSourceChange::Materialized {
            let mut transaction = self.begin_transaction().await?;
            for component_key in &result.removed_component_keys {
                sqlx::query(&format!(
                    "DELETE FROM {} WHERE collection_key = $1 AND component_key = $2",
                    self.names.collection_memberships
                ))
                .bind(collection_key)
                .bind(component_key.as_ref())
                .execute(&mut *transaction)
                .await
                .map_err(|error| {
                    format!("postgres collection source membership delete failed: {error}")
                })?;
            }
            for component_key in &result.added_component_keys {
                sqlx::query(&format!(
                    "INSERT INTO {} (collection_key, component_key) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    self.names.collection_memberships
                ))
                .bind(collection_key)
                .bind(component_key.as_ref())
                .execute(&mut *transaction)
                .await
                .map_err(|error| {
                    format!("postgres collection source membership insert failed: {error}")
                })?;
            }
            self.commit_transaction(transaction).await?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably configure one periodic collection scan schedule in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn configure_collection_scan_schedule(
        &mut self,
        collection_key: &str,
        cadence_minutes: u32,
        freshness: EvidenceFreshness,
        next_due_at_unix_ms: u64,
    ) -> Result<ConfigureCollectionScanScheduleResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.configure_collection_scan_schedule(
            collection_key,
            cadence_minutes,
            freshness,
            next_due_at_unix_ms,
        );
        if result.change == ConfigureCollectionScanScheduleChange::Configured {
            sqlx::query(&format!(
                concat!(
                    "INSERT INTO {} (collection_key, cadence_minutes, freshness, next_due_at_unix_ms) VALUES ($1, $2, $3, $4) ",
                    "ON CONFLICT (collection_key) DO UPDATE SET cadence_minutes = EXCLUDED.cadence_minutes, freshness = EXCLUDED.freshness, next_due_at_unix_ms = EXCLUDED.next_due_at_unix_ms, updated_at = NOW()"
                ),
                self.names.collection_scan_schedules
            ))
            .bind(collection_key)
            .bind(i32::try_from(cadence_minutes).map_err(|_| "cadence_minutes overflow".to_owned())?)
            .bind(freshness_name(freshness))
            .bind(i64::try_from(next_due_at_unix_ms).map_err(|_| "next due overflow".to_owned())?)
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres collection scan schedule upsert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
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
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.configure_provider(component_key, provider_key);
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
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably assign one context profile to one managed component in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn assign_context_profile(
        &mut self,
        component_key: &str,
        profile_key: &str,
    ) -> Result<AssignContextProfileResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.assign_context_profile(component_key, profile_key);
        if result.change == AssignContextProfileChange::Assigned {
            sqlx::query(&format!(
                concat!(
                    "INSERT INTO {} (component_key, profile_key) VALUES ($1, $2) ",
                    "ON CONFLICT (component_key) DO UPDATE SET profile_key = EXCLUDED.profile_key, updated_at = NOW()"
                ),
                self.names.component_context_profiles
            ))
            .bind(component_key)
            .bind(profile_key)
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres component context profile upsert failed: {error}"))?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably assign one context profile across one managed collection in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn assign_context_profile_for_collection(
        &mut self,
        collection_key: &str,
        profile_key: &str,
    ) -> Result<AssignCollectionContextProfileResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result =
            candidate_inventory.assign_context_profile_for_collection(collection_key, profile_key);
        if result.change == AssignCollectionContextProfileChange::Assigned {
            sqlx::query(&format!(
                concat!(
                    "UPDATE {} SET context_profile_key = $2, updated_at = NOW() ",
                    "WHERE collection_key = $1"
                ),
                self.names.collections
            ))
            .bind(collection_key)
            .bind(profile_key)
            .execute(&self.pool)
            .await
            .map_err(|error| {
                format!("postgres collection context profile update failed: {error}")
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
            self.refresh_read_snapshot_caches();
        }
        Ok(result)
    }

    /// Durably assign one context profile to one managed component tag in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn assign_context_profile_for_tag(
        &mut self,
        tag_key: &str,
        profile_key: &str,
    ) -> Result<AssignTagContextProfileResult, String> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.assign_context_profile_for_tag(tag_key, profile_key);
        if result.change == AssignTagContextProfileChange::Assigned {
            sqlx::query(&format!(
                concat!(
                    "UPDATE {} SET context_profile_key = $2, updated_at = NOW() ",
                    "WHERE tag_key = $1"
                ),
                self.names.component_tags
            ))
            .bind(tag_key)
            .bind(profile_key)
            .execute(&self.pool)
            .await
            .map_err(|error| {
                format!("postgres component tag context profile update failed: {error}")
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably configure the system integration publication runtime in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn configure_integration_runtime(
        &mut self,
        config: IntegrationRuntimeConfig,
    ) -> Result<ConfigureIntegrationRuntimeResult, String> {
        let change = if self.integration_runtime_config.as_ref() == Some(&config) {
            ConfigureIntegrationRuntimeChange::Unchanged
        } else {
            sqlx::query(&format!(
                concat!(
                    "INSERT INTO {} (id, publisher_key, endpoint_url, timeout_ms) VALUES (1, $1, $2, $3) ",
                    "ON CONFLICT (id) DO UPDATE SET publisher_key = EXCLUDED.publisher_key, endpoint_url = EXCLUDED.endpoint_url, timeout_ms = EXCLUDED.timeout_ms, updated_at = NOW()"
                ),
                self.names.integration_runtime_config
            ))
            .bind(config.publisher_key())
            .bind(config.endpoint_url())
            .bind(config.timeout_ms().map(i64::from))
            .execute(&self.pool)
            .await
            .map_err(|error| format!("postgres integration runtime config upsert failed: {error}"))?;
            self.integration_runtime_config = Some(config.clone());
            ConfigureIntegrationRuntimeChange::Configured
        };
        Ok(ConfigureIntegrationRuntimeResult { change, config })
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
        let mut candidate_read_model = self.read_model_arc();
        let change_set = candidate_ingestion
            .record_scan_report(report)
            .map_err(|error| format!("provider report cannot be applied: {}", error.as_str()))?;
        Arc::make_mut(&mut candidate_read_model).record_scan_report(report);
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

        let provider_report_row_id = sqlx::query_scalar::<_, i64>(&format!(
            concat!(
                "INSERT INTO {} ",
                "(provider_key, component_key, artifact_kind, artifact_identity, observed_at_micros, freshness, knowledge_revision, findings) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id"
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
        .fetch_one(&mut *transaction)
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
        self.provider_report_row_high_watermark = self.provider_report_row_high_watermark.max(
            u64::try_from(provider_report_row_id)
                .map_err(|_| "postgres provider report id out of range".to_owned())?,
        );
        self.refresh_read_model_and_release_board_snapshot_caches();
        Arc::make_mut(&mut self.pending_integration_events).push(pending_integration_event);
        Ok(change_set)
    }

    /// Durably accept the risk of one currently active finding in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the finding is not active or the durable
    /// write fails.
    pub async fn accept_risk(
        &mut self,
        finding: FindingRef,
        acceptance: RiskAcceptance,
    ) -> Result<AcceptRiskResult, String> {
        if !self.read_model.has_active_finding(&finding) {
            return Err("cannot accept risk for an inactive finding".to_owned());
        }

        let mut candidate_governance = self.governance.clone();
        let mut candidate_read_model = self.read_model_arc();
        let result = candidate_governance.accept_risk(finding.clone(), acceptance.clone());
        if result.change == AcceptRiskChange::Accepted {
            let component_key = finding.component_key.clone();
            let reason = acceptance.reason.clone();
            let occurred_at_unix_ms = current_unix_millis()?;
            let event = SystemEvent {
                event_id: next_system_event_id("finding-risk-accepted"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingRiskAccepted,
                collection_key: None,
                component_key: Some(component_key),
                command_id: None,
                integration_event_id: None,
                finding_count: Some(1),
                retryable: None,
                detail: Some(reason),
            };
            let mut tx = self.pool.begin().await.map_err(|error| {
                format!("postgres finding risk acceptance begin failed: {error}")
            })?;
            self.upsert_risk_acceptance_in_transaction(&mut tx, &finding, &acceptance)
                .await?;
            self.append_governance_journal_entry_in_transaction(
                &mut tx,
                &finding,
                "risk-accepted",
                Some(acceptance.reason.as_ref()),
                acceptance.until_unix_ms,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres finding risk acceptance commit failed: {error}")
            })?;

            Arc::make_mut(&mut candidate_read_model).accept_risk(finding, acceptance);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(result)
    }

    /// Durably accept risk for all matched open findings inside one managed collection.
    ///
    /// # Errors
    ///
    /// Returns an error string when the collection is unknown or the durable
    /// write fails.
    pub async fn accept_risk_for_collection(
        &mut self,
        collection_key: &str,
        query: &BulkGovernanceQuery,
        acceptance: RiskAcceptance,
    ) -> Result<BulkAcceptRiskResult, String> {
        let scope = self
            .ingestion
            .inventory()
            .collection_scoped_artifacts(collection_key)
            .ok_or_else(|| format!("unknown collection: {collection_key}"))?;
        let mut changed = Vec::new();
        let targeted = self.read_model.visit_bulk_governance_finding_refs_matching(
            &scope,
            query,
            |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::RiskAccepted(existing)) if existing == &acceptance
                )
            },
            |finding| changed.push(finding),
        );

        let accepted = changed.len();
        if accepted > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            let mut tx =
                self.pool.begin().await.map_err(|error| {
                    format!("postgres risk acceptance batch begin failed: {error}")
                })?;

            let event = SystemEvent {
                event_id: next_system_event_id("findings-risk-accepted"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsRiskAccepted,
                collection_key: Some(collection_key.into()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(accepted).ok(),
                retryable: None,
                detail: Some(acceptance.reason.clone()),
            };
            self.upsert_risk_acceptances_in_transaction(&mut tx, &changed, &acceptance)
                .await?;
            self.append_governance_journal_entries_in_transaction(
                &mut tx,
                &changed,
                "risk-accepted",
                Some(acceptance.reason.as_ref()),
                acceptance.until_unix_ms,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres risk acceptance batch commit failed: {error}")
            })?;

            for finding in &changed {
                self.governance
                    .accept_risk(finding.clone(), acceptance.clone());
                self.read_model_mut()
                    .accept_risk(finding.clone(), acceptance.clone());
            }
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(BulkAcceptRiskResult {
            targeted,
            accepted,
            unchanged: targeted.saturating_sub(accepted),
            acceptance,
        })
    }

    /// Durably accept risk for all matched open findings inside one managed tag.
    ///
    /// # Errors
    ///
    /// Returns an error string when the tag is unknown or the durable write fails.
    pub async fn accept_risk_for_tag(
        &mut self,
        tag_key: &str,
        query: &BulkGovernanceQuery,
        acceptance: RiskAcceptance,
    ) -> Result<BulkAcceptRiskResult, String> {
        let scope = self
            .ingestion
            .inventory()
            .tag_scoped_artifacts(tag_key)
            .ok_or_else(|| format!("unknown tag: {tag_key}"))?;
        let mut changed = Vec::new();
        let targeted = self.read_model.visit_bulk_governance_finding_refs_matching(
            &scope,
            query,
            |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::RiskAccepted(existing)) if existing == &acceptance
                )
            },
            |finding| changed.push(finding),
        );

        let accepted = changed.len();
        if accepted > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            let mut tx = self.pool.begin().await.map_err(|error| {
                format!("postgres tag risk acceptance batch begin failed: {error}")
            })?;

            let event = SystemEvent {
                event_id: next_system_event_id("tag-findings-risk-accepted"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsRiskAccepted,
                collection_key: None,
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(accepted).ok(),
                retryable: None,
                detail: Some(format!("tag {tag_key}: {}", acceptance.reason).into_boxed_str()),
            };
            self.upsert_risk_acceptances_in_transaction(&mut tx, &changed, &acceptance)
                .await?;
            self.append_governance_journal_entries_in_transaction(
                &mut tx,
                &changed,
                "risk-accepted",
                Some(acceptance.reason.as_ref()),
                acceptance.until_unix_ms,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres tag risk acceptance batch commit failed: {error}")
            })?;

            for finding in &changed {
                self.governance
                    .accept_risk(finding.clone(), acceptance.clone());
                self.read_model_mut()
                    .accept_risk(finding.clone(), acceptance.clone());
            }
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(BulkAcceptRiskResult {
            targeted,
            accepted,
            unchanged: targeted.saturating_sub(accepted),
            acceptance,
        })
    }

    /// Durably reopen one governed active finding in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the finding is not active or the durable
    /// write fails.
    pub async fn reopen_finding(
        &mut self,
        finding: FindingRef,
    ) -> Result<ReopenFindingResult, String> {
        if !self.read_model.has_active_finding(&finding) {
            return Err("cannot reopen an inactive finding".to_owned());
        }

        let mut candidate_governance = self.governance.clone();
        let mut candidate_read_model = self.read_model_arc();
        let result = candidate_governance.reopen(&finding);
        if result.change == ReopenFindingChange::Reopened {
            let component_key = finding.component_key.clone();
            let occurred_at_unix_ms = current_unix_millis()?;
            let event = SystemEvent {
                event_id: next_system_event_id("finding-reopened"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingReopened,
                collection_key: None,
                component_key: Some(component_key),
                command_id: None,
                integration_event_id: None,
                finding_count: Some(1),
                retryable: None,
                detail: None,
            };
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|error| format!("postgres finding reopen begin failed: {error}"))?;
            self.delete_governance_decision_rows_in_transaction(&mut tx, &finding)
                .await?;
            self.append_governance_journal_entry_in_transaction(
                &mut tx,
                &finding,
                "reopened",
                None,
                None,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres finding reopen commit failed: {error}"))?;

            Arc::make_mut(&mut candidate_read_model).reopen(&finding);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(result)
    }

    /// Durably suppress one currently active finding in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the finding is not active or the durable
    /// write fails.
    pub async fn suppress_finding(
        &mut self,
        finding: FindingRef,
        suppression: Suppression,
    ) -> Result<SuppressFindingResult, String> {
        if !self.read_model.has_active_finding(&finding) {
            return Err("cannot suppress an inactive finding".to_owned());
        }

        let mut candidate_governance = self.governance.clone();
        let mut candidate_read_model = self.read_model_arc();
        let result = candidate_governance.suppress(finding.clone(), suppression.clone());
        if result.change == SuppressFindingChange::Suppressed {
            let component_key = finding.component_key.clone();
            let reason = suppression.reason.clone();
            let occurred_at_unix_ms = current_unix_millis()?;
            let event = SystemEvent {
                event_id: next_system_event_id("finding-suppressed"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingSuppressed,
                collection_key: None,
                component_key: Some(component_key),
                command_id: None,
                integration_event_id: None,
                finding_count: Some(1),
                retryable: None,
                detail: Some(reason),
            };
            let mut tx =
                self.pool.begin().await.map_err(|error| {
                    format!("postgres finding suppression begin failed: {error}")
                })?;
            self.upsert_suppression_in_transaction(&mut tx, &finding, &suppression)
                .await?;
            self.append_governance_journal_entry_in_transaction(
                &mut tx,
                &finding,
                "suppressed",
                Some(suppression.reason.as_ref()),
                None,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres finding suppression commit failed: {error}"))?;

            Arc::make_mut(&mut candidate_read_model).suppress(finding, suppression);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(result)
    }

    /// Durably suppress one filtered open cohort of findings in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the collection is unknown or the durable
    /// write fails.
    pub async fn suppress_findings_for_collection(
        &mut self,
        collection_key: &str,
        query: &BulkGovernanceQuery,
        suppression: Suppression,
    ) -> Result<BulkSuppressFindingResult, String> {
        let scope = self
            .ingestion
            .inventory()
            .collection_scoped_artifacts(collection_key)
            .ok_or_else(|| format!("unknown collection: {collection_key}"))?;
        let mut changed_findings = Vec::new();
        let targeted = self.read_model.visit_bulk_governance_finding_refs_matching(
            &scope,
            query,
            |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::Suppressed(existing)) if existing == &suppression
                )
            },
            |finding| changed_findings.push(finding),
        );

        let suppressed = changed_findings.len();
        if suppressed > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|error| format!("postgres suppression batch begin failed: {error}"))?;

            let event = SystemEvent {
                event_id: next_system_event_id("findings-suppressed"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsSuppressed,
                collection_key: Some(collection_key.into()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(suppressed).ok(),
                retryable: None,
                detail: Some(suppression.reason.clone()),
            };
            self.upsert_suppressions_in_transaction(&mut tx, &changed_findings, &suppression)
                .await?;
            self.append_governance_journal_entries_in_transaction(
                &mut tx,
                &changed_findings,
                "suppressed",
                Some(suppression.reason.as_ref()),
                None,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres suppression batch commit failed: {error}"))?;

            for finding in &changed_findings {
                self.governance
                    .suppress(finding.clone(), suppression.clone());
                self.read_model_mut()
                    .suppress(finding.clone(), suppression.clone());
            }
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(BulkSuppressFindingResult {
            targeted,
            suppressed,
            unchanged: targeted.saturating_sub(suppressed),
            suppression,
        })
    }

    /// Durably suppress all matched open findings inside one managed tag.
    ///
    /// # Errors
    ///
    /// Returns an error string when the tag is unknown or the durable write fails.
    pub async fn suppress_findings_for_tag(
        &mut self,
        tag_key: &str,
        query: &BulkGovernanceQuery,
        suppression: Suppression,
    ) -> Result<BulkSuppressFindingResult, String> {
        let scope = self
            .ingestion
            .inventory()
            .tag_scoped_artifacts(tag_key)
            .ok_or_else(|| format!("unknown tag: {tag_key}"))?;
        let mut changed = Vec::new();
        let targeted = self.read_model.visit_bulk_governance_finding_refs_matching(
            &scope,
            query,
            |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::Suppressed(existing)) if existing == &suppression
                )
            },
            |finding| changed.push(finding),
        );

        let suppressed = changed.len();
        if suppressed > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            let mut tx =
                self.pool.begin().await.map_err(|error| {
                    format!("postgres tag suppression batch begin failed: {error}")
                })?;

            let event = SystemEvent {
                event_id: next_system_event_id("tag-findings-suppressed"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsSuppressed,
                collection_key: None,
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(suppressed).ok(),
                retryable: None,
                detail: Some(format!("tag {tag_key}: {}", suppression.reason).into_boxed_str()),
            };
            self.upsert_suppressions_in_transaction(&mut tx, &changed, &suppression)
                .await?;
            self.append_governance_journal_entries_in_transaction(
                &mut tx,
                &changed,
                "suppressed",
                Some(suppression.reason.as_ref()),
                None,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres tag suppression batch commit failed: {error}")
            })?;

            for finding in &changed {
                self.governance
                    .suppress(finding.clone(), suppression.clone());
                self.read_model_mut()
                    .suppress(finding.clone(), suppression.clone());
            }
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(BulkSuppressFindingResult {
            targeted,
            suppressed,
            unchanged: targeted.saturating_sub(suppressed),
            suppression,
        })
    }

    /// Durably reopen one filtered governed cohort of findings in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the collection is unknown or the durable
    /// write fails.
    pub async fn reopen_findings_for_collection(
        &mut self,
        collection_key: &str,
        query: &BulkGovernanceQuery,
    ) -> Result<BulkReopenFindingResult, String> {
        let scope = self
            .ingestion
            .inventory()
            .collection_scoped_artifacts(collection_key)
            .ok_or_else(|| format!("unknown collection: {collection_key}"))?;
        let mut reopened_findings = Vec::new();
        let targeted = self.read_model.visit_bulk_governance_finding_refs_matching(
            &scope,
            query,
            |finding| self.governance.decision(finding).is_some(),
            |finding| reopened_findings.push(finding),
        );

        let reopened = reopened_findings.len();
        if reopened > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|error| format!("postgres reopen batch begin failed: {error}"))?;

            let event = SystemEvent {
                event_id: next_system_event_id("findings-reopened"),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsReopened,
                collection_key: Some(collection_key.into()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(reopened).ok(),
                retryable: None,
                detail: None,
            };
            for finding in &reopened_findings {
                self.delete_governance_decision_rows_in_transaction(&mut tx, finding)
                    .await?;
            }
            self.append_governance_journal_entries_in_transaction(
                &mut tx,
                &reopened_findings,
                "reopened",
                None,
                None,
            )
            .await?;
            let system_event_cursor = self
                .insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres reopen batch commit failed: {error}"))?;

            for finding in &reopened_findings {
                self.governance.reopen(finding);
                self.read_model_mut().reopen(finding);
            }
            self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
            self.refresh_read_model_and_release_board_snapshot_caches();
            self.system_event_source_cursor =
                max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
            self.push_system_event(event);
        }

        Ok(BulkReopenFindingResult {
            targeted,
            reopened,
            unchanged: targeted.saturating_sub(reopened),
        })
    }

    #[must_use]
    pub fn read_model_snapshot_arc(&self) -> Arc<FindingReadModel> {
        Arc::clone(&self.read_model_snapshot_cache)
    }

    #[must_use]
    pub const fn read_model_source_watermark(&self) -> u64 {
        self.provider_report_row_high_watermark
    }

    #[must_use]
    pub const fn governance_source_watermark(&self) -> u64 {
        self.governance_journal_high_watermark
    }

    #[must_use]
    pub fn read_model_arc(&self) -> Arc<FindingReadModel> {
        Arc::clone(&self.read_model)
    }

    #[cfg(test)]
    pub const fn governance(&self) -> &FindingGovernance {
        &self.governance
    }

    fn read_model_mut(&mut self) -> &mut FindingReadModel {
        Arc::make_mut(&mut self.read_model)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub fn inventory_snapshot(&self) -> ComponentInventory {
        self.ingestion.inventory().clone()
    }

    #[must_use]
    pub fn inventory_snapshot_arc(&self) -> Arc<ComponentInventory> {
        Arc::clone(&self.inventory_snapshot_cache)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub fn system_events_snapshot(&self) -> Vec<SystemEvent> {
        self.system_event_index_snapshot_cache
            .query(
                &venom_domain::operations::SystemEventsQuery::new().with_limit(
                    venom_domain::operations::system_event_trace::MAX_SYSTEM_EVENTS_LIMIT,
                ),
            )
            .events
            .into_iter()
            .map(|event| event.as_ref().clone())
            .collect()
    }

    #[must_use]
    pub fn system_event_index_snapshot_arc(&self) -> Arc<SystemEventQueryIndex> {
        Arc::clone(&self.system_event_index_snapshot_cache)
    }

    #[must_use]
    pub fn system_event_source_cursor(&self) -> EventSourceCursor {
        self.system_event_source_cursor.clone()
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

    #[must_use]
    pub const fn integration_runtime_config(&self) -> Option<&IntegrationRuntimeConfig> {
        self.integration_runtime_config.as_ref()
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
        let occurred_at_unix_ms = current_unix_millis()?;
        let event = SystemEvent {
            event_id: next_system_event_id("scan-command-enqueued"),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandEnqueued,
            collection_key: None,
            component_key: Some(component_key.into()),
            command_id: Some(command_id.clone()),
            integration_event_id: None,
            finding_count: None,
            retryable: None,
            detail: None,
        };

        let mut transaction = self.begin_transaction().await?;
        let command_status_cursor = self
            .insert_pending_scan_commands(
                &mut transaction,
                std::slice::from_ref(&command_id),
                std::slice::from_ref(&request),
            )
            .await
            .map_err(|error| format!("postgres scan command insert failed: {error}"))?;
        let system_event_cursor = self
            .insert_system_events_in_transaction(&mut transaction, std::slice::from_ref(&event))
            .await?;
        self.commit_transaction(transaction).await?;

        Arc::make_mut(&mut self.order).push(command_id.clone());
        Arc::make_mut(&mut self.commands).insert(
            command_id.clone(),
            ScanCommandRecord {
                request,
                status: ScanCommandStatus::Pending,
            },
        );
        self.set_command_status_snapshot(command_id.as_ref(), ScanCommandStatus::Pending);
        self.command_status_source_cursor =
            max_row_source_cursor(&self.command_status_source_cursor, command_status_cursor);
        self.system_event_source_cursor =
            max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
        self.push_system_event(event);
        Ok(command_id)
    }

    /// Durably enqueue one canonical scan batch for one managed collection in Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the collection is unmanaged or the durable write fails.
    pub async fn request_collection_scan(
        &mut self,
        collection_key: &str,
        freshness: EvidenceFreshness,
    ) -> Result<Vec<Box<str>>, String> {
        let batch = ScanPlanner::new(self.ingestion.inventory())
            .plan_collection(collection_key, freshness)
            .map_err(|error| error.as_str().to_owned())?;

        if batch.requests.is_empty() {
            return Ok(Vec::new());
        }

        let command_ids = (0..batch.requests.len())
            .map(|_| next_command_id())
            .collect::<Vec<_>>();
        let occurred_at_unix_ms = current_unix_millis()?;
        let system_events = command_ids
            .iter()
            .cloned()
            .zip(batch.requests.iter())
            .map(|(command_id, request)| SystemEvent {
                event_id: next_system_event_id("scan-command-enqueued"),
                occurred_at_unix_ms,
                kind: SystemEventKind::ScanCommandEnqueued,
                collection_key: Some(collection_key.into()),
                component_key: Some(request.component_key.clone()),
                command_id: Some(command_id),
                integration_event_id: None,
                finding_count: None,
                retryable: None,
                detail: None,
            })
            .collect::<Vec<_>>();

        let mut transaction = self.begin_transaction().await?;
        let command_status_cursor = self
            .insert_pending_scan_commands(&mut transaction, &command_ids, &batch.requests)
            .await
            .map_err(|error| format!("postgres collection scan command insert failed: {error}"))?;
        let system_event_cursor = self
            .insert_system_events_in_transaction(&mut transaction, &system_events)
            .await?;
        self.commit_transaction(transaction).await?;

        self.command_status_source_cursor =
            max_row_source_cursor(&self.command_status_source_cursor, command_status_cursor);
        self.system_event_source_cursor =
            max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
        for ((command_id, request), event) in command_ids
            .iter()
            .cloned()
            .zip(batch.requests)
            .zip(system_events)
        {
            self.push_system_event(event);
            Arc::make_mut(&mut self.order).push(command_id.clone());
            let snapshot_command_id = command_id.clone();
            Arc::make_mut(&mut self.commands).insert(
                command_id,
                ScanCommandRecord {
                    request,
                    status: ScanCommandStatus::Pending,
                },
            );
            self.set_command_status_snapshot(
                snapshot_command_id.as_ref(),
                ScanCommandStatus::Pending,
            );
        }

        Ok(command_ids)
    }

    /// Durably materialize due collection scan schedules into canonical pending commands.
    ///
    /// # Errors
    ///
    /// Returns an error string when the durable write fails.
    pub async fn drain_due_collection_scans(
        &mut self,
        max_collections: usize,
        now_unix_ms: u64,
    ) -> Result<DrainDueCollectionScansResult, String> {
        self.refresh_from_remote_if_stale().await?;
        if max_collections == 0 {
            return Ok(DrainDueCollectionScansResult {
                outcome: "idle".into(),
                processed_collections: 0,
                enqueued_commands: 0,
                pending_due_remaining: 0,
                last_collection_key: None,
                partial_progress: false,
                last_error: None,
            });
        }

        let (due_scans, schedule_rows, pending_due_remaining) =
            self.collect_due_collection_scans(now_unix_ms, max_collections);
        let processed_collections = due_scans.len();
        if due_scans.is_empty() {
            return Ok(DrainDueCollectionScansResult {
                outcome: "idle".into(),
                processed_collections: 0,
                enqueued_commands: 0,
                pending_due_remaining,
                last_collection_key: None,
                partial_progress: false,
                last_error: None,
            });
        }

        let all_requests = Self::flatten_due_scan_requests(&due_scans);
        let command_ids = (0..all_requests.len())
            .map(|_| next_command_id())
            .collect::<Vec<_>>();
        let system_events = Self::build_due_collection_system_events(
            &due_scans,
            &command_ids,
            &all_requests,
            now_unix_ms,
        );
        let tail_cursors = self
            .persist_due_collection_scans(
                &command_ids,
                &all_requests,
                &schedule_rows,
                &system_events,
            )
            .await?;

        self.apply_due_collection_scan_state(
            &due_scans,
            now_unix_ms,
            system_events,
            &command_ids,
            all_requests,
            tail_cursors,
        );

        Ok(DrainDueCollectionScansResult {
            outcome: if pending_due_remaining == 0 {
                "drained".into()
            } else {
                "limited".into()
            },
            processed_collections,
            enqueued_commands: command_ids.len(),
            pending_due_remaining,
            last_collection_key: due_scans
                .last()
                .map(|due_scan| due_scan.collection_key.clone()),
            partial_progress: false,
            last_error: None,
        })
    }

    async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, String> {
        self.pool
            .begin()
            .await
            .map_err(|error| format!("postgres transaction begin failed: {error}"))
    }

    async fn commit_transaction(
        &self,
        transaction: sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        transaction
            .commit()
            .await
            .map_err(|error| format!("postgres transaction commit failed: {error}"))
    }

    async fn insert_pending_scan_commands(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        command_ids: &[Box<str>],
        requests: &[ScanRequest],
    ) -> Result<RowSourceCursor, String> {
        if requests.is_empty() {
            return Ok(RowSourceCursor::default());
        }

        let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(format!(
            "INSERT INTO {} (command_id, component_key, artifact_kind, artifact_identity, freshness, status) ",
            self.names.scan_commands
        ));
        query_builder.push_values(
            command_ids.iter().zip(requests.iter()),
            |mut row, (command_id, request)| {
                row.push_bind(command_id.as_ref())
                    .push_bind(request.component_key.as_ref())
                    .push_bind(artifact_kind_name(request.artifact.kind))
                    .push_bind(request.artifact.identity.as_ref())
                    .push_bind(freshness_name(request.freshness))
                    .push_bind(scan_command_status_name(ScanCommandStatus::Pending));
            },
        );
        query_builder.push(
            " RETURNING command_id, (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_micros",
        );
        let rows = query_builder
            .build_query_as::<(String, i64)>()
            .fetch_all(&mut **transaction)
            .await
            .map_err(|error| format!("postgres due collection scan insert failed: {error}"))?;
        let mut cursor = RowSourceCursor::default();
        for (command_id, updated_at_micros) in rows {
            cursor = max_row_source_cursor(
                &cursor,
                row_source_cursor(
                    u64::try_from(updated_at_micros)
                        .map_err(|_| "postgres scan command updated_at out of range".to_owned())?,
                    command_id.into_boxed_str(),
                ),
            );
        }
        Ok(cursor)
    }

    async fn upsert_collection_scan_schedules(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        schedule_rows: &[(Box<str>, venom_domain::CollectionScanSchedule)],
    ) -> Result<(), String> {
        if schedule_rows.is_empty() {
            return Ok(());
        }

        let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(format!(
            "INSERT INTO {} (collection_key, cadence_minutes, freshness, next_due_at_unix_ms, last_materialized_at_unix_ms, last_enqueued_commands) ",
            self.names.collection_scan_schedules
        ));
        query_builder.push_values(
            schedule_rows.iter(),
            |mut row, (collection_key, schedule)| {
                row.push_bind(collection_key.as_ref())
                    .push_bind(
                        i32::try_from(schedule.cadence_minutes).expect("cadence should fit i32"),
                    )
                    .push_bind(freshness_name(schedule.freshness))
                    .push_bind(
                        i64::try_from(schedule.next_due_at_unix_ms)
                            .expect("next due should fit i64"),
                    )
                    .push_bind(schedule.last_materialized_at_unix_ms.map(|value| {
                        i64::try_from(value).expect("last materialized should fit i64")
                    }))
                    .push_bind(
                        schedule
                            .last_enqueued_commands
                            .map(i32::try_from)
                            .transpose()
                            .expect("last command count should fit i32"),
                    );
            },
        );
        query_builder.push(
            " ON CONFLICT (collection_key) DO UPDATE SET cadence_minutes = EXCLUDED.cadence_minutes, freshness = EXCLUDED.freshness, next_due_at_unix_ms = EXCLUDED.next_due_at_unix_ms, last_materialized_at_unix_ms = EXCLUDED.last_materialized_at_unix_ms, last_enqueued_commands = EXCLUDED.last_enqueued_commands, updated_at = NOW()",
        );
        query_builder
            .build()
            .execute(&mut **transaction)
            .await
            .map_err(|error| {
                format!("postgres collection scan schedule batch upsert failed: {error}")
            })?;
        Ok(())
    }

    #[must_use]
    pub fn command_status(&self, command_id: &str) -> Option<ScanCommandStatus> {
        self.commands.get(command_id).map(|record| record.status)
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn command_statuses_snapshot(&self) -> BTreeMap<Box<str>, ScanCommandStatus> {
        self.command_statuses_snapshot_cache.as_ref().clone()
    }

    #[must_use]
    pub fn command_statuses_snapshot_arc(&self) -> Arc<BTreeMap<Box<str>, ScanCommandStatus>> {
        Arc::clone(&self.command_statuses_snapshot_cache)
    }

    #[must_use]
    pub fn command_status_source_cursor(&self) -> RowSourceCursor {
        self.command_status_source_cursor.clone()
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
            let occurred_at_unix_ms = current_unix_millis()?;
            if let Some(failure) = self
                .publish_pending_integration_event_attempt(
                    &event,
                    attempted_at_micros,
                    occurred_at_unix_ms,
                    publisher,
                )
                .await?
            {
                result.last_failure = Some(failure);
                break;
            }
            result.published += 1;
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
                self.refresh_from_remote_if_stale().await?;
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
        let mut candidate_read_model = self.read_model_arc();
        let change_set = candidate_ingestion
            .record_scan_report(&report)
            .map_err(|error| format!("provider report cannot be applied: {}", error.as_str()))?;
        Arc::make_mut(&mut candidate_read_model).record_scan_report(&report);
        let findings_reported = report.findings.len();
        let finding_changes_event = PendingIntegrationEvent::finding_changes_observed(
            request.component_key.clone(),
            request.artifact.clone(),
            report.provider_key.clone(),
            request.freshness,
            report.observed_at,
            change_set.clone(),
        );
        let scan_command_completed_event = PendingIntegrationEvent::scan_command_completed(
            command_id.as_ref(),
            request.component_key.clone(),
            request.artifact.clone(),
            report.provider_key.clone(),
            request.freshness,
            findings_reported,
            change_set.clone(),
        );
        let occurred_at_unix_ms = current_unix_millis()?;
        let system_event = SystemEvent {
            event_id: next_system_event_id("scan-command-completed"),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandCompleted,
            collection_key: None,
            component_key: Some(request.component_key.clone()),
            command_id: Some(command_id.clone()),
            integration_event_id: None,
            finding_count: u32::try_from(findings_reported).ok(),
            retryable: None,
            detail: Some(
                format!(
                    "discovered {}, repeated {}, withdrawn {}, active {}",
                    change_set.discovered,
                    change_set.repeated,
                    change_set.withdrawn,
                    change_set.active
                )
                .into_boxed_str(),
            ),
        };
        let (provider_report_row_id, command_status_cursor, system_event_cursor) = self
            .persist_completed_scan_command(
                command_id.as_ref(),
                &report,
                &finding_changes_event,
                &scan_command_completed_event,
                &system_event,
            )
            .await?;

        self.ingestion = candidate_ingestion;
        self.read_model = candidate_read_model;
        self.provider_report_row_high_watermark = self
            .provider_report_row_high_watermark
            .max(provider_report_row_id);
        self.refresh_read_model_and_release_board_snapshot_caches();
        let completed = CompletedScanCommand {
            command_id,
            provider_key: report.provider_key,
            findings_reported,
            change_set,
        };
        self.apply_completed_scan_command(
            &completed,
            finding_changes_event,
            scan_command_completed_event,
            system_event,
            command_status_cursor,
            system_event_cursor,
        );

        Ok(RunNextScanResult::Completed(completed))
    }

    fn collect_due_collection_scans(
        &self,
        now_unix_ms: u64,
        max_collections: usize,
    ) -> (Vec<DueCollectionScan>, DueCollectionScanRows, usize) {
        let due_scans = CollectionScanScheduler::new(self.ingestion.inventory())
            .collect_due(now_unix_ms, max_collections);
        let schedule_rows = Self::build_due_schedule_rows(self.ingestion.inventory(), &due_scans);
        let pending_due_remaining = self
            .ingestion
            .inventory()
            .due_collection_keys(now_unix_ms, usize::MAX)
            .len();
        (due_scans, schedule_rows, pending_due_remaining)
    }

    fn build_due_schedule_rows(
        inventory: &ComponentInventory,
        due_scans: &[DueCollectionScan],
    ) -> Vec<(Box<str>, venom_domain::CollectionScanSchedule)> {
        due_scans
            .iter()
            .map(|due_scan| {
                let schedule = inventory
                    .collection_scan_schedule(due_scan.collection_key.as_ref())
                    .expect("scheduled collection should still exist after scheduler pass");
                (due_scan.collection_key.clone(), schedule)
            })
            .collect()
    }

    fn flatten_due_scan_requests(due_scans: &[DueCollectionScan]) -> Vec<ScanRequest> {
        due_scans
            .iter()
            .flat_map(|due_scan| due_scan.requests.iter().cloned())
            .collect()
    }

    async fn persist_due_collection_scans(
        &self,
        command_ids: &[Box<str>],
        requests: &[ScanRequest],
        schedule_rows: &[(Box<str>, venom_domain::CollectionScanSchedule)],
        system_events: &[SystemEvent],
    ) -> Result<TailRefreshCursors, String> {
        let mut transaction = self.begin_transaction().await?;
        let command_status_cursor = self
            .insert_pending_scan_commands(&mut transaction, command_ids, requests)
            .await?;
        self.upsert_collection_scan_schedules(&mut transaction, schedule_rows)
            .await?;
        let system_event_cursor = self
            .insert_system_events_in_transaction(&mut transaction, system_events)
            .await?;
        self.commit_transaction(transaction).await?;
        Ok(TailRefreshCursors {
            command_status: command_status_cursor,
            system_event: system_event_cursor,
        })
    }

    fn apply_due_collection_scan_state(
        &mut self,
        due_scans: &[DueCollectionScan],
        materialized_at_unix_ms: u64,
        system_events: Vec<SystemEvent>,
        command_ids: &[Box<str>],
        requests: Vec<ScanRequest>,
        tail_cursors: TailRefreshCursors,
    ) {
        for due_scan in due_scans {
            let _ = self
                .ingestion
                .inventory_mut()
                .record_collection_scan_materialization(
                    due_scan.collection_key.as_ref(),
                    due_scan.next_due_at_unix_ms,
                    materialized_at_unix_ms,
                    u32::try_from(due_scan.requests.len()).unwrap_or(u32::MAX),
                );
        }
        for event in system_events {
            self.push_system_event(event);
        }
        for (command_id, request) in command_ids.iter().cloned().zip(requests) {
            Arc::make_mut(&mut self.order).push(command_id.clone());
            let snapshot_command_id = command_id.clone();
            Arc::make_mut(&mut self.commands).insert(
                command_id,
                ScanCommandRecord {
                    request,
                    status: ScanCommandStatus::Pending,
                },
            );
            self.set_command_status_snapshot(
                snapshot_command_id.as_ref(),
                ScanCommandStatus::Pending,
            );
        }
        self.command_status_source_cursor = max_row_source_cursor(
            &self.command_status_source_cursor,
            tail_cursors.command_status,
        );
        self.system_event_source_cursor =
            max_event_source_cursor(&self.system_event_source_cursor, tail_cursors.system_event);
    }

    fn build_due_collection_system_events(
        due_scans: &[DueCollectionScan],
        command_ids: &[Box<str>],
        requests: &[ScanRequest],
        now_unix_ms: u64,
    ) -> Vec<SystemEvent> {
        let mut events = Vec::with_capacity(due_scans.len() + command_ids.len());
        let mut command_iter = command_ids.iter().cloned().zip(requests.iter().cloned());
        for due_scan in due_scans {
            events.push(SystemEvent {
                event_id: next_system_event_id("collection-scan-materialized"),
                occurred_at_unix_ms: now_unix_ms,
                kind: SystemEventKind::CollectionScanMaterialized,
                collection_key: Some(due_scan.collection_key.clone()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(due_scan.requests.len()).ok(),
                retryable: None,
                detail: Some(
                    format!(
                        "next due {}, enqueued {}",
                        due_scan.next_due_at_unix_ms,
                        due_scan.requests.len()
                    )
                    .into_boxed_str(),
                ),
            });
            for (command_id, request) in command_iter.by_ref().take(due_scan.requests.len()) {
                events.push(SystemEvent {
                    event_id: next_system_event_id("scan-command-enqueued"),
                    occurred_at_unix_ms: now_unix_ms,
                    kind: SystemEventKind::ScanCommandEnqueued,
                    collection_key: None,
                    component_key: Some(request.component_key.clone()),
                    command_id: Some(command_id),
                    integration_event_id: None,
                    finding_count: None,
                    retryable: None,
                    detail: None,
                });
            }
        }
        events
    }

    async fn publish_pending_integration_event_attempt(
        &mut self,
        event: &PendingIntegrationEvent,
        attempted_at_micros: i64,
        occurred_at_unix_ms: u64,
        publisher: &(impl IntegrationEventPublisher + Sync),
    ) -> Result<Option<IntegrationEventPublicationFailure>, String> {
        match publisher.publish(event).await {
            Ok(()) => {
                self.mark_integration_event_published(
                    event.event_id.as_ref(),
                    attempted_at_micros,
                    occurred_at_unix_ms,
                )
                .await?;
                Ok(None)
            }
            Err(error) => self
                .mark_integration_event_publish_failed(
                    event.event_id.as_ref(),
                    attempted_at_micros,
                    occurred_at_unix_ms,
                    error,
                )
                .await
                .map(Some),
        }
    }

    async fn mark_integration_event_published(
        &mut self,
        event_id: &str,
        attempted_at_micros: i64,
        occurred_at_unix_ms: u64,
    ) -> Result<(), String> {
        let system_event = SystemEvent {
            event_id: next_system_event_id("integration-event-published"),
            occurred_at_unix_ms,
            kind: SystemEventKind::IntegrationEventPublished,
            collection_key: None,
            component_key: None,
            command_id: None,
            integration_event_id: Some(event_id.into()),
            finding_count: None,
            retryable: None,
            detail: None,
        };
        let mut transaction = self.begin_transaction().await?;
        sqlx::query(&format!(
            concat!(
                "UPDATE {} ",
                "SET publication_status = 'published', last_error = NULL, ",
                "last_attempted_at_micros = $2, published_at_micros = $3, attempt_count = attempt_count + 1 ",
                "WHERE event_id = $1"
            ),
            self.names.integration_outbox
        ))
        .bind(event_id)
        .bind(attempted_at_micros)
        .bind(attempted_at_micros)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("postgres integration outbox publish update failed: {error}"))?;
        let system_event_cursor = self
            .insert_system_event_in_transaction(&mut transaction, &system_event)
            .await?;
        self.commit_transaction(transaction).await?;

        self.remove_pending_integration_event(event_id);
        self.system_event_source_cursor =
            max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
        self.push_system_event(system_event);
        Ok(())
    }

    async fn mark_integration_event_publish_failed(
        &mut self,
        event_id: &str,
        attempted_at_micros: i64,
        occurred_at_unix_ms: u64,
        error: IntegrationEventPublishError,
    ) -> Result<IntegrationEventPublicationFailure, String> {
        let failure = IntegrationEventPublicationFailure {
            event_id: event_id.into(),
            retryable: error.retryable,
            message: error.message,
        };
        let system_event = SystemEvent {
            event_id: next_system_event_id("integration-event-publication-failed"),
            occurred_at_unix_ms,
            kind: SystemEventKind::IntegrationEventPublicationFailed,
            collection_key: None,
            component_key: None,
            command_id: None,
            integration_event_id: Some(failure.event_id.clone()),
            finding_count: None,
            retryable: Some(failure.retryable),
            detail: Some(failure.message.clone()),
        };
        let mut transaction = self.begin_transaction().await?;
        sqlx::query(&format!(
            concat!(
                "UPDATE {} ",
                "SET publication_status = 'pending', last_error = $2, ",
                "last_attempted_at_micros = $3, attempt_count = attempt_count + 1 ",
                "WHERE event_id = $1"
            ),
            self.names.integration_outbox
        ))
        .bind(event_id)
        .bind(failure.message.as_ref())
        .bind(attempted_at_micros)
        .execute(&mut *transaction)
        .await
        .map_err(|sql_error| {
            format!("postgres integration outbox failure update failed: {sql_error}")
        })?;
        let system_event_cursor = self
            .insert_system_event_in_transaction(&mut transaction, &system_event)
            .await?;
        self.commit_transaction(transaction).await?;

        self.system_event_source_cursor =
            max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
        self.push_system_event(system_event);
        Ok(failure)
    }

    async fn persist_completed_scan_command(
        &self,
        command_id: &str,
        report: &ProviderScanReport,
        finding_changes_event: &PendingIntegrationEvent,
        scan_command_completed_event: &PendingIntegrationEvent,
        system_event: &SystemEvent,
    ) -> Result<(u64, RowSourceCursor, EventSourceCursor), String> {
        let mut transaction = self.begin_transaction().await?;
        let provider_report_row_id = self
            .insert_provider_report(&mut transaction, report)
            .await?;
        self.insert_pending_integration_events(
            &mut transaction,
            &[
                finding_changes_event.clone(),
                scan_command_completed_event.clone(),
            ],
        )
        .await?;
        let (updated_at_micros, updated_command_id) = sqlx::query_as::<_, (i64, String)>(&format!(
            concat!(
                "UPDATE {} ",
                "SET status = $2, updated_at = NOW() ",
                "WHERE command_id = $1 ",
                "RETURNING (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_micros, command_id"
            ),
            self.names.scan_commands
        ))
        .bind(command_id)
        .bind(scan_command_status_name(ScanCommandStatus::Completed))
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| format!("postgres scan command completion failed: {error}"))?;
        let system_event_cursor = self
            .insert_system_events_in_transaction(
                &mut transaction,
                std::slice::from_ref(system_event),
            )
            .await?;
        self.commit_transaction(transaction).await?;
        Ok((
            u64::try_from(provider_report_row_id)
                .map_err(|_| "postgres provider report id out of range".to_owned())?,
            row_source_cursor(
                u64::try_from(updated_at_micros)
                    .map_err(|_| "postgres scan command updated_at out of range".to_owned())?,
                updated_command_id.into_boxed_str(),
            ),
            system_event_cursor,
        ))
    }

    fn apply_completed_scan_command(
        &mut self,
        completed: &CompletedScanCommand,
        finding_changes_event: PendingIntegrationEvent,
        scan_command_completed_event: PendingIntegrationEvent,
        system_event: SystemEvent,
        command_status_cursor: RowSourceCursor,
        system_event_cursor: EventSourceCursor,
    ) {
        Arc::make_mut(&mut self.pending_integration_events).push(finding_changes_event);
        Arc::make_mut(&mut self.pending_integration_events).push(scan_command_completed_event);
        let command = Arc::make_mut(&mut self.commands)
            .get_mut(completed.command_id.as_ref())
            .expect("completed scan command missing from postgres runtime");
        command.status = ScanCommandStatus::Completed;
        self.set_command_status_snapshot(
            completed.command_id.as_ref(),
            ScanCommandStatus::Completed,
        );
        self.command_status_source_cursor =
            max_row_source_cursor(&self.command_status_source_cursor, command_status_cursor);
        self.system_event_source_cursor =
            max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
        self.push_system_event(system_event);
    }

    async fn insert_provider_report(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        report: &ProviderScanReport,
    ) -> Result<i64, String> {
        sqlx::query_scalar::<_, i64>(&format!(
            concat!(
                "INSERT INTO {} ",
                "(provider_key, component_key, artifact_kind, artifact_identity, observed_at_micros, freshness, knowledge_revision, findings) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id"
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
        .fetch_one(&mut **transaction)
        .await
        .map_err(|error| format!("postgres provider report insert failed: {error}"))
    }

    async fn insert_pending_integration_events(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        events: &[PendingIntegrationEvent],
    ) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }

        let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(format!(
            "INSERT INTO {} (event_id, event_kind, payload, publication_status) ",
            self.names.integration_outbox
        ));
        query_builder.push_values(events.iter(), |mut row, event| {
            row.push_bind(event.event_id.as_ref())
                .push_bind(event.event.kind_name())
                .push_bind(Json(event.clone()))
                .push_bind("pending");
        });
        query_builder
            .build()
            .execute(&mut **transaction)
            .await
            .map_err(|error| format!("postgres integration outbox insert failed: {error}"))?;
        Ok(())
    }

    async fn insert_system_events_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        events: &[SystemEvent],
    ) -> Result<EventSourceCursor, String> {
        let mut cursor = EventSourceCursor::default();
        for event in events {
            let (created_at_micros, event_id) = sqlx::query_as::<_, (i64, String)>(&format!(
                concat!(
                    "INSERT INTO {} (event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                    "command_id, integration_event_id, finding_count, retryable, detail) ",
                    "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ",
                    "RETURNING (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_micros, event_id"
                ),
                self.names.system_events
            ))
            .bind(event.event_id.as_ref())
            .bind(i64::try_from(event.occurred_at_unix_ms).map_err(|_| {
                "system event occurred_at_unix_ms does not fit postgres".to_owned()
            })?)
            .bind(event.kind.category().as_str())
            .bind(event.kind.as_str())
            .bind(event.collection_key.as_deref())
            .bind(event.component_key.as_deref())
            .bind(event.command_id.as_deref())
            .bind(event.integration_event_id.as_deref())
            .bind(
                event.finding_count
                    .map(i32::try_from)
                    .transpose()
                    .map_err(|_| "system event finding count does not fit postgres".to_owned())?,
            )
            .bind(event.retryable)
            .bind(event.detail.as_deref())
            .fetch_one(&mut **transaction)
            .await
            .map_err(|error| format!("postgres system event insert failed: {error}"))?;
            cursor = max_event_source_cursor(
                &cursor,
                event_source_cursor(
                    u64::try_from(created_at_micros)
                        .map_err(|_| "postgres system event created_at out of range".to_owned())?,
                    event_id.into_boxed_str(),
                ),
            );
        }
        Ok(cursor)
    }

    async fn delete_governance_decision_rows_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        finding: &FindingRef,
    ) -> Result<(), String> {
        let package_purl = finding.package.purl.as_deref().unwrap_or("");
        sqlx::query(&format!(
            concat!(
                "DELETE FROM {} ",
                "WHERE component_key = $1 AND artifact_kind = $2 AND artifact_identity = $3 ",
                "AND vulnerability_id = $4 AND package_name = $5 AND package_version = $6 AND package_purl = $7"
            ),
            self.names.finding_risk_acceptances
        ))
        .bind(finding.component_key.as_ref())
        .bind(artifact_kind_name(finding.artifact.kind))
        .bind(finding.artifact.identity.as_ref())
        .bind(finding.vulnerability_id.as_ref())
        .bind(finding.package.name.as_ref())
        .bind(finding.package.version.as_ref())
        .bind(package_purl)
        .execute(&mut **transaction)
        .await
        .map_err(|error| format!("postgres risk acceptance delete failed: {error}"))?;

        sqlx::query(&format!(
            concat!(
                "DELETE FROM {} ",
                "WHERE component_key = $1 AND artifact_kind = $2 AND artifact_identity = $3 ",
                "AND vulnerability_id = $4 AND package_name = $5 AND package_version = $6 AND package_purl = $7"
            ),
            self.names.finding_suppressions
        ))
        .bind(finding.component_key.as_ref())
        .bind(artifact_kind_name(finding.artifact.kind))
        .bind(finding.artifact.identity.as_ref())
        .bind(finding.vulnerability_id.as_ref())
        .bind(finding.package.name.as_ref())
        .bind(finding.package.version.as_ref())
        .bind(package_purl)
        .execute(&mut **transaction)
        .await
        .map_err(|error| format!("postgres suppression delete failed: {error}"))?;
        Ok(())
    }

    async fn fail_scan_command(
        &mut self,
        command_id: Box<str>,
        error: FindingProviderError,
    ) -> Result<RunNextScanResult, String> {
        let component_key = self
            .commands
            .get(command_id.as_ref())
            .map(|record| record.request.component_key.clone());
        let occurred_at_unix_ms = current_unix_millis()?;
        let (updated_at_micros, updated_command_id) = sqlx::query_as::<_, (i64, String)>(&format!(
            concat!(
                "UPDATE {} ",
                "SET status = $2, error_code = $3, retryable = $4, detail = $5, updated_at = NOW() ",
                "WHERE command_id = $1 ",
                "RETURNING (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_micros, command_id"
            ),
            self.names.scan_commands
        ))
        .bind(command_id.as_ref())
        .bind(scan_command_status_name(ScanCommandStatus::Failed))
        .bind(provider_error_code(error.kind))
        .bind(error.retryable)
        .bind(error.message.as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(|sql_error| format!("postgres scan command failure update failed: {sql_error}"))?;

        let Some(command) = Arc::make_mut(&mut self.commands).get_mut(command_id.as_ref()) else {
            return Err("failed scan command missing from postgres runtime".to_owned());
        };
        command.status = ScanCommandStatus::Failed;
        self.set_command_status_snapshot(command_id.as_ref(), ScanCommandStatus::Failed);
        let event = SystemEvent {
            event_id: next_system_event_id("scan-command-failed"),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandFailed,
            collection_key: None,
            component_key,
            command_id: Some(command_id.clone()),
            integration_event_id: None,
            finding_count: None,
            retryable: Some(error.retryable),
            detail: Some(error.message.clone()),
        };
        self.command_status_source_cursor = max_row_source_cursor(
            &self.command_status_source_cursor,
            row_source_cursor(
                u64::try_from(updated_at_micros)
                    .map_err(|_| "postgres scan command updated_at out of range".to_owned())?,
                updated_command_id.into_boxed_str(),
            ),
        );
        let system_event_cursor = self.insert_system_event(&event).await?;
        self.system_event_source_cursor =
            max_event_source_cursor(&self.system_event_source_cursor, system_event_cursor);
        self.push_system_event(event);

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

        self.create_change_watermark_table().await?;
        self.create_change_journal_table().await?;
        self.create_components_table().await?;
        self.create_context_profiles_table().await?;
        self.create_component_context_profiles_table().await?;
        self.create_component_tags_table().await?;
        self.create_component_tag_memberships_table().await?;
        self.create_collections_table().await?;
        self.create_collection_sources_table().await?;
        self.create_collection_memberships_table().await?;
        self.create_collection_scan_schedules_table().await?;
        self.create_artifact_bindings_table().await?;
        self.create_provider_runtime_configs_table().await?;
        self.create_integration_runtime_config_table().await?;
        self.create_provider_reports_table().await?;
        self.create_finding_risk_acceptances_table().await?;
        self.create_finding_suppressions_table().await?;
        self.create_finding_governance_journal_table().await?;
        self.create_scan_commands_table().await?;
        self.create_integration_outbox_table().await?;
        self.create_system_events_table().await?;
        self.install_change_watermark_triggers().await?;

        Ok(())
    }

    async fn create_change_watermark_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "singleton BOOLEAN PRIMARY KEY DEFAULT TRUE, ",
                "change_seq BIGINT NOT NULL DEFAULT 0",
                ")"
            ),
            self.names.change_watermark
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark table create failed: {error}"))?;
        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} (singleton, change_seq) VALUES (TRUE, 0) ",
                "ON CONFLICT (singleton) DO NOTHING"
            ),
            self.names.change_watermark
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark seed failed: {error}"))?;
        sqlx::query(&format!(
            concat!(
                "CREATE OR REPLACE FUNCTION {}.touch_change_watermark() RETURNS trigger AS $$ ",
                "DECLARE next_change_seq BIGINT; ",
                "DECLARE lane_mask INTEGER; ",
                "BEGIN ",
                "UPDATE {} SET change_seq = change_seq + 1 WHERE singleton = TRUE RETURNING change_seq INTO next_change_seq; ",
                "lane_mask := CASE TG_TABLE_NAME ",
                "WHEN 'components' THEN {} ",
                "WHEN 'context_profiles' THEN {} ",
                "WHEN 'component_context_profiles' THEN {} ",
                "WHEN 'component_tags' THEN {} ",
                "WHEN 'component_tag_memberships' THEN {} ",
                "WHEN 'collections' THEN {} ",
                "WHEN 'collection_sources' THEN {} ",
                "WHEN 'collection_memberships' THEN {} ",
                "WHEN 'collection_scan_schedules' THEN {} ",
                "WHEN 'artifact_bindings' THEN {} ",
                "WHEN 'provider_runtime_configs' THEN {} ",
                "WHEN 'integration_runtime_config' THEN {} ",
                "WHEN 'provider_reports' THEN {} ",
                "WHEN 'finding_risk_acceptances' THEN {} ",
                "WHEN 'finding_suppressions' THEN {} ",
                "WHEN 'finding_governance_journal' THEN {} ",
                "WHEN 'scan_commands' THEN {} ",
                "WHEN 'integration_outbox' THEN {} ",
                "WHEN 'system_events' THEN {} ",
                "ELSE 0 END; ",
                "INSERT INTO {} (change_seq, lane_mask) VALUES (next_change_seq, lane_mask) ",
                "ON CONFLICT (change_seq) DO UPDATE SET lane_mask = {}.lane_mask | EXCLUDED.lane_mask; ",
                "DELETE FROM {} WHERE change_seq < next_change_seq - 4096; ",
                "RETURN NULL; ",
                "END; ",
                "$$ LANGUAGE plpgsql"
            ),
            self.names.schema,
            self.names.change_watermark,
            CHANGE_LANE_INVENTORY,
            CHANGE_LANE_INVENTORY,
            CHANGE_LANE_COMPONENT_BINDINGS,
            CHANGE_LANE_INVENTORY,
            CHANGE_LANE_COMPONENT_BINDINGS,
            CHANGE_LANE_COLLECTIONS,
            CHANGE_LANE_COLLECTIONS,
            CHANGE_LANE_COLLECTIONS,
            CHANGE_LANE_COLLECTION_SCHEDULES,
            CHANGE_LANE_COMPONENT_BINDINGS,
            CHANGE_LANE_PROVIDER_RUNTIME_CONFIGS,
            CHANGE_LANE_INTEGRATION_RUNTIME,
            CHANGE_LANE_READ_MODEL,
            CHANGE_LANE_GOVERNANCE,
            CHANGE_LANE_GOVERNANCE,
            CHANGE_LANE_GOVERNANCE,
            CHANGE_LANE_COMMAND_STATUSES,
            CHANGE_LANE_INTEGRATION_OUTBOX,
            CHANGE_LANE_SYSTEM_EVENTS,
            self.names.change_journal,
            self.names.change_journal,
            self.names.change_journal
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark function create failed: {error}"))?;
        Ok(())
    }

    async fn create_change_journal_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "change_seq BIGINT PRIMARY KEY, ",
                "lane_mask INTEGER NOT NULL",
                ")"
            ),
            self.names.change_journal
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres change journal table create failed: {error}"))?;
        Ok(())
    }

    async fn install_change_watermark_triggers(&self) -> Result<(), String> {
        for table in [
            self.names.components.as_ref(),
            self.names.context_profiles.as_ref(),
            self.names.component_context_profiles.as_ref(),
            self.names.component_tags.as_ref(),
            self.names.component_tag_memberships.as_ref(),
            self.names.collections.as_ref(),
            self.names.collection_sources.as_ref(),
            self.names.collection_memberships.as_ref(),
            self.names.collection_scan_schedules.as_ref(),
            self.names.artifact_bindings.as_ref(),
            self.names.provider_runtime_configs.as_ref(),
            self.names.integration_runtime_config.as_ref(),
            self.names.provider_reports.as_ref(),
            self.names.finding_risk_acceptances.as_ref(),
            self.names.finding_suppressions.as_ref(),
            self.names.finding_governance_journal.as_ref(),
            self.names.scan_commands.as_ref(),
            self.names.integration_outbox.as_ref(),
            self.names.system_events.as_ref(),
        ] {
            self.install_change_watermark_trigger(table).await?;
        }
        Ok(())
    }

    async fn install_change_watermark_trigger(&self, qualified_table: &str) -> Result<(), String> {
        let trigger_name = qualified_table.replace('.', "_");
        let trigger_name = format!("{trigger_name}_touch_change_watermark");
        sqlx::query(&format!(
            "DROP TRIGGER IF EXISTS {trigger_name} ON {qualified_table}"
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark trigger drop failed: {error}"))?;
        sqlx::query(&format!(
            "CREATE TRIGGER {trigger_name} AFTER INSERT OR UPDATE OR DELETE ON {qualified_table} FOR EACH STATEMENT EXECUTE FUNCTION {}.touch_change_watermark()",
            self.names.schema,
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark trigger create failed: {error}"))?;
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

    async fn create_context_profiles_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "profile_key TEXT PRIMARY KEY, ",
                "name TEXT NOT NULL, ",
                "internet_exposed BOOLEAN NULL, ",
                "production BOOLEAN NULL, ",
                "mission_critical BOOLEAN NULL, ",
                "vpn_restricted BOOLEAN NULL, ",
                "non_privileged_user BOOLEAN NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.context_profiles
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres context profiles table create failed: {error}"))?;
        sqlx::query(&format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS vpn_restricted BOOLEAN NULL",
            self.names.context_profiles
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres context profiles table alter failed: {error}"))?;
        sqlx::query(&format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS non_privileged_user BOOLEAN NULL",
            self.names.context_profiles
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres context profiles table alter failed: {error}"))?;
        Ok(())
    }

    async fn create_component_context_profiles_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "component_key TEXT PRIMARY KEY REFERENCES {}(component_key) ON DELETE CASCADE, ",
                "profile_key TEXT NOT NULL REFERENCES {}(profile_key) ON DELETE CASCADE, ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.component_context_profiles,
            self.names.components,
            self.names.context_profiles
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres component context profiles table create failed: {error}")
        })?;
        Ok(())
    }

    async fn create_component_tags_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "tag_key TEXT PRIMARY KEY, ",
                "name TEXT NOT NULL, ",
                "context_profile_key TEXT NULL REFERENCES {}(profile_key) ON DELETE SET NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.component_tags, self.names.context_profiles
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres component tags table create failed: {error}"))?;
        Ok(())
    }

    async fn create_component_tag_memberships_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "tag_key TEXT NOT NULL REFERENCES {}(tag_key) ON DELETE CASCADE, ",
                "component_key TEXT NOT NULL REFERENCES {}(component_key) ON DELETE CASCADE, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "PRIMARY KEY (tag_key, component_key)",
                ")"
            ),
            self.names.component_tag_memberships, self.names.component_tags, self.names.components
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres component tag memberships table create failed: {error}")
        })?;
        Ok(())
    }

    async fn create_collections_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "collection_key TEXT PRIMARY KEY, ",
                "name TEXT NOT NULL, ",
                "context_profile_key TEXT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.collections
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres collections table create failed: {error}"))?;
        sqlx::query(&format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS context_profile_key TEXT NULL",
            self.names.collections
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres collections table alter failed: {error}"))?;
        Ok(())
    }

    async fn create_collection_memberships_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "collection_key TEXT NOT NULL REFERENCES {}(collection_key) ON DELETE CASCADE, ",
                "component_key TEXT NOT NULL REFERENCES {}(component_key) ON DELETE CASCADE, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "PRIMARY KEY (collection_key, component_key)",
                ")"
            ),
            self.names.collection_memberships, self.names.collections, self.names.components
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres collection memberships table create failed: {error}"))?;
        Ok(())
    }

    async fn create_collection_sources_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "collection_key TEXT PRIMARY KEY REFERENCES {}(collection_key) ON DELETE CASCADE, ",
                "source_kind TEXT NOT NULL, ",
                "mode TEXT NOT NULL, ",
                "component_keys JSONB NOT NULL, ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.collection_sources, self.names.collections
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres collection sources table create failed: {error}"))?;
        Ok(())
    }

    async fn create_collection_scan_schedules_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "collection_key TEXT PRIMARY KEY REFERENCES {}(collection_key) ON DELETE CASCADE, ",
                "cadence_minutes INTEGER NOT NULL, ",
                "freshness TEXT NOT NULL, ",
                "next_due_at_unix_ms BIGINT NOT NULL, ",
                "last_materialized_at_unix_ms BIGINT NULL, ",
                "last_enqueued_commands INTEGER NULL, ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.collection_scan_schedules, self.names.collections
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres collection scan schedules table create failed: {error}")
        })?;
        sqlx::query(&format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS last_materialized_at_unix_ms BIGINT NULL",
            self.names.collection_scan_schedules
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres collection scan schedules add last_materialized_at failed: {error}")
        })?;
        sqlx::query(&format!(
            "ALTER TABLE {} ADD COLUMN IF NOT EXISTS last_enqueued_commands INTEGER NULL",
            self.names.collection_scan_schedules
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres collection scan schedules add last_enqueued_commands failed: {error}")
        })?;
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

    async fn create_integration_runtime_config_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "id SMALLINT PRIMARY KEY, ",
                "publisher_key TEXT NOT NULL, ",
                "endpoint_url TEXT NULL, ",
                "timeout_ms BIGINT NULL, ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.integration_runtime_config
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres integration runtime config table create failed: {error}")
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

    async fn create_finding_risk_acceptances_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "component_key TEXT NOT NULL, ",
                "artifact_kind TEXT NOT NULL, ",
                "artifact_identity TEXT NOT NULL, ",
                "vulnerability_id TEXT NOT NULL, ",
                "package_name TEXT NOT NULL, ",
                "package_version TEXT NOT NULL, ",
                "package_purl TEXT NOT NULL DEFAULT '', ",
                "reason TEXT NOT NULL, ",
                "until_unix_ms BIGINT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "PRIMARY KEY (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl)",
                ")"
            ),
            self.names.finding_risk_acceptances
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres finding risk acceptances table create failed: {error}"))?;
        Ok(())
    }

    async fn create_finding_suppressions_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "component_key TEXT NOT NULL, ",
                "artifact_kind TEXT NOT NULL, ",
                "artifact_identity TEXT NOT NULL, ",
                "vulnerability_id TEXT NOT NULL, ",
                "package_name TEXT NOT NULL, ",
                "package_version TEXT NOT NULL, ",
                "package_purl TEXT NOT NULL DEFAULT '', ",
                "reason TEXT NOT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), ",
                "PRIMARY KEY (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl)",
                ")"
            ),
            self.names.finding_suppressions
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres finding suppressions table create failed: {error}"))?;
        Ok(())
    }

    async fn create_finding_governance_journal_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "id BIGSERIAL PRIMARY KEY, ",
                "component_key TEXT NOT NULL, ",
                "artifact_kind TEXT NOT NULL, ",
                "artifact_identity TEXT NOT NULL, ",
                "vulnerability_id TEXT NOT NULL, ",
                "package_name TEXT NOT NULL, ",
                "package_version TEXT NOT NULL, ",
                "package_purl TEXT NOT NULL DEFAULT '', ",
                "decision_kind TEXT NOT NULL, ",
                "reason TEXT NULL, ",
                "until_unix_ms BIGINT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.finding_governance_journal
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| {
            format!("postgres finding governance journal table create failed: {error}")
        })?;
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

    async fn create_system_events_table(&self) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "CREATE TABLE IF NOT EXISTS {} (",
                "event_id TEXT PRIMARY KEY, ",
                "occurred_at_unix_ms BIGINT NOT NULL, ",
                "category TEXT NOT NULL, ",
                "kind TEXT NOT NULL, ",
                "collection_key TEXT NULL, ",
                "component_key TEXT NULL, ",
                "command_id TEXT NULL, ",
                "integration_event_id TEXT NULL, ",
                "finding_count INTEGER NULL, ",
                "retryable BOOLEAN NULL, ",
                "detail TEXT NULL, ",
                "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
                ")"
            ),
            self.names.system_events
        ))
        .execute(&self.pool)
        .await
        .map_err(|error| format!("postgres system events table create failed: {error}"))?;
        Ok(())
    }

    async fn rebuild(&mut self) -> Result<(), String> {
        self.ingestion = FindingIngestion::new();
        self.governance = FindingGovernance::new();
        self.read_model = Arc::new(FindingReadModel::new());
        self.integration_runtime_config = None;
        self.provider_report_row_high_watermark = 0;
        self.governance_journal_high_watermark = 0;
        Arc::make_mut(&mut self.commands).clear();
        Arc::make_mut(&mut self.order).clear();
        Arc::make_mut(&mut self.pending_integration_events).clear();
        self.pending_integration_source_cursor = RowSourceCursor::default();
        self.system_event_index_snapshot_cache = Arc::new(SystemEventQueryIndex::new());
        self.system_event_source_cursor = EventSourceCursor::default();
        self.command_statuses_snapshot_cache = Arc::new(BTreeMap::new());
        self.command_status_source_cursor = RowSourceCursor::default();

        self.load_components().await?;
        self.load_context_profiles().await?;
        self.load_component_context_profiles().await?;
        self.load_component_tags().await?;
        self.load_component_tag_memberships().await?;
        self.load_collections().await?;
        self.load_collection_sources().await?;
        self.load_collection_memberships().await?;
        self.load_collection_scan_schedules().await?;
        self.load_artifact_bindings().await?;
        self.load_provider_runtime_configs().await?;
        self.load_integration_runtime_config().await?;
        self.load_provider_reports().await?;
        self.load_finding_risk_acceptances().await?;
        self.load_finding_suppressions().await?;
        self.governance_journal_high_watermark = self.load_governance_source_watermark().await?;
        self.load_scan_commands().await?;
        self.load_pending_integration_events().await?;
        self.load_system_events().await?;
        self.refresh_read_snapshot_caches();
        self.set_observed_change_watermark(self.current_change_watermark().await?);

        Ok(())
    }

    pub fn observed_change_watermark(&self) -> u64 {
        self.observed_change_watermark.load(Ordering::Relaxed)
    }

    fn set_observed_change_watermark(&self, change_watermark: u64) {
        self.observed_change_watermark
            .store(change_watermark, Ordering::Relaxed);
    }

    async fn changed_lane_mask(
        &self,
        since_change_watermark: u64,
        current_change_watermark: u64,
    ) -> Result<i32, String> {
        let earliest_retained_change_seq = sqlx::query_scalar::<_, Option<i64>>(&format!(
            "SELECT MIN(change_seq) FROM {}",
            self.names.change_journal
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres change journal coverage read failed: {error}"))?
        .map(|value| {
            u64::try_from(value)
                .map_err(|_| "postgres change journal minimum change_seq out of range".to_owned())
        })
        .transpose()?;
        let lane_mask = sqlx::query_scalar::<_, Option<i32>>(&format!(
            concat!(
                "SELECT COALESCE(bit_or(lane_mask), 0) FROM {} ",
                "WHERE change_seq > $1 AND change_seq <= $2"
            ),
            self.names.change_journal
        ))
        .bind(
            i64::try_from(since_change_watermark)
                .map_err(|_| "postgres change watermark lower bound out of range".to_owned())?,
        )
        .bind(
            i64::try_from(current_change_watermark)
                .map_err(|_| "postgres change watermark upper bound out of range".to_owned())?,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres change journal read failed: {error}"))
        .map(Option::unwrap_or_default)?;
        if change_journal_gap_requires_full_refresh(
            since_change_watermark,
            current_change_watermark,
            earliest_retained_change_seq,
        ) {
            Ok(CHANGE_LANE_ALL)
        } else {
            Ok(lane_mask)
        }
    }

    async fn current_change_watermark(&self) -> Result<u64, String> {
        sqlx::query_scalar::<_, i64>(&format!(
            "SELECT change_seq FROM {} WHERE singleton = TRUE",
            self.names.change_watermark
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark read failed: {error}"))
        .and_then(|value| {
            u64::try_from(value).map_err(|_| "postgres change watermark out of range".to_owned())
        })
    }

    async fn refresh_inventory_core_from_remote(&mut self) -> Result<(), String> {
        let mut backend = Self::detached(self.pool.clone(), self.names.clone());
        backend.ingestion = FindingIngestion::from_inventory_arc(self.ingestion.inventory_arc());
        backend.load_components().await?;
        backend.load_context_profiles().await?;
        backend.load_component_tags().await?;
        self.ingestion = backend.ingestion;
        self.refresh_inventory_snapshot_cache();
        Ok(())
    }

    async fn refresh_component_bindings_from_remote(&mut self) -> Result<(), String> {
        let mut backend = Self::detached(self.pool.clone(), self.names.clone());
        let mut inventory = Arc::unwrap_or_clone(self.ingestion.inventory_arc());
        inventory.reset_component_bindings_for_rebuild();
        backend.ingestion = FindingIngestion::from_inventory_arc(Arc::new(inventory));
        backend.load_component_context_profiles().await?;
        backend.load_component_tag_memberships().await?;
        backend.load_artifact_bindings().await?;
        self.ingestion = backend.ingestion;
        self.refresh_inventory_snapshot_cache();
        Ok(())
    }

    async fn refresh_collections_from_remote(&mut self) -> Result<(), String> {
        let mut backend = Self::detached(self.pool.clone(), self.names.clone());
        let mut inventory = Arc::unwrap_or_clone(self.ingestion.inventory_arc());
        inventory.reset_collections_for_rebuild();
        backend.ingestion = FindingIngestion::from_inventory_arc(Arc::new(inventory));
        backend.load_collections().await?;
        backend.load_collection_sources().await?;
        backend.load_collection_memberships().await?;
        backend.load_collection_scan_schedules().await?;
        self.ingestion = backend.ingestion;
        self.refresh_inventory_snapshot_cache();
        Ok(())
    }

    async fn refresh_collection_scan_schedules_from_remote(&mut self) -> Result<(), String> {
        self.load_collection_scan_schedules().await?;
        self.refresh_inventory_snapshot_cache();
        Ok(())
    }

    async fn refresh_provider_runtime_configs_from_remote(&mut self) -> Result<(), String> {
        self.load_provider_runtime_configs().await?;
        self.refresh_inventory_snapshot_cache();
        Ok(())
    }

    async fn refresh_read_model_from_remote(&mut self, lane_mask: i32) -> Result<(), String> {
        let mut backend = Self::detached(self.pool.clone(), self.names.clone());
        backend.ingestion = self.ingestion.clone();
        backend.governance = self.governance.clone();
        backend.provider_report_row_high_watermark = self.provider_report_row_high_watermark;
        backend.governance_journal_high_watermark = self.governance_journal_high_watermark;
        backend.read_model = Arc::clone(&self.read_model);
        if lane_mask & CHANGE_LANE_READ_MODEL != 0 {
            backend
                .load_provider_reports_after(self.provider_report_row_high_watermark)
                .await?;
        }
        if lane_mask & CHANGE_LANE_GOVERNANCE != 0 {
            backend
                .load_governance_journal_after(self.governance_journal_high_watermark, true)
                .await?;
        }
        let read_model = backend.read_model_arc();
        self.ingestion = backend.ingestion;
        self.governance = backend.governance;
        self.provider_report_row_high_watermark = backend.provider_report_row_high_watermark;
        self.governance_journal_high_watermark = backend.governance_journal_high_watermark;
        self.read_model = read_model;
        self.refresh_read_model_snapshot_cache();
        Ok(())
    }

    async fn refresh_command_statuses_from_remote(&mut self) -> Result<(), String> {
        self.load_scan_commands_after(self.command_status_source_cursor.clone())
            .await?;
        Ok(())
    }

    async fn refresh_pending_integration_events_from_remote(&mut self) -> Result<(), String> {
        self.load_pending_integration_events_after(self.pending_integration_source_cursor.clone())
            .await?;
        Ok(())
    }

    async fn refresh_integration_runtime_config_from_remote(&mut self) -> Result<(), String> {
        let mut backend = Self::detached(self.pool.clone(), self.names.clone());
        backend.load_integration_runtime_config().await?;
        self.integration_runtime_config = backend.integration_runtime_config;
        Ok(())
    }

    async fn refresh_system_events_from_remote(&mut self) -> Result<(), String> {
        let (system_event_index, system_event_source_cursor) = self
            .load_system_event_index_snapshot(
                Arc::clone(&self.system_event_index_snapshot_cache),
                self.system_event_source_cursor.clone(),
            )
            .await?;
        self.system_event_index_snapshot_cache = system_event_index;
        self.system_event_source_cursor = system_event_source_cursor;
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

    async fn load_collections(&mut self) -> Result<(), String> {
        let collections = sqlx::query_as::<_, (String, String, Option<String>)>(&format!(
            "SELECT collection_key, name, context_profile_key FROM {} ORDER BY created_at, collection_key",
            self.names.collections
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres collections load failed: {error}"))?;
        for (collection_key, name, context_profile_key) in collections {
            let collection_key_boxed = collection_key.clone();
            let result = self
                .ingestion
                .inventory_mut()
                .register_collection(CollectionRegistration::new(collection_key, name));
            if result.change == RegisterCollectionChange::Rejected {
                return Err("postgres collections contain conflicting registration".to_owned());
            }
            if let Some(profile_key) = context_profile_key {
                let result = self
                    .ingestion
                    .inventory_mut()
                    .assign_context_profile_for_collection(&collection_key_boxed, &profile_key);
                if result.change == AssignCollectionContextProfileChange::Rejected {
                    return Err(
                        "postgres collections contain invalid context assignment".to_owned()
                    );
                }
            }
        }
        Ok(())
    }

    async fn load_collection_sources(&mut self) -> Result<(), String> {
        let sources = sqlx::query_as::<_, (String, String, String, Json<Vec<String>>)>(&format!(
            concat!(
                "SELECT collection_key, source_kind, mode, component_keys FROM {} ",
                "ORDER BY collection_key"
            ),
            self.names.collection_sources
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres collection sources load failed: {error}"))?;
        for (collection_key, source_kind, mode, Json(component_keys)) in sources {
            let source = parse_collection_source(
                &source_kind,
                &mode,
                component_keys
                    .into_iter()
                    .map(String::into_boxed_str)
                    .collect::<Vec<_>>(),
            )?;
            let result = self
                .ingestion
                .inventory_mut()
                .configure_collection_source(&collection_key, source);
            if result.change == ConfigureCollectionSourceChange::Rejected {
                return Err("postgres collection sources contain invalid configuration".to_owned());
            }
        }
        Ok(())
    }

    async fn load_context_profiles(&mut self) -> Result<(), String> {
        let profiles = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<bool>,
                Option<bool>,
                Option<bool>,
                Option<bool>,
                Option<bool>,
            ),
        >(&format!(
            concat!(
                "SELECT profile_key, name, internet_exposed, production, mission_critical, vpn_restricted, non_privileged_user ",
                "FROM {} ORDER BY created_at, profile_key"
            ),
            self.names.context_profiles
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres context profiles load failed: {error}"))?;
        for (
            profile_key,
            name,
            internet_exposed,
            production,
            mission_critical,
            vpn_restricted,
            non_privileged_user,
        ) in profiles
        {
            let mut registration = ContextProfileRegistration::overlay(profile_key, name);
            if let Some(value) = internet_exposed {
                registration = registration.with_internet_exposed(value);
            }
            if let Some(value) = production {
                registration = registration.with_production(value);
            }
            if let Some(value) = mission_critical {
                registration = registration.with_mission_critical(value);
            }
            if let Some(value) = vpn_restricted {
                registration = registration.with_vpn_restricted(value);
            }
            if let Some(value) = non_privileged_user {
                registration = registration.with_non_privileged_user(value);
            }
            let result = self
                .ingestion
                .inventory_mut()
                .register_context_profile(registration);
            if result.change == RegisterContextProfileChange::Rejected {
                return Err("postgres context profiles contain conflicting registration".to_owned());
            }
        }
        Ok(())
    }

    async fn load_component_context_profiles(&mut self) -> Result<(), String> {
        let assignments = sqlx::query_as::<_, (String, String)>(&format!(
            concat!("SELECT component_key, profile_key FROM {} ORDER BY component_key"),
            self.names.component_context_profiles
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres component context profiles load failed: {error}"))?;
        for (component_key, profile_key) in assignments {
            let result = self
                .ingestion
                .inventory_mut()
                .assign_context_profile(&component_key, &profile_key);
            if result.change == AssignContextProfileChange::Rejected {
                return Err(
                    "postgres component context profiles contain invalid assignment".to_owned(),
                );
            }
        }
        Ok(())
    }

    async fn load_component_tags(&mut self) -> Result<(), String> {
        let tags = sqlx::query_as::<_, (String, String, Option<String>)>(&format!(
            "SELECT tag_key, name, context_profile_key FROM {} ORDER BY created_at, tag_key",
            self.names.component_tags
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres component tags load failed: {error}"))?;
        for (tag_key, name, context_profile_key) in tags {
            let tag_key_boxed = tag_key.clone();
            let result = self
                .ingestion
                .inventory_mut()
                .register_component_tag(ComponentTagRegistration::new(tag_key, name));
            if result.change == RegisterComponentTagChange::Rejected {
                return Err("postgres component tags contain conflicting registration".to_owned());
            }
            if let Some(profile_key) = context_profile_key {
                let result = self
                    .ingestion
                    .inventory_mut()
                    .assign_context_profile_for_tag(&tag_key_boxed, &profile_key);
                if result.change == AssignTagContextProfileChange::Rejected {
                    return Err(
                        "postgres component tags contain invalid context assignment".to_owned()
                    );
                }
            }
        }
        Ok(())
    }

    async fn load_component_tag_memberships(&mut self) -> Result<(), String> {
        let memberships = sqlx::query_as::<_, (String, String)>(&format!(
            concat!(
                "SELECT tag_key, component_key FROM {} ",
                "ORDER BY created_at, tag_key, component_key"
            ),
            self.names.component_tag_memberships
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres component tag memberships load failed: {error}"))?;
        for (tag_key, component_key) in memberships {
            let result = self
                .ingestion
                .inventory_mut()
                .assign_component_tag(&tag_key, &component_key);
            if result.change == AssignComponentTagChange::Rejected {
                return Err(
                    "postgres component tag memberships contain invalid ownership".to_owned(),
                );
            }
        }
        Ok(())
    }

    async fn load_collection_memberships(&mut self) -> Result<(), String> {
        let memberships = sqlx::query_as::<_, (String, String)>(&format!(
            concat!(
                "SELECT collection_key, component_key FROM {} ",
                "ORDER BY created_at, collection_key, component_key"
            ),
            self.names.collection_memberships
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres collection memberships load failed: {error}"))?;
        for (collection_key, component_key) in memberships {
            let result = self
                .ingestion
                .inventory_mut()
                .add_component_to_collection(&collection_key, &component_key);
            if result.change == venom_domain::AddCollectionComponentChange::Rejected {
                return Err("postgres collection memberships contain invalid ownership".to_owned());
            }
        }
        Ok(())
    }

    async fn load_collection_scan_schedules(&mut self) -> Result<(), String> {
        let schedules = sqlx::query_as::<_, (String, i32, String, i64, Option<i64>, Option<i32>)>(&format!(
            concat!(
                "SELECT collection_key, cadence_minutes, freshness, next_due_at_unix_ms, last_materialized_at_unix_ms, last_enqueued_commands ",
                "FROM {} ORDER BY collection_key"
            ),
            self.names.collection_scan_schedules
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres collection scan schedules load failed: {error}"))?;
        for (
            collection_key,
            cadence_minutes,
            freshness,
            next_due_at_unix_ms,
            last_materialized_at_unix_ms,
            last_enqueued_commands,
        ) in schedules
        {
            let result = self
                .ingestion
                .inventory_mut()
                .configure_collection_scan_schedule(
                    &collection_key,
                    u32::try_from(cadence_minutes)
                        .map_err(|_| "postgres schedule cadence must be positive".to_owned())?,
                    parse_freshness(&freshness)?,
                    u64::try_from(next_due_at_unix_ms)
                        .map_err(|_| "postgres schedule next due must be positive".to_owned())?,
                );
            if result.change == ConfigureCollectionScanScheduleChange::Rejected {
                return Err(
                    "postgres collection scan schedules contain invalid configuration".to_owned(),
                );
            }
            if let Some(materialized_at) = last_materialized_at_unix_ms {
                let materialized_result = self
                    .ingestion
                    .inventory_mut()
                    .record_collection_scan_materialization(
                        &collection_key,
                        u64::try_from(next_due_at_unix_ms).map_err(|_| {
                            "postgres schedule next due must be positive".to_owned()
                        })?,
                        u64::try_from(materialized_at).map_err(|_| {
                            "postgres schedule materialized time must be positive".to_owned()
                        })?,
                        u32::try_from(last_enqueued_commands.unwrap_or_default()).map_err(
                            |_| "postgres schedule command count must fit u32".to_owned(),
                        )?,
                    );
                if materialized_result.change == ConfigureCollectionScanScheduleChange::Rejected {
                    return Err(
                        "postgres collection scan materializations contain invalid state"
                            .to_owned(),
                    );
                }
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
                i64,
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
                "SELECT id, provider_key, component_key, artifact_kind, artifact_identity, ",
                "observed_at_micros, freshness, knowledge_revision, findings ",
                "FROM {} ORDER BY id"
            ),
            self.names.provider_reports
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres provider reports load failed: {error}"))?;
        for (
            id,
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
            self.apply_provider_report_row(id, &report)?;
        }

        Ok(())
    }

    async fn load_provider_reports_after(&mut self, after_id: u64) -> Result<(), String> {
        let reports = sqlx::query_as::<
            _,
            (
                i64,
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
                "SELECT id, provider_key, component_key, artifact_kind, artifact_identity, ",
                "observed_at_micros, freshness, knowledge_revision, findings ",
                "FROM {} WHERE id > $1 ORDER BY id"
            ),
            self.names.provider_reports
        ))
        .bind(
            i64::try_from(after_id)
                .map_err(|_| "postgres provider report cursor out of range".to_owned())?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres provider report delta load failed: {error}"))?;
        for (
            id,
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
            self.apply_provider_report_row(id, &report)?;
        }
        Ok(())
    }

    fn apply_provider_report_row(
        &mut self,
        id: i64,
        report: &ProviderScanReport,
    ) -> Result<(), String> {
        self.ingestion.replay_scan_report(report).map_err(|error| {
            format!("postgres provider report replay failed: {}", error.as_str())
        })?;
        self.read_model_mut().record_scan_report(report);
        self.provider_report_row_high_watermark = self.provider_report_row_high_watermark.max(
            u64::try_from(id).map_err(|_| "postgres provider report id out of range".to_owned())?,
        );
        Ok(())
    }

    async fn load_finding_risk_acceptances(&mut self) -> Result<(), String> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<i64>,
            ),
        >(&format!(
            concat!(
                "SELECT component_key, artifact_kind, artifact_identity, vulnerability_id, ",
                "package_name, package_version, package_purl, reason, until_unix_ms ",
                "FROM {} ORDER BY created_at"
            ),
            self.names.finding_risk_acceptances
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres finding risk acceptances load failed: {error}"))?;

        for (
            component_key,
            artifact_kind,
            artifact_identity,
            vulnerability_id,
            package_name,
            package_version,
            package_purl,
            reason,
            until_unix_ms,
        ) in rows
        {
            let finding = FindingRef::new(
                component_key,
                ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
                vulnerability_id,
                venom_domain::PackageCoordinate {
                    name: package_name.into_boxed_str(),
                    version: package_version.into_boxed_str(),
                    purl: (!package_purl.is_empty()).then(|| package_purl.into_boxed_str()),
                },
            );
            let acceptance = match until_unix_ms {
                Some(until_unix_ms) => RiskAcceptance::new(reason).until_unix_ms(
                    u64::try_from(until_unix_ms).map_err(|_| {
                        "postgres finding risk acceptance until must be positive".to_owned()
                    })?,
                ),
                None => RiskAcceptance::new(reason),
            };
            self.governance
                .replay_risk_acceptance(finding.clone(), acceptance.clone());
            self.read_model_mut()
                .replay_risk_acceptance(finding, acceptance);
        }

        Ok(())
    }

    async fn load_finding_suppressions(&mut self) -> Result<(), String> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
            ),
        >(&format!(
            concat!(
                "SELECT component_key, artifact_kind, artifact_identity, vulnerability_id, ",
                "package_name, package_version, package_purl, reason ",
                "FROM {} ORDER BY created_at"
            ),
            self.names.finding_suppressions
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres finding suppressions load failed: {error}"))?;

        for (
            component_key,
            artifact_kind,
            artifact_identity,
            vulnerability_id,
            package_name,
            package_version,
            package_purl,
            reason,
        ) in rows
        {
            let finding = FindingRef::new(
                component_key,
                ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
                vulnerability_id,
                venom_domain::PackageCoordinate {
                    name: package_name.into_boxed_str(),
                    version: package_version.into_boxed_str(),
                    purl: (!package_purl.is_empty()).then(|| package_purl.into_boxed_str()),
                },
            );
            let suppression = Suppression::new(reason);
            self.governance
                .replay_suppression(finding.clone(), suppression.clone());
            self.read_model_mut()
                .replay_suppression(finding, suppression);
        }

        Ok(())
    }

    async fn load_governance_journal_after(
        &mut self,
        after_id: u64,
        apply_to_governance: bool,
    ) -> Result<(), String> {
        let rows = sqlx::query_as::<_, GovernanceJournalRow>(&format!(
            concat!(
                "SELECT id, component_key, artifact_kind, artifact_identity, vulnerability_id, ",
                "package_name, package_version, package_purl, decision_kind, reason, until_unix_ms ",
                "FROM {} WHERE id > $1 ORDER BY id"
            ),
            self.names.finding_governance_journal
        ))
        .bind(
            i64::try_from(after_id)
                .map_err(|_| "postgres governance journal cursor out of range".to_owned())?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres governance journal load failed: {error}"))?;

        for row in rows {
            self.apply_governance_journal_row(row, apply_to_governance)?;
        }
        Ok(())
    }

    async fn load_governance_source_watermark(&self) -> Result<u64, String> {
        let max_id = sqlx::query_scalar::<_, Option<i64>>(&format!(
            "SELECT MAX(id) FROM {}",
            self.names.finding_governance_journal
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres governance journal watermark read failed: {error}"))?
        .unwrap_or_default();
        u64::try_from(max_id)
            .map_err(|_| "postgres governance journal watermark out of range".to_owned())
    }

    async fn load_system_event_index_snapshot(
        &self,
        base_system_event_index: Arc<SystemEventQueryIndex>,
        base_system_event_source_cursor: EventSourceCursor,
    ) -> Result<(Arc<SystemEventQueryIndex>, EventSourceCursor), String> {
        let rows = sqlx::query_as::<_, SystemEventDeltaRow>(&format!(
            concat!(
                "SELECT event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail, created_at_micros ",
                "FROM (",
                "SELECT event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail, ",
                "(EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_micros ",
                "FROM {}",
                ") delta ",
                "WHERE created_at_micros > $1 OR (created_at_micros = $1 AND event_id > $2) ",
                "ORDER BY created_at_micros DESC, event_id DESC"
            ),
            self.names.system_events
        ))
        .bind(
            i64::try_from(base_system_event_source_cursor.unix_micros)
                .map_err(|_| "postgres system event cursor out of range".to_owned())?,
        )
        .bind(
            base_system_event_source_cursor
                .tie_breaker
                .as_deref()
                .unwrap_or(""),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres system event delta load failed: {error}"))?;

        if rows.is_empty() {
            return Ok((base_system_event_index, base_system_event_source_cursor));
        }

        let mut cursor = base_system_event_source_cursor;
        let mut delta_events = Vec::with_capacity(rows.len());
        for row in rows {
            let (
                event_id,
                occurred_at_unix_ms,
                category,
                kind,
                collection_key,
                component_key,
                command_id,
                integration_event_id,
                finding_count,
                retryable,
                detail,
                created_at_micros,
            ) = row;
            cursor = max_event_source_cursor(
                &cursor,
                event_source_cursor(
                    u64::try_from(created_at_micros)
                        .map_err(|_| "postgres system event created_at out of range".to_owned())?,
                    event_id.clone().into_boxed_str(),
                ),
            );
            delta_events.push(parse_system_event_row((
                event_id,
                occurred_at_unix_ms,
                category,
                kind,
                collection_key,
                component_key,
                command_id,
                integration_event_id,
                finding_count,
                retryable,
                detail,
            ))?);
        }

        let delta_index = SystemEventQueryIndex::from_newest_first(delta_events.iter());
        Ok((
            Arc::new(SystemEventQueryIndex::merged(
                base_system_event_index.as_ref(),
                &delta_index,
            )),
            cursor,
        ))
    }

    fn apply_governance_journal_row(
        &mut self,
        row: GovernanceJournalRow,
        apply_to_governance: bool,
    ) -> Result<(), String> {
        let (
            id,
            component_key,
            artifact_kind,
            artifact_identity,
            vulnerability_id,
            package_name,
            package_version,
            package_purl,
            decision_kind,
            reason,
            until_unix_ms,
        ) = row;
        let finding = FindingRef::new(
            component_key,
            ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
            vulnerability_id,
            venom_domain::PackageCoordinate {
                name: package_name.into_boxed_str(),
                version: package_version.into_boxed_str(),
                purl: (!package_purl.is_empty()).then(|| package_purl.into_boxed_str()),
            },
        );

        match decision_kind.as_str() {
            "risk-accepted" => {
                let acceptance = match until_unix_ms {
                    Some(until_unix_ms) => RiskAcceptance::new(reason.ok_or_else(|| {
                        "postgres governance journal risk acceptance missing reason".to_owned()
                    })?)
                    .until_unix_ms(
                        u64::try_from(until_unix_ms).map_err(|_| {
                            "postgres governance journal acceptance until out of range"
                                .to_owned()
                        })?,
                    ),
                    None => RiskAcceptance::new(reason.ok_or_else(|| {
                        "postgres governance journal risk acceptance missing reason".to_owned()
                    })?),
                };
                if apply_to_governance {
                    self.governance
                        .replay_risk_acceptance(finding.clone(), acceptance.clone());
                }
                self.read_model_mut().replay_risk_acceptance(finding, acceptance);
            }
            "suppressed" => {
                let suppression = Suppression::new(reason.ok_or_else(|| {
                    "postgres governance journal suppression missing reason".to_owned()
                })?);
                if apply_to_governance {
                    self.governance
                        .replay_suppression(finding.clone(), suppression.clone());
                }
                self.read_model_mut().replay_suppression(finding, suppression);
            }
            "reopened" => {
                if apply_to_governance {
                    self.governance.replay_reopen(&finding);
                }
                self.read_model_mut().replay_reopen(&finding);
            }
            _ => {
                return Err(format!(
                    "postgres governance journal contains unsupported decision kind: {decision_kind}"
                ));
            }
        }

        self.governance_journal_high_watermark = self.governance_journal_high_watermark.max(
            u64::try_from(id)
                .map_err(|_| "postgres governance journal id out of range".to_owned())?,
        );
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

    async fn load_integration_runtime_config(&mut self) -> Result<(), String> {
        let config = sqlx::query_as::<_, (String, Option<String>, Option<i64>)>(&format!(
            concat!(
                "SELECT publisher_key, endpoint_url, timeout_ms ",
                "FROM {} WHERE id = 1"
            ),
            self.names.integration_runtime_config
        ))
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| format!("postgres integration runtime config load failed: {error}"))?;

        self.integration_runtime_config = match config {
            None => None,
            Some((publisher_key, endpoint_url, timeout_ms)) => Some(
                parse_integration_runtime_config_row(&publisher_key, endpoint_url, timeout_ms)?,
            ),
        };
        Ok(())
    }

    async fn load_scan_commands(&mut self) -> Result<(), String> {
        let commands = sqlx::query_as::<_, (String, String, String, String, String, String, i64)>(&format!(
            concat!(
                "SELECT command_id, component_key, artifact_kind, artifact_identity, freshness, status, ",
                "(EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_micros ",
                "FROM {} ORDER BY order_id"
            ),
            self.names.scan_commands
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres scan commands load failed: {error}"))?;
        for (
            command_id,
            component_key,
            artifact_kind,
            artifact_identity,
            freshness,
            status,
            updated_at_micros,
        ) in commands
        {
            let command_id = command_id.into_boxed_str();
            let status = parse_scan_command_status(&status)?;
            let request = ScanRequest::new(
                component_key,
                ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
                parse_freshness(&freshness)?,
            );
            Arc::make_mut(&mut self.order).push(command_id.clone());
            self.set_command_status_snapshot(command_id.as_ref(), status);
            self.command_status_source_cursor = max_row_source_cursor(
                &self.command_status_source_cursor,
                row_source_cursor(
                    u64::try_from(updated_at_micros)
                        .map_err(|_| "postgres scan command updated_at out of range".to_owned())?,
                    command_id.clone(),
                ),
            );
            Arc::make_mut(&mut self.commands)
                .insert(command_id, ScanCommandRecord { request, status });
        }

        Ok(())
    }

    async fn load_scan_commands_after(&mut self, base_cursor: RowSourceCursor) -> Result<(), String> {
        let commands = sqlx::query_as::<_, (String, String, String, String, String, String, i64)>(&format!(
            concat!(
                "SELECT command_id, component_key, artifact_kind, artifact_identity, freshness, status, ",
                "(EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_micros ",
                "FROM {} WHERE (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint > $1 ",
                "OR ((EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint = $1 AND command_id > $2) ",
                "ORDER BY (EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint ASC, command_id ASC"
            ),
            self.names.scan_commands
        ))
        .bind(
            i64::try_from(base_cursor.unix_micros)
                .map_err(|_| "postgres scan command cursor out of range".to_owned())?,
        )
        .bind(base_cursor.tie_breaker.as_deref().unwrap_or(""))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres scan command delta load failed: {error}"))?;

        if commands.is_empty() {
            self.command_status_source_cursor = base_cursor;
            return Ok(());
        }

        let mut cursor = base_cursor;
        let mut order = Arc::unwrap_or_clone(Arc::clone(&self.order));
        let mut command_statuses = Arc::unwrap_or_clone(Arc::clone(&self.command_statuses_snapshot_cache));
        let mut records = Arc::unwrap_or_clone(Arc::clone(&self.commands));
        for (
            command_id,
            component_key,
            artifact_kind,
            artifact_identity,
            freshness,
            status,
            updated_at_micros,
        ) in commands {
            let command_id = command_id.into_boxed_str();
            let status = parse_scan_command_status(&status)?;
            let request = ScanRequest::new(
                component_key,
                ArtifactRef::new(parse_artifact_kind(&artifact_kind)?, artifact_identity),
                parse_freshness(&freshness)?,
            );
            if !records.contains_key(command_id.as_ref()) {
                order.push(command_id.clone());
            }
            command_statuses.insert(command_id.clone(), status);
            records.insert(command_id.clone(), ScanCommandRecord { request, status });
            cursor = max_row_source_cursor(
                &cursor,
                row_source_cursor(
                    u64::try_from(updated_at_micros)
                        .map_err(|_| "postgres scan command updated_at out of range".to_owned())?,
                    command_id,
                ),
            );
        }

        self.order = Arc::new(order);
        self.command_statuses_snapshot_cache = Arc::new(command_statuses);
        self.commands = Arc::new(records);
        self.command_status_source_cursor = cursor;
        Ok(())
    }

    async fn load_pending_integration_events(&mut self) -> Result<(), String> {
        let events = sqlx::query_as::<_, (Json<PendingIntegrationEvent>, i64, i64)>(&format!(
            concat!(
                "SELECT payload, order_id, ",
                "GREATEST(COALESCE(published_at_micros, 0), COALESCE(last_attempted_at_micros, 0), (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint) AS updated_at_micros ",
                "FROM {} WHERE publication_status = 'pending' ORDER BY order_id"
            ),
            self.names.integration_outbox
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres integration outbox load failed: {error}"))?;
        let mut cursor = RowSourceCursor::default();
        let pending = events
            .into_iter()
            .map(|(payload, order_id, updated_at_micros)| {
                cursor = max_row_source_cursor(
                    &cursor,
                    row_source_cursor(
                        u64::try_from(updated_at_micros).map_err(|_| {
                            "postgres integration outbox updated_at out of range".to_owned()
                        })?,
                        payload.0.event_id.clone(),
                    ),
                );
                let _ = order_id;
                Ok(payload.0)
            })
            .collect::<Result<Vec<_>, String>>()?;
        self.pending_integration_events = Arc::new(pending);
        self.pending_integration_source_cursor = cursor;
        Ok(())
    }

    async fn load_pending_integration_events_after(
        &mut self,
        base_cursor: RowSourceCursor,
    ) -> Result<(), String> {
        let rows = sqlx::query_as::<_, (String, Json<PendingIntegrationEvent>, String, i64)>(&format!(
            concat!(
                "SELECT event_id, payload, publication_status, updated_at_micros FROM (",
                "SELECT event_id, payload, publication_status, ",
                "GREATEST(COALESCE(published_at_micros, 0), COALESCE(last_attempted_at_micros, 0), ",
                "(EXTRACT(EPOCH FROM created_at) * 1000000)::bigint) AS updated_at_micros ",
                "FROM {}",
                ") delta WHERE updated_at_micros > $1 OR (updated_at_micros = $1 AND event_id > $2) ",
                "ORDER BY updated_at_micros ASC, event_id ASC"
            ),
            self.names.integration_outbox
        ))
        .bind(
            i64::try_from(base_cursor.unix_micros)
                .map_err(|_| "postgres integration outbox cursor out of range".to_owned())?,
        )
        .bind(base_cursor.tie_breaker.as_deref().unwrap_or(""))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres integration outbox delta load failed: {error}"))?;

        if rows.is_empty() {
            self.pending_integration_source_cursor = base_cursor;
            return Ok(());
        }

        let mut cursor = base_cursor;
        let mut pending = Arc::unwrap_or_clone(Arc::clone(&self.pending_integration_events));
        for (event_id, payload, publication_status, updated_at_micros) in rows {
            cursor = max_row_source_cursor(
                &cursor,
                row_source_cursor(
                    u64::try_from(updated_at_micros).map_err(|_| {
                        "postgres integration outbox updated_at out of range".to_owned()
                    })?,
                    event_id.clone().into_boxed_str(),
                ),
            );
            match publication_status.as_str() {
                "pending" => {
                    if let Some(existing) = pending
                        .iter_mut()
                        .find(|event| event.event_id.as_ref() == event_id.as_str())
                    {
                        *existing = payload.0;
                    } else {
                        pending.push(payload.0);
                    }
                }
                _ => pending.retain(|event| event.event_id.as_ref() != event_id.as_str()),
            }
        }

        self.pending_integration_events = Arc::new(pending);
        self.pending_integration_source_cursor = cursor;
        Ok(())
    }

    async fn load_system_events(&mut self) -> Result<(), String> {
        let totals = self.load_system_event_totals().await?;
        let windows = self.load_recent_system_event_windows().await?;
        let cursor = self.load_latest_system_event_cursor().await?;

        self.system_event_index_snapshot_cache =
            Arc::new(SystemEventQueryIndex::from_recent_windows(
                SystemEventWindowTotals {
                    total: totals.total,
                    scheduler_total: totals.scheduler_total,
                    command_total: totals.command_total,
                    governance_total: totals.governance_total,
                    publication_total: totals.publication_total,
                },
                windows,
            ));
        self.system_event_source_cursor = cursor;
        Ok(())
    }

    async fn load_system_event_totals(&self) -> Result<SystemEventTotals, String> {
        let rows = sqlx::query_as::<_, (String, i64)>(&format!(
            "SELECT category, COUNT(*)::bigint FROM {} GROUP BY category",
            self.names.system_events
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres system event totals load failed: {error}"))?;

        let mut totals = SystemEventTotals::default();
        for (category, count) in rows {
            let count = usize::try_from(count)
                .map_err(|_| "postgres system event total out of range".to_owned())?;
            totals.total += count;
            match category.as_str() {
                "scheduler" => totals.scheduler_total = count,
                "command" => totals.command_total = count,
                "governance" => totals.governance_total = count,
                "publication" => totals.publication_total = count,
                _ => {
                    return Err(format!(
                        "postgres system events contain unknown category: {category}"
                    ));
                }
            }
        }
        Ok(totals)
    }

    async fn load_recent_system_event_windows(&self) -> Result<SystemEventRecentWindows, String> {
        let limit = i64::try_from(MAX_SYSTEM_EVENTS_LIMIT).expect("system event limit fits in i64");
        let rows = sqlx::query_as::<_, SystemEventWindowRow>(&format!(
            concat!(
                "SELECT event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail, global_rank, category_rank ",
                "FROM (",
                "SELECT event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail, ",
                "ROW_NUMBER() OVER (ORDER BY occurred_at_unix_ms DESC, event_id DESC) AS global_rank, ",
                "ROW_NUMBER() OVER (PARTITION BY category ORDER BY occurred_at_unix_ms DESC, event_id DESC) AS category_rank ",
                "FROM {}",
                ") ranked ",
                "WHERE global_rank <= $1 OR category_rank <= $1 ",
                "ORDER BY occurred_at_unix_ms DESC, event_id DESC"
            ),
            self.names.system_events
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres recent system events load failed: {error}"))?;

        let mut windows = SystemEventRecentWindows::default();
        for row in rows {
            let (
                event_id,
                occurred_at_unix_ms,
                category,
                kind,
                collection_key,
                component_key,
                command_id,
                integration_event_id,
                finding_count,
                retryable,
                detail,
                global_rank,
                category_rank,
            ) = row;
            let event = Arc::new(parse_system_event_row((
                event_id,
                occurred_at_unix_ms,
                category.clone(),
                kind,
                collection_key,
                component_key,
                command_id,
                integration_event_id,
                finding_count,
                retryable,
                detail,
            ))?);
            if global_rank <= limit {
                windows.recent_events.push(event.clone());
            }
            match category.as_str() {
                "scheduler" => {
                    if category_rank <= limit {
                        windows.recent_scheduler_events.push(event.clone());
                    }
                }
                "command" => {
                    if category_rank <= limit {
                        windows.recent_command_events.push(event.clone());
                    }
                }
                "governance" => {
                    if category_rank <= limit {
                        windows.recent_governance_events.push(event.clone());
                    }
                }
                "publication" => {
                    if category_rank <= limit {
                        windows.recent_publication_events.push(event.clone());
                    }
                }
                _ => {
                    return Err(format!(
                        "postgres system events contain unknown category: {category}"
                    ));
                }
            }
        }
        Ok(windows)
    }

    async fn load_latest_system_event_cursor(&self) -> Result<EventSourceCursor, String> {
        sqlx::query_as::<_, (i64, String)>(&format!(
            concat!(
                "SELECT (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_micros, event_id ",
                "FROM {} ORDER BY created_at DESC, event_id DESC LIMIT 1"
            ),
            self.names.system_events
        ))
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| format!("postgres latest system event cursor load failed: {error}"))
        .and_then(|row| match row {
            Some((created_at_micros, event_id)) => Ok(event_source_cursor(
                u64::try_from(created_at_micros)
                    .map_err(|_| "postgres system event created_at out of range".to_owned())?,
                event_id.into_boxed_str(),
            )),
            None => Ok(EventSourceCursor::default()),
        })
    }

    fn remove_pending_integration_event(&mut self, event_id: &str) {
        if let Some(index) = self
            .pending_integration_events
            .iter()
            .position(|event| event.event_id.as_ref() == event_id)
        {
            Arc::make_mut(&mut self.pending_integration_events).remove(index);
        }
    }

    fn push_system_event(&mut self, event: SystemEvent) {
        Arc::make_mut(&mut self.system_event_index_snapshot_cache).push_newest(event);
    }

    fn refresh_read_snapshot_caches(&mut self) {
        self.refresh_inventory_snapshot_cache();
        self.refresh_read_model_snapshot_cache();
    }

    fn refresh_inventory_and_release_board_snapshot_caches(&mut self) {
        self.refresh_inventory_snapshot_cache();
    }

    fn refresh_read_model_and_release_board_snapshot_caches(&mut self) {
        self.refresh_read_model_snapshot_cache();
    }

    fn refresh_inventory_snapshot_cache(&mut self) {
        self.inventory_snapshot_cache = self.ingestion.inventory_arc();
    }

    fn refresh_read_model_snapshot_cache(&mut self) {
        self.read_model_snapshot_cache = self.read_model_arc();
    }
    fn set_command_status_snapshot(&mut self, command_id: &str, status: ScanCommandStatus) {
        Arc::make_mut(&mut self.command_statuses_snapshot_cache).insert(command_id.into(), status);
    }

    async fn insert_system_event(&self, event: &SystemEvent) -> Result<EventSourceCursor, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| format!("postgres system event begin failed: {error}"))?;
        let cursor = self
            .insert_system_event_in_transaction(&mut tx, event)
            .await?;
        tx.commit()
            .await
            .map_err(|error| format!("postgres system event commit failed: {error}"))?;
        Ok(cursor)
    }

    async fn insert_system_event_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &SystemEvent,
    ) -> Result<EventSourceCursor, String> {
        sqlx::query_as::<_, (i64, String)>(&format!(
            concat!(
                "INSERT INTO {} (event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ",
                "RETURNING (EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_micros, event_id"
            ),
            self.names.system_events
        ))
        .bind(event.event_id.as_ref())
        .bind(i64::try_from(event.occurred_at_unix_ms).map_err(|_| {
            "system event occurred_at_unix_ms does not fit postgres".to_owned()
        })?)
        .bind(event.kind.category().as_str())
        .bind(event.kind.as_str())
        .bind(event.collection_key.as_deref())
        .bind(event.component_key.as_deref())
        .bind(event.command_id.as_deref())
        .bind(event.integration_event_id.as_deref())
        .bind(
            event.finding_count
                .map(i32::try_from)
                .transpose()
                .map_err(|_| "system event finding count does not fit postgres".to_owned())?,
        )
        .bind(event.retryable)
        .bind(event.detail.as_deref())
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| format!("postgres system event insert failed: {error}"))
        .and_then(|(created_at_micros, event_id)| {
            Ok(event_source_cursor(
                u64::try_from(created_at_micros)
                    .map_err(|_| "postgres system event created_at out of range".to_owned())?,
                event_id.into_boxed_str(),
            ))
        })
    }

    async fn upsert_risk_acceptance_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        finding: &FindingRef,
        acceptance: &RiskAcceptance,
    ) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl, reason, until_unix_ms) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ",
                "ON CONFLICT (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl) ",
                "DO UPDATE SET reason = EXCLUDED.reason, until_unix_ms = EXCLUDED.until_unix_ms, updated_at = NOW()"
            ),
            self.names.finding_risk_acceptances
        ))
        .bind(finding.component_key.as_ref())
        .bind(artifact_kind_name(finding.artifact.kind))
        .bind(finding.artifact.identity.as_ref())
        .bind(finding.vulnerability_id.as_ref())
        .bind(finding.package.name.as_ref())
        .bind(finding.package.version.as_ref())
        .bind(finding.package.purl.as_deref().unwrap_or(""))
        .bind(acceptance.reason.as_ref())
        .bind(
            acceptance
                .until_unix_ms
                .map(i64::try_from)
                .transpose()
                .map_err(|_| "risk acceptance until overflow".to_owned())?,
        )
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("postgres finding risk acceptance upsert failed: {error}"))?;
        Ok(())
    }

    async fn upsert_risk_acceptances_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        findings: &[FindingRef],
        acceptance: &RiskAcceptance,
    ) -> Result<(), String> {
        if findings.is_empty() {
            return Ok(());
        }

        let until_unix_ms = acceptance
            .until_unix_ms
            .map(i64::try_from)
            .transpose()
            .map_err(|_| "risk acceptance until overflow".to_owned())?;
        let mut query = QueryBuilder::<Postgres>::new(format!(
            "INSERT INTO {} \
            (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl, reason, until_unix_ms) ",
            self.names.finding_risk_acceptances
        ));
        query.push_values(findings, |mut row, finding| {
            row.push_bind(finding.component_key.as_ref())
                .push_bind(artifact_kind_name(finding.artifact.kind))
                .push_bind(finding.artifact.identity.as_ref())
                .push_bind(finding.vulnerability_id.as_ref())
                .push_bind(finding.package.name.as_ref())
                .push_bind(finding.package.version.as_ref())
                .push_bind(finding.package.purl.as_deref().unwrap_or(""))
                .push_bind(acceptance.reason.as_ref())
                .push_bind(until_unix_ms);
        });
        query.push(
            " ON CONFLICT (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl) \
            DO UPDATE SET reason = EXCLUDED.reason, until_unix_ms = EXCLUDED.until_unix_ms, updated_at = NOW()",
        );
        query.build().execute(&mut **tx).await.map_err(|error| {
            format!("postgres finding risk acceptance batch upsert failed: {error}")
        })?;
        Ok(())
    }

    async fn upsert_suppression_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        finding: &FindingRef,
        suppression: &Suppression,
    ) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl, reason) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ",
                "ON CONFLICT (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl) ",
                "DO UPDATE SET reason = EXCLUDED.reason, updated_at = NOW()"
            ),
            self.names.finding_suppressions
        ))
        .bind(finding.component_key.as_ref())
        .bind(artifact_kind_name(finding.artifact.kind))
        .bind(finding.artifact.identity.as_ref())
        .bind(finding.vulnerability_id.as_ref())
        .bind(finding.package.name.as_ref())
        .bind(finding.package.version.as_ref())
        .bind(finding.package.purl.as_deref().unwrap_or(""))
        .bind(suppression.reason.as_ref())
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("postgres finding suppression upsert failed: {error}"))?;
        Ok(())
    }

    async fn upsert_suppressions_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        findings: &[FindingRef],
        suppression: &Suppression,
    ) -> Result<(), String> {
        if findings.is_empty() {
            return Ok(());
        }

        let mut query = QueryBuilder::<Postgres>::new(format!(
            "INSERT INTO {} \
            (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl, reason) ",
            self.names.finding_suppressions
        ));
        query.push_values(findings, |mut row, finding| {
            row.push_bind(finding.component_key.as_ref())
                .push_bind(artifact_kind_name(finding.artifact.kind))
                .push_bind(finding.artifact.identity.as_ref())
                .push_bind(finding.vulnerability_id.as_ref())
                .push_bind(finding.package.name.as_ref())
                .push_bind(finding.package.version.as_ref())
                .push_bind(finding.package.purl.as_deref().unwrap_or(""))
                .push_bind(suppression.reason.as_ref());
        });
        query.push(
            " ON CONFLICT (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl) \
            DO UPDATE SET reason = EXCLUDED.reason, updated_at = NOW()",
        );
        query.build().execute(&mut **tx).await.map_err(|error| {
            format!("postgres finding suppression batch upsert failed: {error}")
        })?;
        Ok(())
    }

    async fn append_governance_journal_entry_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        finding: &FindingRef,
        decision_kind: &str,
        reason: Option<&str>,
        until_unix_ms: Option<u64>,
    ) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} ",
                "(component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl, decision_kind, reason, until_unix_ms) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            ),
            self.names.finding_governance_journal
        ))
        .bind(finding.component_key.as_ref())
        .bind(artifact_kind_name(finding.artifact.kind))
        .bind(finding.artifact.identity.as_ref())
        .bind(finding.vulnerability_id.as_ref())
        .bind(finding.package.name.as_ref())
        .bind(finding.package.version.as_ref())
        .bind(finding.package.purl.as_deref().unwrap_or(""))
        .bind(decision_kind)
        .bind(reason)
        .bind(
            until_unix_ms
                .map(i64::try_from)
                .transpose()
                .map_err(|_| "governance journal until overflow".to_owned())?,
        )
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("postgres governance journal insert failed: {error}"))?;
        Ok(())
    }

    async fn append_governance_journal_entries_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        findings: &[FindingRef],
        decision_kind: &str,
        reason: Option<&str>,
        until_unix_ms: Option<u64>,
    ) -> Result<(), String> {
        if findings.is_empty() {
            return Ok(());
        }

        let until_unix_ms = until_unix_ms
            .map(i64::try_from)
            .transpose()
            .map_err(|_| "governance journal until overflow".to_owned())?;
        let mut query = QueryBuilder::<Postgres>::new(format!(
            "INSERT INTO {} \
            (component_key, artifact_kind, artifact_identity, vulnerability_id, package_name, package_version, package_purl, decision_kind, reason, until_unix_ms) ",
            self.names.finding_governance_journal
        ));
        query.push_values(findings, |mut row, finding| {
            row.push_bind(finding.component_key.as_ref())
                .push_bind(artifact_kind_name(finding.artifact.kind))
                .push_bind(finding.artifact.identity.as_ref())
                .push_bind(finding.vulnerability_id.as_ref())
                .push_bind(finding.package.name.as_ref())
                .push_bind(finding.package.version.as_ref())
                .push_bind(finding.package.purl.as_deref().unwrap_or(""))
                .push_bind(decision_kind)
                .push_bind(reason)
                .push_bind(until_unix_ms);
        });
        query.build().execute(&mut **tx).await.map_err(|error| {
            format!("postgres governance journal batch insert failed: {error}")
        })?;
        Ok(())
    }
}

impl PostgresRemoteChangeProbe {
    /// Read the current schema-local change watermark observed by Postgres.
    ///
    /// # Errors
    ///
    /// Returns an error string when the watermark cannot be read.
    pub async fn current_change_watermark(&self) -> Result<u64, String> {
        sqlx::query_scalar::<_, i64>(&format!(
            "SELECT change_seq FROM {} WHERE singleton = TRUE",
            self.change_watermark_table
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark read failed: {error}"))
        .and_then(|value| {
            u64::try_from(value).map_err(|_| "postgres change watermark out of range".to_owned())
        })
    }

    #[must_use]
    pub fn observed_change_watermark(&self) -> u64 {
        self.observed_change_watermark.load(Ordering::Relaxed)
    }

    pub fn observe_change_watermark(&self, change_watermark: u64) {
        self.observed_change_watermark
            .fetch_max(change_watermark, Ordering::Relaxed);
    }
}

impl PostgresReadSnapshotLoader {
    /// Load one detached Postgres-backed read snapshot delta without taking the
    /// live mutable application slot.
    ///
    /// # Errors
    ///
    /// Returns an error string when one detached lane reload fails.
    pub async fn load(
        &self,
        since_change_watermark: u64,
        base: PostgresReadSnapshotBase,
    ) -> Result<LoadedPostgresReadSnapshot, String> {
        let change_watermark = self.current_change_watermark().await?;
        if change_watermark <= since_change_watermark {
            return Ok(LoadedPostgresReadSnapshot {
                inventory: base.inventory,
                read_model: base.read_model,
                read_model_source_watermark: base.read_model_source_watermark,
                governance_source_watermark: base.governance_source_watermark,
                system_event_index: base.system_event_index,
                system_event_source_cursor: base.system_event_source_cursor,
                command_statuses: base.command_statuses,
                command_status_source_cursor: base.command_status_source_cursor,
                change_watermark,
            });
        }

        let lane_mask = self
            .changed_lane_mask(since_change_watermark, change_watermark)
            .await?;
        let inventory = {
            let inventory = if lane_mask & CHANGE_LANE_INVENTORY != 0 {
                self.load_inventory_core_snapshot(base.inventory).await?
            } else {
                base.inventory
            };
            let inventory = if lane_mask & CHANGE_LANE_COMPONENT_BINDINGS != 0 {
                self.load_component_binding_inventory_snapshot(inventory)
                    .await?
            } else {
                inventory
            };
            let inventory = if lane_mask & CHANGE_LANE_COLLECTIONS != 0 {
                self.load_collection_inventory_snapshot(inventory).await?
            } else {
                inventory
            };
            let inventory = if lane_mask & CHANGE_LANE_COLLECTION_SCHEDULES != 0 {
                self.load_collection_schedule_inventory_snapshot(inventory)
                    .await?
            } else {
                inventory
            };
            if lane_mask & CHANGE_LANE_PROVIDER_RUNTIME_CONFIGS != 0 {
                self.load_provider_runtime_inventory_snapshot(inventory)
                    .await?
            } else {
                inventory
            }
        };
        let read_model = if lane_mask & (CHANGE_LANE_READ_MODEL | CHANGE_LANE_GOVERNANCE) != 0 {
            self.load_read_model_snapshot(
                Arc::clone(&inventory),
                base.read_model,
                base.read_model_source_watermark,
                base.governance_source_watermark,
                lane_mask,
            )
            .await?
        } else {
            base.read_model
        };
        let (read_model_source_watermark, governance_source_watermark) =
            if lane_mask & (CHANGE_LANE_READ_MODEL | CHANGE_LANE_GOVERNANCE) != 0 {
                self.load_read_model_source_watermarks(
                    base.read_model_source_watermark,
                    base.governance_source_watermark,
                    lane_mask,
                )
                .await?
            } else {
                (
                    base.read_model_source_watermark,
                    base.governance_source_watermark,
                )
            };
        let (system_event_index, system_event_source_cursor) =
            if lane_mask & CHANGE_LANE_SYSTEM_EVENTS != 0 {
                self.load_system_event_index_snapshot(
                    base.system_event_index,
                    base.system_event_source_cursor,
                )
                .await?
            } else {
                (base.system_event_index, base.system_event_source_cursor)
            };
        let (command_statuses, command_status_source_cursor) =
            if lane_mask & CHANGE_LANE_COMMAND_STATUSES != 0 {
                self.load_command_statuses_snapshot(
                    base.command_statuses,
                    base.command_status_source_cursor,
                )
                .await?
            } else {
                (base.command_statuses, base.command_status_source_cursor)
            };

        Ok(LoadedPostgresReadSnapshot {
            inventory,
            read_model,
            read_model_source_watermark,
            governance_source_watermark,
            system_event_index,
            system_event_source_cursor,
            command_statuses,
            command_status_source_cursor,
            change_watermark,
        })
    }

    async fn current_change_watermark(&self) -> Result<u64, String> {
        sqlx::query_scalar::<_, i64>(&format!(
            "SELECT change_seq FROM {} WHERE singleton = TRUE",
            self.change_watermark_table
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres change watermark read failed: {error}"))
        .and_then(|value| {
            u64::try_from(value).map_err(|_| "postgres change watermark out of range".to_owned())
        })
    }

    async fn load_governance_source_watermark(&self) -> Result<u64, String> {
        let max_id = sqlx::query_scalar::<_, Option<i64>>(&format!(
            "SELECT MAX(id) FROM {}",
            self.names.finding_governance_journal
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres governance journal watermark read failed: {error}"))?
        .unwrap_or_default();
        u64::try_from(max_id)
            .map_err(|_| "postgres governance journal watermark out of range".to_owned())
    }

    async fn load_inventory_core_snapshot(
        &self,
        base_inventory: Arc<ComponentInventory>,
    ) -> Result<Arc<ComponentInventory>, String> {
        let mut backend = PostgresStore::detached(self.pool.clone(), self.names.clone());
        backend.ingestion = FindingIngestion::from_inventory_arc(base_inventory);
        backend.load_components().await?;
        backend.load_context_profiles().await?;
        backend.load_component_tags().await?;
        Ok(backend.inventory_snapshot_arc())
    }

    async fn changed_lane_mask(
        &self,
        since_change_watermark: u64,
        current_change_watermark: u64,
    ) -> Result<i32, String> {
        let earliest_retained_change_seq = sqlx::query_scalar::<_, Option<i64>>(&format!(
            "SELECT MIN(change_seq) FROM {}",
            self.names.change_journal
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres change journal coverage read failed: {error}"))?
        .map(|value| {
            u64::try_from(value)
                .map_err(|_| "postgres change journal minimum change_seq out of range".to_owned())
        })
        .transpose()?;
        let lane_mask = sqlx::query_scalar::<_, Option<i32>>(&format!(
            concat!(
                "SELECT COALESCE(bit_or(lane_mask), 0) FROM {} ",
                "WHERE change_seq > $1 AND change_seq <= $2"
            ),
            self.names.change_journal
        ))
        .bind(
            i64::try_from(since_change_watermark)
                .map_err(|_| "postgres change watermark lower bound out of range".to_owned())?,
        )
        .bind(
            i64::try_from(current_change_watermark)
                .map_err(|_| "postgres change watermark upper bound out of range".to_owned())?,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("postgres change journal read failed: {error}"))
        .map(Option::unwrap_or_default)?;
        if change_journal_gap_requires_full_refresh(
            since_change_watermark,
            current_change_watermark,
            earliest_retained_change_seq,
        ) {
            Ok(CHANGE_LANE_ALL)
        } else {
            Ok(lane_mask)
        }
    }

    async fn load_component_binding_inventory_snapshot(
        &self,
        inventory: Arc<ComponentInventory>,
    ) -> Result<Arc<ComponentInventory>, String> {
        let mut backend = PostgresStore::detached(self.pool.clone(), self.names.clone());
        let mut inventory = Arc::unwrap_or_clone(inventory);
        inventory.reset_component_bindings_for_rebuild();
        backend.ingestion = FindingIngestion::from_inventory_arc(Arc::new(inventory));
        backend.load_component_context_profiles().await?;
        backend.load_component_tag_memberships().await?;
        backend.load_artifact_bindings().await?;
        backend.refresh_inventory_snapshot_cache();
        Ok(backend.inventory_snapshot_arc())
    }

    async fn load_collection_inventory_snapshot(
        &self,
        inventory: Arc<ComponentInventory>,
    ) -> Result<Arc<ComponentInventory>, String> {
        let mut backend = PostgresStore::detached(self.pool.clone(), self.names.clone());
        let mut inventory = Arc::unwrap_or_clone(inventory);
        inventory.reset_collections_for_rebuild();
        backend.ingestion = FindingIngestion::from_inventory_arc(Arc::new(inventory));
        backend.load_collections().await?;
        backend.load_collection_sources().await?;
        backend.load_collection_memberships().await?;
        backend.load_collection_scan_schedules().await?;
        backend.refresh_inventory_snapshot_cache();
        Ok(backend.inventory_snapshot_arc())
    }

    async fn load_collection_schedule_inventory_snapshot(
        &self,
        inventory: Arc<ComponentInventory>,
    ) -> Result<Arc<ComponentInventory>, String> {
        let mut backend = PostgresStore::detached(self.pool.clone(), self.names.clone());
        backend.ingestion = FindingIngestion::from_inventory_arc(inventory);
        backend.load_collection_scan_schedules().await?;
        backend.refresh_inventory_snapshot_cache();
        Ok(backend.inventory_snapshot_arc())
    }

    async fn load_provider_runtime_inventory_snapshot(
        &self,
        inventory: Arc<ComponentInventory>,
    ) -> Result<Arc<ComponentInventory>, String> {
        let mut backend = PostgresStore::detached(self.pool.clone(), self.names.clone());
        backend.ingestion = FindingIngestion::from_inventory_arc(inventory);
        backend.load_provider_runtime_configs().await?;
        backend.refresh_inventory_snapshot_cache();
        Ok(backend.inventory_snapshot_arc())
    }

    async fn load_read_model_snapshot(
        &self,
        inventory: Arc<ComponentInventory>,
        base_read_model: Arc<FindingReadModel>,
        base_read_model_source_watermark: u64,
        base_governance_source_watermark: u64,
        lane_mask: i32,
    ) -> Result<Arc<FindingReadModel>, String> {
        let mut backend = PostgresStore::detached(self.pool.clone(), self.names.clone());
        backend.ingestion = FindingIngestion::from_inventory_arc(inventory);
        backend.provider_report_row_high_watermark = base_read_model_source_watermark;
        backend.governance_journal_high_watermark = base_governance_source_watermark;
        backend.read_model = base_read_model;
        if lane_mask & CHANGE_LANE_READ_MODEL != 0 {
            backend
                .load_provider_reports_after(base_read_model_source_watermark)
                .await?;
        }
        if lane_mask & CHANGE_LANE_GOVERNANCE != 0 {
            backend
                .load_governance_journal_after(base_governance_source_watermark, false)
                .await?;
        }
        backend.refresh_read_model_snapshot_cache();
        Ok(backend.read_model_snapshot_arc())
    }

    async fn load_read_model_source_watermarks(
        &self,
        base_read_model_source_watermark: u64,
        base_governance_source_watermark: u64,
        lane_mask: i32,
    ) -> Result<(u64, u64), String> {
        let read_model_source_watermark = if lane_mask & CHANGE_LANE_READ_MODEL == 0 {
            base_read_model_source_watermark
        } else {
            let max_id = sqlx::query_scalar::<_, Option<i64>>(&format!(
                "SELECT MAX(id) FROM {}",
                self.names.provider_reports
            ))
            .fetch_one(&self.pool)
            .await
            .map_err(|error| format!("postgres provider report watermark read failed: {error}"))?
            .unwrap_or_default();
            u64::try_from(max_id)
                .map_err(|_| "postgres provider report watermark out of range".to_owned())?
        };
        let governance_source_watermark = if lane_mask & CHANGE_LANE_GOVERNANCE == 0 {
            base_governance_source_watermark
        } else {
            self.load_governance_source_watermark().await?
        };
        Ok((read_model_source_watermark, governance_source_watermark))
    }

    async fn load_system_event_index_snapshot(
        &self,
        base_system_event_index: Arc<SystemEventQueryIndex>,
        base_system_event_source_cursor: EventSourceCursor,
    ) -> Result<(Arc<SystemEventQueryIndex>, EventSourceCursor), String> {
        let rows = sqlx::query_as::<_, SystemEventDeltaRow>(&format!(
            concat!(
                "SELECT event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail, created_at_micros ",
                "FROM (",
                "SELECT event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail, ",
                "(EXTRACT(EPOCH FROM created_at) * 1000000)::bigint AS created_at_micros ",
                "FROM {}",
                ") delta ",
                "WHERE created_at_micros > $1 OR (created_at_micros = $1 AND event_id > $2) ",
                "ORDER BY created_at_micros DESC, event_id DESC"
            ),
            self.names.system_events
        ))
        .bind(
            i64::try_from(base_system_event_source_cursor.unix_micros)
                .map_err(|_| "postgres system event cursor out of range".to_owned())?,
        )
        .bind(
            base_system_event_source_cursor
                .tie_breaker
                .as_deref()
                .unwrap_or(""),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres system event delta load failed: {error}"))?;

        if rows.is_empty() {
            return Ok((base_system_event_index, base_system_event_source_cursor));
        }

        let mut cursor = base_system_event_source_cursor;
        let mut delta_events = Vec::with_capacity(rows.len());
        for row in rows {
            let (
                event_id,
                occurred_at_unix_ms,
                category,
                kind,
                collection_key,
                component_key,
                command_id,
                integration_event_id,
                finding_count,
                retryable,
                detail,
                created_at_micros,
            ) = row;
            cursor = max_event_source_cursor(
                &cursor,
                event_source_cursor(
                    u64::try_from(created_at_micros)
                        .map_err(|_| "postgres system event created_at out of range".to_owned())?,
                    event_id.clone().into_boxed_str(),
                ),
            );
            delta_events.push(parse_system_event_row((
                event_id,
                occurred_at_unix_ms,
                category,
                kind,
                collection_key,
                component_key,
                command_id,
                integration_event_id,
                finding_count,
                retryable,
                detail,
            ))?);
        }

        let delta_index = SystemEventQueryIndex::from_newest_first(delta_events.iter());
        Ok((
            Arc::new(SystemEventQueryIndex::merged(
                base_system_event_index.as_ref(),
                &delta_index,
            )),
            cursor,
        ))
    }

    async fn load_command_statuses_snapshot(
        &self,
        base_command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
        base_command_status_source_cursor: RowSourceCursor,
    ) -> Result<(Arc<BTreeMap<Box<str>, ScanCommandStatus>>, RowSourceCursor), String> {
        let rows = sqlx::query_as::<_, CommandStatusDeltaRow>(&format!(
            concat!(
                "SELECT command_id, status, updated_at_micros ",
                "FROM (",
                "SELECT command_id, status, ",
                "(EXTRACT(EPOCH FROM updated_at) * 1000000)::bigint AS updated_at_micros ",
                "FROM {}",
                ") delta ",
                "WHERE updated_at_micros > $1 OR (updated_at_micros = $1 AND command_id > $2) ",
                "ORDER BY updated_at_micros ASC, command_id ASC"
            ),
            self.names.scan_commands
        ))
        .bind(
            i64::try_from(base_command_status_source_cursor.unix_micros)
                .map_err(|_| "postgres command status cursor out of range".to_owned())?,
        )
        .bind(
            base_command_status_source_cursor
                .tie_breaker
                .as_deref()
                .unwrap_or(""),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres command status delta load failed: {error}"))?;

        if rows.is_empty() {
            return Ok((base_command_statuses, base_command_status_source_cursor));
        }

        let mut cursor = base_command_status_source_cursor;
        let mut statuses = Arc::unwrap_or_clone(base_command_statuses);
        for (command_id, status, updated_at_micros) in rows {
            let command_id = command_id.into_boxed_str();
            statuses.insert(command_id.clone(), parse_scan_command_status(&status)?);
            cursor = max_row_source_cursor(
                &cursor,
                row_source_cursor(
                    u64::try_from(updated_at_micros)
                        .map_err(|_| "postgres scan command updated_at out of range".to_owned())?,
                    command_id,
                ),
            );
        }

        Ok((Arc::new(statuses), cursor))
    }
}

#[derive(Debug, Clone)]
struct ScanCommandRecord {
    request: ScanRequest,
    status: ScanCommandStatus,
}

const fn change_journal_gap_requires_full_refresh(
    since_change_watermark: u64,
    current_change_watermark: u64,
    earliest_retained_change_seq: Option<u64>,
) -> bool {
    if current_change_watermark <= since_change_watermark {
        return false;
    }
    match earliest_retained_change_seq {
        Some(earliest_retained) => earliest_retained > since_change_watermark.saturating_add(1),
        None => true,
    }
}

#[derive(Debug, Clone)]
struct TableNames {
    schema: Box<str>,
    change_watermark: Box<str>,
    change_journal: Box<str>,
    components: Box<str>,
    context_profiles: Box<str>,
    component_context_profiles: Box<str>,
    component_tags: Box<str>,
    component_tag_memberships: Box<str>,
    collections: Box<str>,
    collection_sources: Box<str>,
    collection_memberships: Box<str>,
    collection_scan_schedules: Box<str>,
    artifact_bindings: Box<str>,
    provider_runtime_configs: Box<str>,
    integration_runtime_config: Box<str>,
    provider_reports: Box<str>,
    finding_risk_acceptances: Box<str>,
    finding_suppressions: Box<str>,
    finding_governance_journal: Box<str>,
    scan_commands: Box<str>,
    integration_outbox: Box<str>,
    system_events: Box<str>,
}

impl TableNames {
    fn new(schema: &str) -> Result<Self, String> {
        let schema = validate_schema_name(schema)?;
        Ok(Self {
            change_watermark: format!("{schema}.change_watermark").into_boxed_str(),
            change_journal: format!("{schema}.change_journal").into_boxed_str(),
            components: format!("{schema}.components").into_boxed_str(),
            context_profiles: format!("{schema}.context_profiles").into_boxed_str(),
            component_context_profiles: format!("{schema}.component_context_profiles")
                .into_boxed_str(),
            component_tags: format!("{schema}.component_tags").into_boxed_str(),
            component_tag_memberships: format!("{schema}.component_tag_memberships")
                .into_boxed_str(),
            collections: format!("{schema}.collections").into_boxed_str(),
            collection_sources: format!("{schema}.collection_sources").into_boxed_str(),
            collection_memberships: format!("{schema}.collection_memberships").into_boxed_str(),
            collection_scan_schedules: format!("{schema}.collection_scan_schedules")
                .into_boxed_str(),
            artifact_bindings: format!("{schema}.artifact_bindings").into_boxed_str(),
            provider_runtime_configs: format!("{schema}.provider_runtime_configs").into_boxed_str(),
            integration_runtime_config: format!("{schema}.integration_runtime_config")
                .into_boxed_str(),
            provider_reports: format!("{schema}.provider_reports").into_boxed_str(),
            finding_risk_acceptances: format!("{schema}.finding_risk_acceptances").into_boxed_str(),
            finding_suppressions: format!("{schema}.finding_suppressions").into_boxed_str(),
            finding_governance_journal: format!("{schema}.finding_governance_journal")
                .into_boxed_str(),
            scan_commands: format!("{schema}.scan_commands").into_boxed_str(),
            integration_outbox: format!("{schema}.integration_outbox").into_boxed_str(),
            system_events: format!("{schema}.system_events").into_boxed_str(),
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

const fn collection_source_kind_name(value: &CollectionSource) -> &'static str {
    match value {
        CollectionSource::ComponentList(_) => "component-list",
    }
}

const fn collection_source_mode_name(value: CollectionSourceMode) -> &'static str {
    match value {
        CollectionSourceMode::Replace => "replace",
        CollectionSourceMode::Reconcile => "reconcile",
    }
}

fn parse_system_event_kind(value: &str) -> Result<SystemEventKind, String> {
    match value {
        "collection-scan-materialized" => Ok(SystemEventKind::CollectionScanMaterialized),
        "scan-command-enqueued" => Ok(SystemEventKind::ScanCommandEnqueued),
        "scan-command-completed" => Ok(SystemEventKind::ScanCommandCompleted),
        "scan-command-failed" => Ok(SystemEventKind::ScanCommandFailed),
        "finding-risk-accepted" => Ok(SystemEventKind::FindingRiskAccepted),
        "findings-risk-accepted" => Ok(SystemEventKind::FindingsRiskAccepted),
        "finding-suppressed" => Ok(SystemEventKind::FindingSuppressed),
        "findings-suppressed" => Ok(SystemEventKind::FindingsSuppressed),
        "finding-reopened" => Ok(SystemEventKind::FindingReopened),
        "findings-reopened" => Ok(SystemEventKind::FindingsReopened),
        "integration-event-published" => Ok(SystemEventKind::IntegrationEventPublished),
        "integration-event-publication-failed" => {
            Ok(SystemEventKind::IntegrationEventPublicationFailed)
        }
        other => Err(format!("unsupported system event kind: {other}")),
    }
}

fn parse_system_event_row(row: SystemEventRow) -> Result<SystemEvent, String> {
    let (
        event_id,
        occurred_at_unix_ms,
        _category,
        kind,
        collection_key,
        component_key,
        command_id,
        integration_event_id,
        finding_count,
        retryable,
        detail,
    ) = row;

    Ok(SystemEvent {
        event_id: event_id.into_boxed_str(),
        occurred_at_unix_ms: u64::try_from(occurred_at_unix_ms)
            .map_err(|_| "negative system event timestamp".to_owned())?,
        kind: parse_system_event_kind(&kind)?,
        collection_key: collection_key.map(String::into_boxed_str),
        component_key: component_key.map(String::into_boxed_str),
        command_id: command_id.map(String::into_boxed_str),
        integration_event_id: integration_event_id.map(String::into_boxed_str),
        finding_count: finding_count
            .map(u32::try_from)
            .transpose()
            .map_err(|_| "system event finding count out of range".to_owned())?,
        retryable,
        detail: detail.map(String::into_boxed_str),
    })
}

const fn event_source_cursor(unix_micros: u64, tie_breaker: Box<str>) -> EventSourceCursor {
    EventSourceCursor {
        unix_micros,
        tie_breaker: Some(tie_breaker),
    }
}

const fn row_source_cursor(unix_micros: u64, tie_breaker: Box<str>) -> RowSourceCursor {
    RowSourceCursor {
        unix_micros,
        tie_breaker: Some(tie_breaker),
    }
}

fn max_event_source_cursor(
    left: &EventSourceCursor,
    right: EventSourceCursor,
) -> EventSourceCursor {
    if compare_source_cursor(
        left.unix_micros,
        left.tie_breaker.as_deref(),
        right.unix_micros,
        right.tie_breaker.as_deref(),
    )
    .is_ge()
    {
        left.clone()
    } else {
        right
    }
}

fn max_row_source_cursor(left: &RowSourceCursor, right: RowSourceCursor) -> RowSourceCursor {
    if compare_source_cursor(
        left.unix_micros,
        left.tie_breaker.as_deref(),
        right.unix_micros,
        right.tie_breaker.as_deref(),
    )
    .is_ge()
    {
        left.clone()
    } else {
        right
    }
}

fn compare_source_cursor(
    left_unix_micros: u64,
    left_tie_breaker: Option<&str>,
    right_unix_micros: u64,
    right_tie_breaker: Option<&str>,
) -> std::cmp::Ordering {
    left_unix_micros
        .cmp(&right_unix_micros)
        .then_with(|| left_tie_breaker.cmp(&right_tie_breaker))
}

fn parse_freshness(value: &str) -> Result<EvidenceFreshness, String> {
    match value {
        "deterministic" => Ok(EvidenceFreshness::Deterministic),
        "live" => Ok(EvidenceFreshness::Live),
        other => Err(format!("unsupported freshness: {other}")),
    }
}

fn parse_collection_source(
    kind: &str,
    mode: &str,
    component_keys: Vec<Box<str>>,
) -> Result<CollectionSource, String> {
    let mode = match mode {
        "replace" => CollectionSourceMode::Replace,
        "reconcile" => CollectionSourceMode::Reconcile,
        value => return Err(format!("unsupported collection source mode: {value}")),
    };

    match kind {
        "component-list" => Ok(CollectionSource::ComponentList(
            ComponentListCollectionSource::new(mode, component_keys),
        )),
        value => Err(format!("unsupported collection source kind: {value}")),
    }
}

fn parse_integration_runtime_config_row(
    publisher_key: &str,
    endpoint_url: Option<String>,
    timeout_ms: Option<i64>,
) -> Result<IntegrationRuntimeConfig, String> {
    match publisher_key {
        "fixture-publisher" => Ok(IntegrationRuntimeConfig::Fixture),
        "http-publisher" => {
            let endpoint_url = endpoint_url
                .ok_or_else(|| "postgres http publisher config missing endpoint_url".to_owned())?;
            let timeout_ms = timeout_ms
                .ok_or_else(|| "postgres http publisher config missing timeout_ms".to_owned())?;
            let timeout_ms = u32::try_from(timeout_ms)
                .map_err(|_| "postgres http publisher timeout_ms is invalid".to_owned())?;
            Ok(IntegrationRuntimeConfig::Http {
                endpoint_url: endpoint_url.into_boxed_str(),
                timeout_ms,
            })
        }
        _ => Err("postgres integration runtime config has unsupported publisher".to_owned()),
    }
}

const fn scan_command_status_name(value: ScanCommandStatus) -> &'static str {
    match value {
        ScanCommandStatus::Pending => "pending",
        ScanCommandStatus::Applying => "applying",
        ScanCommandStatus::Completed => "completed",
        ScanCommandStatus::Failed => "failed",
    }
}

fn parse_scan_command_status(value: &str) -> Result<ScanCommandStatus, String> {
    match value {
        "pending" => Ok(ScanCommandStatus::Pending),
        "applying" => Ok(ScanCommandStatus::Applying),
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
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("scan-command-{nanos}-{counter}").into_boxed_str()
}

fn next_system_event_id(prefix: &str) -> Box<str> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("system-event-{prefix}-{nanos}-{counter}").into_boxed_str()
}

fn current_unix_millis() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system time must be after unix epoch: {error}"))?;
    u64::try_from(duration.as_millis()).map_err(|_| "current unix millis overflow".to_owned())
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
    use super::PostgresReadSnapshotBase;
    use super::PostgresStore;
    use super::change_journal_gap_requires_full_refresh;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use venom_domain::{
        ArtifactKind, ArtifactRef, BulkGovernanceQuery, CollectionRegistration, CollectionSource,
        CollectionSourceMode, ComponentListCollectionSource, ComponentRegistration,
        EvidenceFreshness, FindingGovernanceState, FindingProvider, FindingProviderError,
        IntegrationEvent, IntegrationEventPublishError, IntegrationEventPublisher,
        PackageCoordinate, PendingIntegrationEvent, ProviderScanReport, ReportedFinding,
        RiskAcceptance, RunNextScanResult, ScanCommandStatus, SystemEventKind,
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

    #[test]
    fn change_journal_gap_requires_full_refresh_after_retention_hole() {
        assert!(change_journal_gap_requires_full_refresh(12, 32, Some(18)));
    }

    #[test]
    fn change_journal_gap_does_not_require_full_refresh_when_coverage_is_contiguous() {
        assert!(!change_journal_gap_requires_full_refresh(12, 32, Some(13)));
        assert!(!change_journal_gap_requires_full_refresh(12, 12, Some(13)));
    }

    #[tokio::test]
    async fn postgres_remote_change_probe_ignores_unrelated_schema_writes() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("remote_probe_scope");
        let backend = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should open");
        let probe = backend.remote_change_probe();
        let observed = probe.observed_change_watermark();
        assert_eq!(
            probe
                .current_change_watermark()
                .await
                .expect("watermark should be readable"),
            observed
        );

        let unrelated_schema = temp_schema("remote_probe_other");
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {unrelated_schema}"))
            .execute(&backend.pool)
            .await
            .expect("unrelated schema should create");
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS {unrelated_schema}.noise (id BIGSERIAL PRIMARY KEY, value TEXT NOT NULL)"
        ))
        .execute(&backend.pool)
        .await
        .expect("unrelated table should create");
        sqlx::query(&format!(
            "INSERT INTO {unrelated_schema}.noise (value) VALUES ('unrelated')"
        ))
        .execute(&backend.pool)
        .await
        .expect("unrelated row should insert");

        assert_eq!(
            probe
                .current_change_watermark()
                .await
                .expect("watermark should stay readable"),
            observed
        );
    }

    #[tokio::test]
    async fn postgres_inventory_snapshot_cache_reuses_live_inventory_arc() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("inventory_arc");
        let mut backend = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should open");
        let _ = backend
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .await
            .expect("registration should persist");

        let snapshot_before = backend.inventory_snapshot_arc();
        let live_before = backend.ingestion.inventory_arc();
        assert!(Arc::ptr_eq(&snapshot_before, &live_before));

        let _ = backend
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");

        let snapshot_after = backend.inventory_snapshot_arc();
        let live_after = backend.ingestion.inventory_arc();
        assert!(Arc::ptr_eq(&snapshot_after, &live_after));
        assert!(!Arc::ptr_eq(&snapshot_before, &snapshot_after));
        assert!(!snapshot_before.component_owns_artifact("component:payments-api", &artifact()));
        assert!(snapshot_after.component_owns_artifact("component:payments-api", &artifact()));
    }

    #[tokio::test]
    async fn postgres_read_model_snapshot_cache_reuses_live_read_model_arc() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("read_model_arc");
        let mut backend = PostgresStore::open(&database_url, &schema)
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

        let snapshot_before = backend.read_model_snapshot_arc();
        let live_before = backend.read_model_arc();
        assert!(Arc::ptr_eq(&snapshot_before, &live_before));

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

        let snapshot_after = backend.read_model_snapshot_arc();
        let live_after = backend.read_model_arc();
        assert!(Arc::ptr_eq(&snapshot_after, &live_after));
        assert!(!Arc::ptr_eq(&snapshot_before, &snapshot_after));
        assert_eq!(
            snapshot_before.active_finding_count("component:payments-api", &artifact()),
            0
        );
        assert_eq!(
            snapshot_after.active_finding_count("component:payments-api", &artifact()),
            1
        );
    }

    #[tokio::test]
    async fn postgres_fork_shares_runtime_and_outbox_sources_until_lane_mutation() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("forked_runtime_sources");
        let mut backend = PostgresStore::open(&database_url, &schema)
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

        let mut fork = PostgresStore::fork_from(&backend);
        assert!(Arc::ptr_eq(&backend.commands, &fork.commands));
        assert!(Arc::ptr_eq(&backend.order, &fork.order));
        assert!(Arc::ptr_eq(
            &backend.pending_integration_events,
            &fork.pending_integration_events
        ));

        let _ = fork
            .request_scan(
                "component:payments-api",
                artifact(),
                EvidenceFreshness::Deterministic,
            )
            .await
            .expect("forked scan request should persist");
        assert!(!Arc::ptr_eq(&backend.commands, &fork.commands));
        assert!(!Arc::ptr_eq(&backend.order, &fork.order));
        assert_eq!(backend.pending_commands(), 0);
        assert_eq!(fork.pending_commands(), 1);

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
        let _ = fork
            .record_scan_report(&report)
            .await
            .expect("forked provider report should persist");
        assert!(!Arc::ptr_eq(
            &backend.pending_integration_events,
            &fork.pending_integration_events
        ));
        assert_eq!(backend.pending_integration_events().len(), 0);
        assert_eq!(fork.pending_integration_events().len(), 1);
    }

    #[tokio::test]
    async fn detached_postgres_read_snapshot_advances_read_model_source_watermark_for_new_reports()
    {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("detached_read_model_cursor");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
        let first_report = ProviderScanReport::new(
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
            .record_scan_report(&first_report)
            .await
            .expect("first provider report should persist");

        let loader = backend.read_snapshot_loader();
        let since_change_watermark = backend
            .current_change_watermark()
            .await
            .expect("current watermark should be readable after first report");
        let base_inventory = backend.inventory_snapshot_arc();
        let base_read_model = backend.read_model_snapshot_arc();
        let base_read_model_source_watermark = backend.read_model_source_watermark();
        let base_governance_source_watermark = backend.governance_source_watermark();
        let base_system_event_index = backend.system_event_index_snapshot_arc();
        let base_system_event_source_cursor = backend.system_event_source_cursor();
        let base_command_statuses = backend.command_statuses_snapshot_arc();
        let base_command_status_source_cursor = backend.command_status_source_cursor();
        let snapshot_base = PostgresReadSnapshotBase::new(
            base_inventory,
            Arc::clone(&base_read_model),
            base_read_model_source_watermark,
            base_governance_source_watermark,
            base_system_event_index,
            base_system_event_source_cursor,
            base_command_statuses,
            base_command_status_source_cursor,
        );

        let mut writer = PostgresStore::open(&database_url, &schema)
            .await
            .expect("writer backend should reopen");
        let second_report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![
                ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0")),
                ReportedFinding::new("CVE-2026-0002", PackageCoordinate::new("libxml2", "2.11.0")),
            ],
        )
        .with_knowledge_revision("fixture-db:2026-05-17");
        let _ = writer
            .record_scan_report(&second_report)
            .await
            .expect("second provider report should persist");

        let refreshed_snapshot = loader
            .load(since_change_watermark, snapshot_base)
            .await
            .expect("detached fresh read should load");

        assert!(
            refreshed_snapshot.read_model_source_watermark > base_read_model_source_watermark,
            "detached read should advance the provider-report cursor"
        );
        assert_eq!(
            base_read_model.active_finding_count("component:payments-api", &artifact()),
            1
        );
        assert_eq!(
            refreshed_snapshot
                .read_model
                .active_finding_count("component:payments-api", &artifact()),
            2
        );
    }

    #[tokio::test]
    async fn postgres_reopened_findings_are_replayed_from_governance_journal() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("governance_journal_reopen");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
        let finding = venom_domain::FindingRef::new(
            "component:payments-api",
            artifact(),
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        );
        let _ = backend
            .accept_risk(finding.clone(), RiskAcceptance::new("approved for now"))
            .await
            .expect("risk acceptance should persist");
        let _ = backend
            .reopen_finding(finding.clone())
            .await
            .expect("reopen should persist");

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert!(reopened.governance().decision(&finding).is_none());
    }

    #[tokio::test]
    async fn detached_postgres_read_snapshot_advances_governance_journal_cursor_for_reopened_findings()
    {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("detached_governance_cursor");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
        let finding = venom_domain::FindingRef::new(
            "component:payments-api",
            artifact(),
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        );
        let _ = backend
            .accept_risk(finding.clone(), RiskAcceptance::new("approved for now"))
            .await
            .expect("risk acceptance should persist");

        let loader = backend.read_snapshot_loader();
        let since_change_watermark = backend
            .current_change_watermark()
            .await
            .expect("current watermark should be readable after acceptance");
        let snapshot_base = PostgresReadSnapshotBase::new(
            backend.inventory_snapshot_arc(),
            backend.read_model_snapshot_arc(),
            backend.read_model_source_watermark(),
            backend.governance_source_watermark(),
            backend.system_event_index_snapshot_arc(),
            backend.system_event_source_cursor(),
            backend.command_statuses_snapshot_arc(),
            backend.command_status_source_cursor(),
        );

        let mut writer = PostgresStore::open(&database_url, &schema)
            .await
            .expect("writer backend should reopen");
        let _ = writer
            .reopen_finding(finding)
            .await
            .expect("reopen should persist");

        let refreshed_snapshot = loader
            .load(since_change_watermark, snapshot_base)
            .await
            .expect("detached fresh read should load");
        assert!(
            refreshed_snapshot.governance_source_watermark
                > backend.governance_source_watermark(),
            "detached read should advance the governance cursor"
        );
    }

    #[tokio::test]
    async fn postgres_live_refresh_reloads_pending_commands_incrementally() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("live_pending_commands_delta");
        let mut writer = PostgresStore::open(&database_url, &schema)
            .await
            .expect("writer backend should open");
        let mut follower = PostgresStore::open(&database_url, &schema)
            .await
            .expect("follower backend should open");
        let _ = writer
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .await
            .expect("registration should persist");
        let _ = writer
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");
        let command_id = writer
            .request_scan(
                "component:payments-api",
                artifact(),
                EvidenceFreshness::Deterministic,
            )
            .await
            .expect("scan request should persist");

        assert!(follower
            .refresh_from_remote_if_stale()
            .await
            .expect("follower refresh should succeed"));
        assert_eq!(
            follower.command_status(command_id.as_ref()),
            Some(ScanCommandStatus::Pending)
        );
    }

    #[tokio::test]
    async fn postgres_live_refresh_reloads_system_events_incrementally() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("live_system_events_delta");
        let mut writer = PostgresStore::open(&database_url, &schema)
            .await
            .expect("writer backend should open");
        let mut follower = PostgresStore::open(&database_url, &schema)
            .await
            .expect("follower backend should open");
        let _ = writer
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .await
            .expect("registration should persist");
        let _ = writer
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");
        let _ = writer
            .request_scan(
                "component:payments-api",
                artifact(),
                EvidenceFreshness::Deterministic,
            )
            .await
            .expect("scan request should persist");

        assert!(follower
            .refresh_from_remote_if_stale()
            .await
            .expect("follower refresh should succeed"));
        let kinds = follower
            .system_events_snapshot()
            .into_iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>();
        assert!(kinds.contains(&SystemEventKind::ScanCommandEnqueued));
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
        let mut backend = PostgresStore::open(&database_url, &schema)
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

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(reopened.pending_integration_events().len(), 1);
    }

    #[tokio::test]
    async fn postgres_rebuilds_release_collections() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("collections");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .await
            .expect("collection should persist");
        let _ = backend
            .add_component_to_collection("release:2026.05", "component:payments-api")
            .await
            .expect("collection membership should persist");

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(
            reopened
                .inventory_snapshot()
                .collection_members("release:2026.05"),
            Some(vec![Box::<str>::from("component:payments-api")])
        );
    }

    #[tokio::test]
    async fn postgres_reloads_collection_sources_and_materialized_membership() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("collection_sources");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .await
            .expect("collection should persist");
        let _ = backend
            .configure_collection_source(
                "release:2026.05",
                CollectionSource::ComponentList(ComponentListCollectionSource::new(
                    CollectionSourceMode::Replace,
                    vec![Box::<str>::from("component:payments-api")],
                )),
            )
            .await
            .expect("collection source should persist");
        let _ = backend
            .materialize_collection_source("release:2026.05")
            .await
            .expect("collection source materialization should persist");

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        let source = reopened
            .inventory_snapshot()
            .collection_source("release:2026.05")
            .expect("collection source should reload");
        assert_eq!(source.mode(), CollectionSourceMode::Replace);
        assert_eq!(
            source.component_keys(),
            [Box::<str>::from("component:payments-api")]
        );
        assert_eq!(
            reopened
                .inventory_snapshot()
                .collection_members("release:2026.05"),
            Some(vec![Box::<str>::from("component:payments-api")])
        );
    }

    #[tokio::test]
    async fn postgres_collection_scan_request_batches_pending_commands() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("collection_scan_batch");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
            .register_component(ComponentRegistration::new(
                "component:billing-api",
                "Billing API",
            ))
            .await
            .expect("registration should persist");
        let _ = backend
            .bind_artifact("component:payments-api", artifact())
            .await
            .expect("artifact binding should persist");
        let _ = backend
            .bind_artifact(
                "component:billing-api",
                ArtifactRef::new(
                    ArtifactKind::ContainerImage,
                    "registry.example/billing@sha256:222",
                ),
            )
            .await
            .expect("artifact binding should persist");
        let _ = backend
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .await
            .expect("collection should persist");
        let _ = backend
            .add_component_to_collection("release:2026.05", "component:billing-api")
            .await
            .expect("collection membership should persist");
        let _ = backend
            .add_component_to_collection("release:2026.05", "component:payments-api")
            .await
            .expect("collection membership should persist");

        let command_ids = backend
            .request_collection_scan("release:2026.05", EvidenceFreshness::Deterministic)
            .await
            .expect("collection scan request should persist");

        assert_eq!(command_ids.len(), 2);
        assert_eq!(backend.pending_commands(), 2);

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(reopened.pending_commands(), 2);
        assert_eq!(
            reopened.command_status(command_ids[0].as_ref()),
            Some(ScanCommandStatus::Pending)
        );
        assert_eq!(
            reopened.command_status(command_ids[1].as_ref()),
            Some(ScanCommandStatus::Pending)
        );
        let enqueued_events = reopened
            .system_events_snapshot()
            .into_iter()
            .filter(|event| event.kind == SystemEventKind::ScanCommandEnqueued)
            .count();
        assert_eq!(enqueued_events, 2);
    }

    #[tokio::test]
    async fn postgres_due_collection_scan_drain_reloads_system_events() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("due_schedule_events");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .await
            .expect("collection should persist");
        let _ = backend
            .add_component_to_collection("release:2026.05", "component:payments-api")
            .await
            .expect("collection membership should persist");
        let _ = backend
            .configure_collection_scan_schedule(
                "release:2026.05",
                60,
                EvidenceFreshness::Deterministic,
                1_000,
            )
            .await
            .expect("collection schedule should persist");

        let result = backend
            .drain_due_collection_scans(8, 1_500)
            .await
            .expect("due collection scans should drain");
        assert_eq!(result.processed_collections, 1);
        assert_eq!(result.enqueued_commands, 1);

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        let kinds = reopened
            .system_events_snapshot()
            .into_iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>();
        assert!(kinds.contains(&SystemEventKind::CollectionScanMaterialized));
        assert!(kinds.contains(&SystemEventKind::ScanCommandEnqueued));
    }

    #[tokio::test]
    async fn postgres_completed_scan_command_appends_pending_integration_event() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("outbox_command");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
        assert_eq!(backend.pending_integration_events().len(), 2);
        assert!(matches!(
            backend.pending_integration_events()[0].event,
            IntegrationEvent::FindingChangesObserved { .. }
        ));
        assert!(matches!(
            backend.pending_integration_events()[1].event,
            IntegrationEvent::ScanCommandCompleted { .. }
        ));

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(
            reopened.command_status(command_id.as_ref()),
            Some(ScanCommandStatus::Completed)
        );
        assert_eq!(
            reopened
                .command_statuses_snapshot()
                .get(command_id.as_ref())
                .copied(),
            Some(ScanCommandStatus::Completed)
        );
        assert_eq!(reopened.pending_integration_events().len(), 2);
        assert!(
            reopened
                .system_events_snapshot()
                .into_iter()
                .any(|event| event.kind == SystemEventKind::ScanCommandCompleted)
        );
    }

    #[tokio::test]
    async fn postgres_successful_publication_removes_pending_integration_event() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish_success");
        let mut backend = PostgresStore::open(&database_url, &schema)
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

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(reopened.pending_integration_events().len(), 0);
        assert!(
            reopened
                .system_events_snapshot()
                .iter()
                .any(|event| event.kind == SystemEventKind::IntegrationEventPublished)
        );
    }

    #[tokio::test]
    async fn postgres_failed_publication_keeps_pending_integration_event() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish_failure");
        let mut backend = PostgresStore::open(&database_url, &schema)
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

        let reopened = PostgresStore::open(&database_url, &schema)
            .await
            .expect("postgres backend should reopen");
        assert_eq!(reopened.pending_integration_events().len(), 1);
        assert!(
            reopened
                .system_events_snapshot()
                .iter()
                .any(|event| event.kind == SystemEventKind::IntegrationEventPublicationFailed)
        );
    }

    #[tokio::test]
    async fn postgres_bulk_collection_risk_acceptance_targets_the_full_matching_cohort() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("bulk_cohort");
        let mut backend = PostgresStore::open(&database_url, &schema)
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
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .await
            .expect("collection should persist");
        let _ = backend
            .add_component_to_collection("release:2026.05", "component:payments-api")
            .await
            .expect("collection membership should persist");
        let findings = (0..205)
            .map(|index| {
                ReportedFinding::new(
                    format!("CVE-2026-{index:04}"),
                    PackageCoordinate::new(format!("pkg-{index:04}"), "1.0.0"),
                )
            })
            .collect::<Vec<_>>();
        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            findings,
        );
        let _ = backend
            .record_scan_report(&report)
            .await
            .expect("provider report should persist");

        let result = backend
            .accept_risk_for_collection(
                "release:2026.05",
                &BulkGovernanceQuery::new(FindingGovernanceState::Open),
                RiskAcceptance::new("Accepted whole release"),
            )
            .await
            .expect("bulk risk acceptance should persist");

        assert_eq!(result.targeted, 205);
        assert_eq!(result.accepted, 205);
        assert_eq!(result.unchanged, 0);
    }
}
