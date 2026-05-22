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
    ProviderScanReport, ReleaseBoard, ReopenFindingChange, ReopenFindingResult, ReportedFinding,
    RiskAcceptance, ScanRequest, SuppressFindingChange, SuppressFindingResult, Suppression,
    build_release_board,
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
use venom_domain::operations::{SystemEvent, SystemEventKind};
use venom_domain::scanning::{
    CollectionScanScheduler, CompletedScanCommand, DueCollectionScan, FailedScanCommand,
    RunNextScanResult, ScanCommandStatus, ScanPlanner,
};

#[derive(Debug)]
pub struct PostgresStore {
    pool: PgPool,
    names: TableNames,
    ingestion: FindingIngestion,
    governance: FindingGovernance,
    read_model: FindingReadModel,
    inventory_snapshot_cache: Arc<ComponentInventory>,
    read_model_snapshot_cache: Arc<FindingReadModel>,
    release_board_snapshot_cache: Arc<ReleaseBoard>,
    integration_runtime_config: Option<IntegrationRuntimeConfig>,
    commands: BTreeMap<Box<str>, ScanCommandRecord>,
    order: Vec<Box<str>>,
    pending_integration_events: Vec<PendingIntegrationEvent>,
    system_event_index: SystemEventQueryIndex,
    system_event_index_snapshot_cache: Arc<SystemEventQueryIndex>,
    command_statuses_snapshot_cache: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
}

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

impl PostgresStore {
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
            governance: FindingGovernance::new(),
            read_model: FindingReadModel::new(),
            inventory_snapshot_cache: Arc::new(ComponentInventory::default()),
            read_model_snapshot_cache: Arc::new(FindingReadModel::new()),
            release_board_snapshot_cache: Arc::new(build_release_board(
                &ComponentInventory::default(),
                &FindingReadModel::new(),
            )),
            integration_runtime_config: None,
            commands: BTreeMap::new(),
            order: Vec::new(),
            pending_integration_events: Vec::new(),
            system_event_index: SystemEventQueryIndex::new(),
            system_event_index_snapshot_cache: Arc::new(SystemEventQueryIndex::new()),
            command_statuses_snapshot_cache: Arc::new(BTreeMap::new()),
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
            self.refresh_read_snapshot_caches();
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
            self.refresh_read_snapshot_caches();
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
        self.refresh_read_snapshot_caches();
        self.pending_integration_events
            .push(pending_integration_event);
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
        let mut candidate_read_model = self.read_model.clone();
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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres finding risk acceptance commit failed: {error}")
            })?;

            candidate_read_model.accept_risk(finding, acceptance);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.refresh_read_snapshot_caches();
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
        let (targeted, changed) = self
            .read_model
            .collect_bulk_governance_finding_refs_matching(&scope, query, |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::RiskAccepted(existing)) if existing == &acceptance
                )
            });

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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres risk acceptance batch commit failed: {error}")
            })?;

            for finding in &changed {
                self.governance
                    .accept_risk(finding.clone(), acceptance.clone());
                self.read_model
                    .accept_risk(finding.clone(), acceptance.clone());
            }
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
        let (targeted, changed) = self
            .read_model
            .collect_bulk_governance_finding_refs_matching(&scope, query, |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::RiskAccepted(existing)) if existing == &acceptance
                )
            });

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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres tag risk acceptance batch commit failed: {error}")
            })?;

            for finding in &changed {
                self.governance
                    .accept_risk(finding.clone(), acceptance.clone());
                self.read_model
                    .accept_risk(finding.clone(), acceptance.clone());
            }
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
        let mut candidate_read_model = self.read_model.clone();
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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres finding reopen commit failed: {error}"))?;

            candidate_read_model.reopen(&finding);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.refresh_read_snapshot_caches();
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
        let mut candidate_read_model = self.read_model.clone();
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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres finding suppression commit failed: {error}"))?;

            candidate_read_model.suppress(finding, suppression);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.refresh_read_snapshot_caches();
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
        let (targeted, changed_findings) = self
            .read_model
            .collect_bulk_governance_finding_refs_matching(&scope, query, |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::Suppressed(existing)) if existing == &suppression
                )
            });

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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres suppression batch commit failed: {error}"))?;

            for finding in &changed_findings {
                self.governance
                    .suppress(finding.clone(), suppression.clone());
                self.read_model
                    .suppress(finding.clone(), suppression.clone());
            }
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
        let (targeted, changed) = self
            .read_model
            .collect_bulk_governance_finding_refs_matching(&scope, query, |finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::Suppressed(existing)) if existing == &suppression
                )
            });

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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit().await.map_err(|error| {
                format!("postgres tag suppression batch commit failed: {error}")
            })?;

            for finding in &changed {
                self.governance
                    .suppress(finding.clone(), suppression.clone());
                self.read_model
                    .suppress(finding.clone(), suppression.clone());
            }
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
        let (targeted, reopened_findings) = self
            .read_model
            .collect_bulk_governance_finding_refs_matching(&scope, query, |finding| {
                self.governance.decision(finding).is_some()
            });

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
            self.insert_system_event_in_transaction(&mut tx, &event)
                .await?;
            tx.commit()
                .await
                .map_err(|error| format!("postgres reopen batch commit failed: {error}"))?;

            for finding in &reopened_findings {
                self.governance.reopen(finding);
                self.read_model.reopen(finding);
            }
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
    }

    #[must_use]
    pub fn system_event_index_snapshot_arc(&self) -> Arc<SystemEventQueryIndex> {
        Arc::clone(&self.system_event_index_snapshot_cache)
    }

    #[must_use]
    pub fn release_board_snapshot_arc(&self) -> Arc<ReleaseBoard> {
        Arc::clone(&self.release_board_snapshot_cache)
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
        self.insert_pending_scan_commands(
            &mut transaction,
            std::slice::from_ref(&command_id),
            std::slice::from_ref(&request),
        )
        .await
        .map_err(|error| format!("postgres scan command insert failed: {error}"))?;
        self.insert_system_events_in_transaction(&mut transaction, std::slice::from_ref(&event))
            .await?;
        self.commit_transaction(transaction).await?;

        self.order.push(command_id.clone());
        self.commands.insert(
            command_id.clone(),
            ScanCommandRecord {
                request,
                status: ScanCommandStatus::Pending,
            },
        );
        self.refresh_command_statuses_snapshot_cache();
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
        self.insert_pending_scan_commands(&mut transaction, &command_ids, &batch.requests)
            .await
            .map_err(|error| format!("postgres collection scan command insert failed: {error}"))?;
        self.insert_system_events_in_transaction(&mut transaction, &system_events)
            .await?;
        self.commit_transaction(transaction).await?;

        for ((command_id, request), event) in command_ids
            .iter()
            .cloned()
            .zip(batch.requests)
            .zip(system_events)
        {
            self.push_system_event(event);
            self.order.push(command_id.clone());
            self.commands.insert(
                command_id,
                ScanCommandRecord {
                    request,
                    status: ScanCommandStatus::Pending,
                },
            );
        }
        self.refresh_command_statuses_snapshot_cache();

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

        let (candidate_ingestion, due_scans, pending_due_remaining) =
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

        let schedule_rows =
            Self::build_due_schedule_rows(candidate_ingestion.inventory(), &due_scans);
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
        self.persist_due_collection_scans(
            &command_ids,
            &all_requests,
            &schedule_rows,
            &system_events,
        )
        .await?;

        self.apply_due_collection_scan_state(
            candidate_ingestion,
            system_events,
            &command_ids,
            all_requests,
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
    ) -> Result<(), String> {
        if requests.is_empty() {
            return Ok(());
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
        query_builder
            .build()
            .execute(&mut **transaction)
            .await
            .map_err(|error| format!("postgres due collection scan insert failed: {error}"))?;
        Ok(())
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
        self.persist_completed_scan_command(
            command_id.as_ref(),
            &report,
            &finding_changes_event,
            &scan_command_completed_event,
            &system_event,
        )
        .await?;

        self.ingestion = candidate_ingestion;
        self.read_model = candidate_read_model;
        self.refresh_read_snapshot_caches();
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
        );

        Ok(RunNextScanResult::Completed(completed))
    }

    fn collect_due_collection_scans(
        &self,
        now_unix_ms: u64,
        max_collections: usize,
    ) -> (FindingIngestion, Vec<DueCollectionScan>, usize) {
        let mut candidate_ingestion = self.ingestion.clone();
        let due_scans = CollectionScanScheduler::new(candidate_ingestion.inventory_mut())
            .collect_due(now_unix_ms, max_collections);
        let pending_due_remaining = candidate_ingestion
            .inventory()
            .due_collection_keys(now_unix_ms, usize::MAX)
            .len();
        (candidate_ingestion, due_scans, pending_due_remaining)
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
    ) -> Result<(), String> {
        let mut transaction = self.begin_transaction().await?;
        self.insert_pending_scan_commands(&mut transaction, command_ids, requests)
            .await?;
        self.upsert_collection_scan_schedules(&mut transaction, schedule_rows)
            .await?;
        self.insert_system_events_in_transaction(&mut transaction, system_events)
            .await?;
        self.commit_transaction(transaction).await
    }

    fn apply_due_collection_scan_state(
        &mut self,
        candidate_ingestion: FindingIngestion,
        system_events: Vec<SystemEvent>,
        command_ids: &[Box<str>],
        requests: Vec<ScanRequest>,
    ) {
        self.ingestion = candidate_ingestion;
        for event in system_events {
            self.push_system_event(event);
        }
        for (command_id, request) in command_ids.iter().cloned().zip(requests) {
            self.order.push(command_id.clone());
            self.commands.insert(
                command_id,
                ScanCommandRecord {
                    request,
                    status: ScanCommandStatus::Pending,
                },
            );
        }
        self.refresh_command_statuses_snapshot_cache();
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
        self.insert_system_event_in_transaction(&mut transaction, &system_event)
            .await?;
        self.commit_transaction(transaction).await?;

        self.remove_pending_integration_event(event_id);
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
        self.insert_system_event_in_transaction(&mut transaction, &system_event)
            .await?;
        self.commit_transaction(transaction).await?;

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
    ) -> Result<(), String> {
        let mut transaction = self.begin_transaction().await?;
        self.insert_provider_report(&mut transaction, report)
            .await?;
        self.insert_pending_integration_events(
            &mut transaction,
            &[
                finding_changes_event.clone(),
                scan_command_completed_event.clone(),
            ],
        )
        .await?;
        sqlx::query(&format!(
            concat!(
                "UPDATE {} ",
                "SET status = $2, updated_at = NOW() ",
                "WHERE command_id = $1"
            ),
            self.names.scan_commands
        ))
        .bind(command_id)
        .bind(scan_command_status_name(ScanCommandStatus::Completed))
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("postgres scan command completion failed: {error}"))?;
        self.insert_system_events_in_transaction(
            &mut transaction,
            std::slice::from_ref(system_event),
        )
        .await?;
        self.commit_transaction(transaction).await
    }

    fn apply_completed_scan_command(
        &mut self,
        completed: &CompletedScanCommand,
        finding_changes_event: PendingIntegrationEvent,
        scan_command_completed_event: PendingIntegrationEvent,
        system_event: SystemEvent,
    ) {
        self.pending_integration_events.push(finding_changes_event);
        self.pending_integration_events
            .push(scan_command_completed_event);
        let command = self
            .commands
            .get_mut(completed.command_id.as_ref())
            .expect("completed scan command missing from postgres runtime");
        command.status = ScanCommandStatus::Completed;
        self.refresh_command_statuses_snapshot_cache();
        self.push_system_event(system_event);
    }

    async fn insert_provider_report(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        report: &ProviderScanReport,
    ) -> Result<(), String> {
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
        .execute(&mut **transaction)
        .await
        .map_err(|error| format!("postgres provider report insert failed: {error}"))?;
        Ok(())
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
    ) -> Result<(), String> {
        for event in events {
            sqlx::query(&format!(
                concat!(
                    "INSERT INTO {} (event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                    "command_id, integration_event_id, finding_count, retryable, detail) ",
                    "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
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
            .execute(&mut **transaction)
            .await
            .map_err(|error| format!("postgres system event insert failed: {error}"))?;
        }
        Ok(())
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
        self.refresh_command_statuses_snapshot_cache();
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
        self.insert_system_event(&event).await?;
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
        self.create_scan_commands_table().await?;
        self.create_integration_outbox_table().await?;
        self.create_system_events_table().await?;

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
        self.read_model = FindingReadModel::new();
        self.integration_runtime_config = None;
        self.commands.clear();
        self.order.clear();
        self.pending_integration_events.clear();
        self.system_event_index = SystemEventQueryIndex::new();

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
        self.load_scan_commands().await?;
        self.load_pending_integration_events().await?;
        self.load_system_events().await?;
        self.refresh_read_snapshot_caches();
        self.refresh_command_statuses_snapshot_cache();
        self.refresh_system_event_index_snapshot_cache();

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
            self.read_model.replay_risk_acceptance(finding, acceptance);
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
            self.read_model.replay_suppression(finding, suppression);
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

    async fn load_system_events(&mut self) -> Result<(), String> {
        let rows = sqlx::query_as::<
            _,
            (
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
            ),
        >(&format!(
            concat!(
                "SELECT event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail ",
                "FROM {} ORDER BY occurred_at_unix_ms DESC, event_id DESC"
            ),
            self.names.system_events
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| format!("postgres system events load failed: {error}"))?;

        let events =
            rows.into_iter()
                .map(
                    |(
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
                    )| {
                        Ok(SystemEvent {
                            event_id: event_id.into_boxed_str(),
                            occurred_at_unix_ms: u64::try_from(occurred_at_unix_ms)
                                .map_err(|_| "negative system event timestamp".to_owned())?,
                            kind: parse_system_event_kind(&kind)?,
                            collection_key: collection_key.map(String::into_boxed_str),
                            component_key: component_key.map(String::into_boxed_str),
                            command_id: command_id.map(String::into_boxed_str),
                            integration_event_id: integration_event_id.map(String::into_boxed_str),
                            finding_count: finding_count.map(u32::try_from).transpose().map_err(
                                |_| "system event finding count out of range".to_owned(),
                            )?,
                            retryable,
                            detail: detail.map(String::into_boxed_str),
                        })
                    },
                )
                .collect::<Result<Vec<_>, String>>()?;
        self.system_event_index = SystemEventQueryIndex::from_newest_first(events.iter());
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

    fn push_system_event(&mut self, event: SystemEvent) {
        self.system_event_index.push_newest(event);
        self.refresh_read_snapshot_caches();
        self.refresh_system_event_index_snapshot_cache();
    }

    fn refresh_read_snapshot_caches(&mut self) {
        self.inventory_snapshot_cache = Arc::new(self.ingestion.inventory().clone());
        self.read_model_snapshot_cache = Arc::new(self.read_model.clone());
        self.release_board_snapshot_cache = Arc::new(build_release_board(
            self.ingestion.inventory(),
            &self.read_model,
        ));
    }

    fn refresh_system_event_index_snapshot_cache(&mut self) {
        self.system_event_index_snapshot_cache = Arc::new(self.system_event_index.clone());
    }

    fn refresh_command_statuses_snapshot_cache(&mut self) {
        self.command_statuses_snapshot_cache = Arc::new(
            self.commands
                .iter()
                .map(|(command_id, record)| (command_id.clone(), record.status))
                .collect(),
        );
    }

    async fn insert_system_event(&self, event: &SystemEvent) -> Result<(), String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| format!("postgres system event begin failed: {error}"))?;
        self.insert_system_event_in_transaction(&mut tx, event)
            .await?;
        tx.commit()
            .await
            .map_err(|error| format!("postgres system event commit failed: {error}"))?;
        Ok(())
    }

    async fn insert_system_event_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &SystemEvent,
    ) -> Result<(), String> {
        sqlx::query(&format!(
            concat!(
                "INSERT INTO {} (event_id, occurred_at_unix_ms, category, kind, collection_key, component_key, ",
                "command_id, integration_event_id, finding_count, retryable, detail) ",
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
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
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("postgres system event insert failed: {error}"))?;
        Ok(())
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
    scan_commands: Box<str>,
    integration_outbox: Box<str>,
    system_events: Box<str>,
}

impl TableNames {
    fn new(schema: &str) -> Result<Self, String> {
        let schema = validate_schema_name(schema)?;
        Ok(Self {
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
    use super::PostgresStore;
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
