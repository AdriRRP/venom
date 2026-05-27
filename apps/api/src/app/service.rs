use crate::infra::http_integration_publisher::{HTTP_EVENT_PUBLISHER_KEY, HttpEventPublisher};
use crate::infra::postgres_backend::{DrainDueCollectionScansResult, PostgresStore};
pub use crate::infra::postgres_backend::{PostgresReadSnapshotLoader, PostgresRemoteChangeProbe};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use venom_domain::durable_state::DurableState;
use venom_domain::findings::{
    AcceptRiskResult, ActiveFindingsQuery, ArtifactKind, ArtifactRef, BulkAcceptRiskResult,
    BulkGovernanceQuery, BulkReopenFindingResult, BulkSuppressFindingResult,
    CollectionHealthSummary, ContextualActiveFindingProjection, EvidenceFreshness,
    FindingGovernanceState, FindingProvider, FindingProviderError, FindingProviderErrorKind,
    FindingReadModel, FindingRef, PackageCoordinate, ProviderScanReport, ReleaseBoard,
    ReleaseDashboard, ReopenFindingResult, ReportedFinding, RiskAcceptance, ScanRequest,
    ScopedActiveFindingsQuery, Severity, SuppressFindingResult, Suppression, build_release_board,
    build_release_dashboard, contextualize_active_findings,
    contextualize_collection_active_findings, query_collection_governance_overview,
};
use venom_domain::integration::{
    IntegrationEventPublishError, IntegrationEventPublisher, IntegrationRuntimeConfig,
    PendingIntegrationEvent, PublishIntegrationEventsResult,
};
use venom_domain::inventory::{
    CollectionRegistration, CollectionSource, CollectionSourceKind, CollectionSourceMode,
    CollectionSourceSummary, ComponentInventory, ComponentListCollectionSource,
    ComponentRegistration, ComponentTagRegistration, ContextProfileRegistration,
    ManagedComponentTag, ManagedContextProfile,
};
use venom_domain::operations::system_event_trace::SystemEventQueryIndex;
use venom_domain::operations::{
    SystemEvent, SystemEventCategory, SystemEventsPage, SystemEventsQuery,
};
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

#[derive(Debug)]
pub struct ApiReadSnapshot {
    inventory: Arc<ComponentInventory>,
    read_model: Arc<FindingReadModel>,
    read_model_source_watermark: u64,
    system_event_index: Arc<SystemEventQueryIndex>,
    command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
    release_board: Mutex<Option<Arc<ReleaseBoard>>>,
}

impl ApiReadSnapshot {
    #[must_use]
    pub const fn new(
        inventory: Arc<ComponentInventory>,
        read_model: Arc<FindingReadModel>,
        read_model_source_watermark: u64,
        system_event_index: Arc<SystemEventQueryIndex>,
        command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
    ) -> Self {
        Self {
            inventory,
            read_model,
            read_model_source_watermark,
            system_event_index,
            command_statuses,
            release_board: Mutex::new(None),
        }
    }

    #[must_use]
    pub fn with_inventory_arc(&self, inventory: Arc<ComponentInventory>) -> Self {
        Self {
            inventory,
            read_model: Arc::clone(&self.read_model),
            read_model_source_watermark: self.read_model_source_watermark,
            system_event_index: Arc::clone(&self.system_event_index),
            command_statuses: Arc::clone(&self.command_statuses),
            release_board: Mutex::new(None),
        }
    }

    #[must_use]
    pub fn with_read_model_arc(
        &self,
        read_model: Arc<FindingReadModel>,
        read_model_source_watermark: u64,
    ) -> Self {
        Self {
            inventory: Arc::clone(&self.inventory),
            read_model,
            read_model_source_watermark,
            system_event_index: Arc::clone(&self.system_event_index),
            command_statuses: Arc::clone(&self.command_statuses),
            release_board: Mutex::new(None),
        }
    }

    #[must_use]
    pub fn with_system_event_index_arc(
        &self,
        system_event_index: Arc<SystemEventQueryIndex>,
    ) -> Self {
        Self {
            inventory: Arc::clone(&self.inventory),
            read_model: Arc::clone(&self.read_model),
            read_model_source_watermark: self.read_model_source_watermark,
            system_event_index,
            command_statuses: Arc::clone(&self.command_statuses),
            release_board: Mutex::new(None),
        }
    }

    #[must_use]
    pub fn with_command_statuses_arc(
        &self,
        command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
    ) -> Self {
        Self {
            inventory: Arc::clone(&self.inventory),
            read_model: Arc::clone(&self.read_model),
            read_model_source_watermark: self.read_model_source_watermark,
            system_event_index: Arc::clone(&self.system_event_index),
            command_statuses,
            release_board: Mutex::new(None),
        }
    }

    fn release_board_arc(&self) -> Arc<ReleaseBoard> {
        let mut cache = self
            .release_board
            .lock()
            .expect("release board cache should not be poisoned");
        if let Some(board) = cache.as_ref() {
            return Arc::clone(board);
        }

        let board = Arc::new(build_release_board(&self.inventory, &self.read_model));
        *cache = Some(Arc::clone(&board));
        board
    }

    #[must_use]
    pub(crate) fn inventory_arc(&self) -> Arc<ComponentInventory> {
        Arc::clone(&self.inventory)
    }

    #[must_use]
    pub(crate) fn read_model_arc(&self) -> Arc<FindingReadModel> {
        Arc::clone(&self.read_model)
    }

    #[must_use]
    pub(crate) const fn read_model_source_watermark(&self) -> u64 {
        self.read_model_source_watermark
    }

    #[must_use]
    pub(crate) fn system_event_index_arc(&self) -> Arc<SystemEventQueryIndex> {
        Arc::clone(&self.system_event_index)
    }

    #[must_use]
    pub(crate) fn command_statuses_arc(&self) -> Arc<BTreeMap<Box<str>, ScanCommandStatus>> {
        Arc::clone(&self.command_statuses)
    }

    /// Query the current system-event trace from the indexed read-side snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid.
    pub fn list_system_events(
        &self,
        request: &ListSystemEventsRequest,
    ) -> Result<ListSystemEventsResponse, ApiApplicationError> {
        let query = build_system_events_query(request)?;
        let category = query.category.map(|value| value.as_str().to_owned());
        let mut response =
            ListSystemEventsResponse::from_page(self.system_event_index.query(&query));
        response.category = category;
        Ok(response)
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
        let findings = contextualize_active_findings(&self.inventory, page.findings)
            .into_iter()
            .map(ActiveFindingItem::from_projection)
            .collect::<Vec<_>>();

        Ok(ActiveFindingsResponse {
            component_key: request.component_key,
            artifact_kind: request.artifact_kind,
            artifact_identity: request.artifact_identity,
            min_severity: request.min_severity,
            governance_state: request.governance_state,
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
        let release_board = self.release_board_arc();
        let collections = release_board
            .collections
            .iter()
            .map(|collection| CollectionSummary {
                collection_key: collection.collection_key.to_string(),
                name: collection.name.to_string(),
                members: collection.members,
                source: collection.source.map(CollectionSourceSummaryItem::from),
                scan_schedule: collection
                    .scan_schedule
                    .map(CollectionScanScheduleItem::from),
                due_now: collection
                    .scan_schedule
                    .is_some_and(|schedule| schedule.next_due_at_unix_ms <= now_unix_ms),
                health: CollectionHealthItem::from(collection.health),
            })
            .collect::<Vec<_>>();
        let managed_collections = collections.len();
        Ok(ListCollectionsResponse {
            managed_collections,
            collections,
        })
    }

    /// Query one executive release dashboard over managed collections.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the current system time cannot be read.
    pub fn release_dashboard(&self) -> Result<ReleaseDashboardResponse, ApiApplicationError> {
        let now_unix_ms = current_unix_millis()?;
        Ok(ReleaseDashboardResponse::from_dashboard(
            build_release_dashboard(&self.release_board_arc(), now_unix_ms),
        ))
    }

    /// Query one durable scan command status from the compact read-side snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError::NotFound`] when the command is unknown.
    pub fn scan_command_status(
        &self,
        command_id: &str,
    ) -> Result<ScanCommandStatusResponse, ApiApplicationError> {
        let status = self
            .command_statuses
            .get(command_id)
            .copied()
            .ok_or_else(|| {
                ApiApplicationError::NotFound(format!("unknown scan command: {command_id}"))
            })?;

        Ok(ScanCommandStatusResponse {
            command_id: command_id.to_owned(),
            status: status.as_str().to_owned(),
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
            context_profile_key: collection.context_profile_key.map(Into::into),
            source: collection.source.map(CollectionSourceItem::from),
            scan_schedule: collection
                .scan_schedule
                .map(CollectionScanScheduleItem::from),
            health: self.collection_health_summary(collection_key),
            members: collection
                .component_keys
                .into_iter()
                .map(|component_key| {
                    let effective_context = self
                        .inventory
                        .managed_component_effective_context_in_collection(
                            collection_key,
                            component_key.as_ref(),
                        );
                    CollectionMemberItem {
                        context_profile_key: effective_context
                            .as_ref()
                            .and_then(|context| context.singular_profile())
                            .map(|profile| profile.profile_key.to_string()),
                        component_context_profile: effective_context
                            .as_ref()
                            .and_then(|context| context.component_profile.clone())
                            .map(ContextProfileRefItem::from),
                        collection_context_profile: effective_context
                            .as_ref()
                            .and_then(|context| context.collection_profile.clone())
                            .map(ContextProfileRefItem::from),
                        tag_context_profiles: effective_context
                            .map(|context| {
                                context
                                    .tag_profiles
                                    .into_iter()
                                    .map(ContextProfileRefItem::from)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        tag_keys: self
                            .inventory
                            .component_tag_keys(component_key.as_ref())
                            .unwrap_or_default()
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                        key: component_key.into(),
                    }
                })
                .collect(),
        })
    }

    /// Query the operator-facing catalog of managed execution-context profiles.
    #[must_use]
    pub fn list_context_profiles(&self) -> ListContextProfilesResponse {
        let profiles = self
            .inventory
            .context_profiles()
            .into_iter()
            .map(ContextProfileItem::from)
            .collect::<Vec<_>>();
        ListContextProfilesResponse {
            managed_context_profiles: profiles.len(),
            profiles,
        }
    }

    /// Query the operator-facing catalog of managed component tags.
    #[must_use]
    pub fn list_component_tags(&self) -> ListComponentTagsResponse {
        let tags = self
            .inventory
            .component_tags()
            .into_iter()
            .map(ComponentTagItem::from)
            .collect::<Vec<_>>();
        ListComponentTagsResponse {
            managed_component_tags: tags.len(),
            tags,
        }
    }

    /// Query active findings over one closed managed collection scope.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError::NotFound`] when the collection is unknown
    /// or [`ApiApplicationError`] when one requested severity is unsupported.
    pub fn list_collection_active_findings(
        &self,
        collection_key: &str,
        request: CollectionActiveFindingsRequest,
    ) -> Result<CollectionActiveFindingsResponse, ApiApplicationError> {
        let query = build_scoped_active_findings_query(&request)?;
        let overview = query_collection_governance_overview(
            &self.inventory,
            &self.read_model,
            collection_key,
            &query,
        )
        .ok_or_else(|| {
            ApiApplicationError::NotFound(format!("unknown collection: {collection_key}"))
        })?;
        let findings = contextualize_collection_active_findings(
            &self.inventory,
            collection_key,
            overview.page.findings,
        )
        .into_iter()
        .map(CollectionActiveFindingItem::from_projection)
        .collect::<Vec<_>>();

        Ok(CollectionActiveFindingsResponse {
            collection_key: collection_key.to_owned(),
            min_severity: request.min_severity,
            governance_state: request.governance_state,
            package_name: request.package_name,
            health: CollectionHealthItem::from(overview.health),
            bulk_governance: BulkGovernanceCohortItem::from(overview.bulk_governance),
            total_active_findings: overview.page.total,
            returned: overview.page.returned,
            offset: overview.page.offset,
            limit: overview.page.limit,
            active_findings: findings,
        })
    }

    fn collection_health_summary(&self, collection_key: &str) -> CollectionHealthItem {
        self.release_board_arc()
            .collections
            .iter()
            .find(|collection| collection.collection_key.as_ref() == collection_key)
            .map_or_else(CollectionHealthItem::default, |collection| {
                CollectionHealthItem::from(collection.health)
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
    state_path: PathBuf,
    runtime_path: PathBuf,
    state: DurableState,
    runtime: ScanCommandQueue,
    merged_system_event_snapshot_cache: StdMutex<Option<MergedSystemEventSnapshot>>,
}

struct MergedSystemEventSnapshot {
    state: Arc<SystemEventQueryIndex>,
    runtime: Arc<SystemEventQueryIndex>,
    state_windows: venom_domain::SystemEventRecentWindows,
    runtime_windows: venom_domain::SystemEventRecentWindows,
    merged: Arc<SystemEventQueryIndex>,
}

impl LocalStore {
    fn system_event_index_snapshot_arc(&self) -> Arc<SystemEventQueryIndex> {
        let state = self.state.system_event_index_snapshot_arc();
        let runtime = self.runtime.system_event_index_snapshot_arc();
        let mut cache = self
            .merged_system_event_snapshot_cache
            .lock()
            .expect("merged local system event cache should not be poisoned");
        if let Some(snapshot) = cache.as_ref()
            && Arc::ptr_eq(&snapshot.state, &state)
            && Arc::ptr_eq(&snapshot.runtime, &runtime)
        {
            return Arc::clone(&snapshot.merged);
        }

        let (state_windows, runtime_windows) = match cache.as_ref() {
            Some(snapshot) if Arc::ptr_eq(&snapshot.state, &state) => {
                (snapshot.state_windows.clone(), runtime.recent_windows())
            }
            Some(snapshot) if Arc::ptr_eq(&snapshot.runtime, &runtime) => {
                (state.recent_windows(), snapshot.runtime_windows.clone())
            }
            _ => (state.recent_windows(), runtime.recent_windows()),
        };
        let merged = Arc::new(SystemEventQueryIndex::from_recent_windows(
            merge_system_event_window_totals(&state.window_totals(), &runtime.window_totals()),
            merge_system_event_recent_windows(&state_windows, &runtime_windows),
        ));
        *cache = Some(MergedSystemEventSnapshot {
            state,
            runtime,
            state_windows,
            runtime_windows,
            merged: Arc::clone(&merged),
        });
        merged
    }
}

const fn merge_system_event_window_totals(
    left: &venom_domain::SystemEventWindowTotals,
    right: &venom_domain::SystemEventWindowTotals,
) -> venom_domain::SystemEventWindowTotals {
    venom_domain::SystemEventWindowTotals {
        total: left.total + right.total,
        scheduler_total: left.scheduler_total + right.scheduler_total,
        command_total: left.command_total + right.command_total,
        governance_total: left.governance_total + right.governance_total,
        publication_total: left.publication_total + right.publication_total,
    }
}

fn merge_system_event_recent_windows(
    left: &venom_domain::SystemEventRecentWindows,
    right: &venom_domain::SystemEventRecentWindows,
) -> venom_domain::SystemEventRecentWindows {
    venom_domain::SystemEventRecentWindows {
        recent_events: merge_recent_arc_events(&left.recent_events, &right.recent_events),
        recent_scheduler_events: merge_recent_arc_events(
            &left.recent_scheduler_events,
            &right.recent_scheduler_events,
        ),
        recent_command_events: merge_recent_arc_events(
            &left.recent_command_events,
            &right.recent_command_events,
        ),
        recent_governance_events: merge_recent_arc_events(
            &left.recent_governance_events,
            &right.recent_governance_events,
        ),
        recent_publication_events: merge_recent_arc_events(
            &left.recent_publication_events,
            &right.recent_publication_events,
        ),
    }
}

fn merge_recent_arc_events(
    left: &[Arc<SystemEvent>],
    right: &[Arc<SystemEvent>],
) -> Vec<Arc<SystemEvent>> {
    let mut merged = Vec::with_capacity(
        (left.len() + right.len()).min(venom_domain::operations::MAX_SYSTEM_EVENTS_LIMIT),
    );
    let mut left_index = 0;
    let mut right_index = 0;

    while merged.len() < venom_domain::operations::MAX_SYSTEM_EVENTS_LIMIT
        && (left_index < left.len() || right_index < right.len())
    {
        let take_left = match (left.get(left_index), right.get(right_index)) {
            (Some(left_event), Some(right_event)) => {
                compare_recent_system_event_order(left_event, right_event).is_lt()
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if take_left {
            merged.push(Arc::clone(
                left.get(left_index)
                    .expect("left recent event should exist for merge"),
            ));
            left_index += 1;
        } else {
            merged.push(Arc::clone(
                right
                    .get(right_index)
                    .expect("right recent event should exist for merge"),
            ));
            right_index += 1;
        }
    }

    merged
}

fn compare_recent_system_event_order(
    left: &SystemEvent,
    right: &SystemEvent,
) -> std::cmp::Ordering {
    right
        .occurred_at_unix_ms
        .cmp(&left.occurred_at_unix_ms)
        .then_with(|| left.event_id.cmp(&right.event_id))
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
        let state_path = state_path.into();
        let runtime_path = runtime_path.into();
        let state = DurableState::open(state_path.clone())
            .map_err(|error| ApiApplicationError::State(error.to_string()))?;
        let runtime = ScanCommandQueue::open(runtime_path.clone())
            .map_err(|error| ApiApplicationError::State(error.to_string()))?;
        Ok(Self {
            backend: ApiStore::Local(LocalStore {
                state_path,
                runtime_path,
                state,
                runtime,
                merged_system_event_snapshot_cache: StdMutex::new(None),
            }),
        })
    }

    #[must_use]
    pub const fn from_postgres_store(backend: PostgresStore) -> Self {
        Self {
            backend: ApiStore::Postgres(backend),
        }
    }

    #[must_use]
    pub fn remote_change_probe(&self) -> Option<PostgresRemoteChangeProbe> {
        match &self.backend {
            ApiStore::Local(_) => None,
            ApiStore::Postgres(postgres) => Some(postgres.remote_change_probe()),
        }
    }

    #[must_use]
    pub fn remote_read_snapshot_loader(&self) -> Option<PostgresReadSnapshotLoader> {
        match &self.backend {
            ApiStore::Local(_) => None,
            ApiStore::Postgres(postgres) => Some(postgres.read_snapshot_loader()),
        }
    }

    #[must_use]
    pub fn read_snapshot(&self) -> ApiReadSnapshot {
        match &self.backend {
            ApiStore::Local(local) => ApiReadSnapshot::new(
                local.state.inventory_snapshot_arc(),
                local.state.read_model_snapshot_arc(),
                0,
                local.system_event_index_snapshot_arc(),
                local.runtime.command_statuses_snapshot_arc(),
            ),
            ApiStore::Postgres(postgres) => ApiReadSnapshot::new(
                postgres.inventory_snapshot_arc(),
                postgres.read_model_snapshot_arc(),
                postgres.read_model_source_watermark(),
                postgres.system_event_index_snapshot_arc(),
                postgres.command_statuses_snapshot_arc(),
            ),
        }
    }

    #[must_use]
    pub fn inventory_snapshot_arc(&self) -> Arc<ComponentInventory> {
        match &self.backend {
            ApiStore::Local(local) => local.state.inventory_snapshot_arc(),
            ApiStore::Postgres(postgres) => postgres.inventory_snapshot_arc(),
        }
    }

    #[must_use]
    pub fn read_model_snapshot_arc(&self) -> Arc<FindingReadModel> {
        match &self.backend {
            ApiStore::Local(local) => local.state.read_model_snapshot_arc(),
            ApiStore::Postgres(postgres) => postgres.read_model_snapshot_arc(),
        }
    }

    #[must_use]
    pub const fn read_model_source_watermark(&self) -> u64 {
        match &self.backend {
            ApiStore::Local(_) => 0,
            ApiStore::Postgres(postgres) => postgres.read_model_source_watermark(),
        }
    }

    #[must_use]
    pub fn system_event_index_snapshot_arc(&self) -> Arc<SystemEventQueryIndex> {
        match &self.backend {
            ApiStore::Local(local) => local.system_event_index_snapshot_arc(),
            ApiStore::Postgres(postgres) => postgres.system_event_index_snapshot_arc(),
        }
    }

    #[must_use]
    pub fn command_statuses_snapshot_arc(&self) -> Arc<BTreeMap<Box<str>, ScanCommandStatus>> {
        match &self.backend {
            ApiStore::Local(local) => local.runtime.command_statuses_snapshot_arc(),
            ApiStore::Postgres(postgres) => postgres.command_statuses_snapshot_arc(),
        }
    }
    /// Refresh one Postgres-backed in-memory view when the durable store advanced in another instance.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the Postgres-backed reload fails.
    pub async fn refresh_from_remote_if_stale(&mut self) -> Result<bool, ApiApplicationError> {
        match &mut self.backend {
            ApiStore::Local(_) => Ok(false),
            ApiStore::Postgres(postgres) => postgres
                .refresh_from_remote_if_stale()
                .await
                .map_err(ApiApplicationError::State),
        }
    }

    /// Mark the current Postgres durable change watermark as already observed locally.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the Postgres watermark cannot be read.
    pub async fn mark_remote_change_observed(&self) -> Result<(), ApiApplicationError> {
        match &self.backend {
            ApiStore::Local(_) => Ok(()),
            ApiStore::Postgres(postgres) => postgres
                .mark_remote_change_observed()
                .await
                .map_err(ApiApplicationError::State),
        }
    }

    #[must_use]
    pub fn observed_remote_change_watermark(&self) -> Option<u64> {
        match &self.backend {
            ApiStore::Local(_) => None,
            ApiStore::Postgres(postgres) => Some(postgres.observed_change_watermark()),
        }
    }

    /// Refresh one local file-backed application view from appended durable history.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the local durable state or runtime
    /// cannot be reopened from disk.
    pub fn refresh_local_from_disk(&mut self) -> Result<(), ApiApplicationError> {
        match &mut self.backend {
            ApiStore::Local(local) => {
                if local.state.sync_from_history_tail().is_err() {
                    local.state = DurableState::open(local.state_path.clone())
                        .map_err(|error| ApiApplicationError::State(error.to_string()))?;
                }
                if local.runtime.sync_from_history_tail().is_err() {
                    local.runtime = ScanCommandQueue::open(local.runtime_path.clone())
                        .map_err(|error| ApiApplicationError::State(error.to_string()))?;
                }
                *local
                    .merged_system_event_snapshot_cache
                    .lock()
                    .expect("merged local system event cache should not be poisoned") = None;
                Ok(())
            }
            ApiStore::Postgres(_) => Ok(()),
        }
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

    /// Register one reusable execution-context profile.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn register_context_profile(
        &mut self,
        request: ContextProfileRegistrationRequest,
    ) -> Result<RegisterContextProfileResponse, ApiApplicationError> {
        let mut registration =
            ContextProfileRegistration::overlay(request.profile_key, request.name);
        if let Some(value) = request.internet_exposed {
            registration = registration.with_internet_exposed(value);
        }
        if let Some(value) = request.production {
            registration = registration.with_production(value);
        }
        if let Some(value) = request.mission_critical {
            registration = registration.with_mission_critical(value);
        }
        if let Some(value) = request.vpn_restricted {
            registration = registration.with_vpn_restricted(value);
        }
        if let Some(value) = request.non_privileged_user {
            registration = registration.with_non_privileged_user(value);
        }
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .register_context_profile(registration)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .register_context_profile(registration)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(RegisterContextProfileResponse {
            change: result.change.as_str().to_owned(),
            managed_context_profiles: result.managed_context_profiles,
        })
    }

    /// Register one reusable transversal component tag.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn register_component_tag(
        &mut self,
        request: ComponentTagRegistrationRequest,
    ) -> Result<RegisterComponentTagResponse, ApiApplicationError> {
        let registration = ComponentTagRegistration::new(request.tag_key, request.name);
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .register_component_tag(registration)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .register_component_tag(registration)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(RegisterComponentTagResponse {
            change: result.change.as_str().to_owned(),
            managed_component_tags: result.managed_component_tags,
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

    /// Add one managed component to one managed component tag.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn add_component_to_tag(
        &mut self,
        tag_key: &str,
        request: ComponentTagMembershipRequest,
    ) -> Result<ComponentTagMembershipResponse, ApiApplicationError> {
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .assign_component_tag(tag_key, &request.component_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .assign_component_tag(tag_key, &request.component_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(ComponentTagMembershipResponse {
            change: result.change.as_str().to_owned(),
            members: result.members,
            conflict: result.conflict.map(ComponentTagConflictItem::from),
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

    /// Assign one managed context profile to one managed component tag.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable state write fails.
    pub async fn assign_context_profile_for_tag(
        &mut self,
        tag_key: &str,
        request: AssignTagContextProfileRequest,
    ) -> Result<AssignTagContextProfileResponse, ApiApplicationError> {
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .assign_context_profile_for_tag(tag_key, &request.profile_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .assign_context_profile_for_tag(tag_key, &request.profile_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(AssignTagContextProfileResponse {
            change: result.change.as_str().to_owned(),
            profile_key: result.profile_key.map(Into::into),
            conflict: result.conflict.map(ComponentTagConflictItem::from),
        })
    }

    /// Configure one declared source for one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the source request is invalid or the durable write fails.
    pub async fn configure_collection_source(
        &mut self,
        collection_key: &str,
        request: ConfigureCollectionSourceRequest,
    ) -> Result<ConfigureCollectionSourceResponse, ApiApplicationError> {
        let source = parse_collection_source(request)?;
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .configure_collection_source(collection_key, source)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .configure_collection_source(collection_key, source)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(ConfigureCollectionSourceResponse {
            change: result.change.as_str().to_owned(),
            source: result.source.map(CollectionSourceItem::from),
        })
    }

    /// Materialize one declared source into collection membership.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable write fails.
    pub async fn materialize_collection_source(
        &mut self,
        collection_key: &str,
    ) -> Result<MaterializeCollectionSourceResponse, ApiApplicationError> {
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .materialize_collection_source(collection_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .materialize_collection_source(collection_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(MaterializeCollectionSourceResponse {
            change: result.change.as_str().to_owned(),
            members: result.members,
            added: result.added,
            removed: result.removed,
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

    /// Assign one managed execution-context profile to one managed component.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable write fails.
    pub async fn assign_context_profile(
        &mut self,
        component_key: &str,
        request: AssignContextProfileRequest,
    ) -> Result<AssignContextProfileResponse, ApiApplicationError> {
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .assign_context_profile(component_key, &request.profile_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .assign_context_profile(component_key, &request.profile_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(AssignContextProfileResponse {
            change: result.change.as_str().to_owned(),
            profile_key: result.profile_key.map(Into::into),
        })
    }

    /// Assign one managed execution-context profile across one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the durable write fails.
    pub async fn assign_collection_context_profile(
        &mut self,
        collection_key: &str,
        request: AssignCollectionContextProfileRequest,
    ) -> Result<AssignCollectionContextProfileResponse, ApiApplicationError> {
        let result = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .assign_context_profile_for_collection(collection_key, &request.profile_key)
                .map_err(|error| ApiApplicationError::State(error.to_string()))?,
            ApiStore::Postgres(postgres) => postgres
                .assign_context_profile_for_collection(collection_key, &request.profile_key)
                .await
                .map_err(ApiApplicationError::State)?,
        };

        Ok(AssignCollectionContextProfileResponse {
            change: result.change.as_str().to_owned(),
            profile_key: result.profile_key.map(Into::into),
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

    /// Accept the risk of one currently active finding.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the finding
    /// is not active, or the durable write fails.
    pub async fn accept_risk(
        &mut self,
        request: AcceptRiskRequest,
    ) -> Result<AcceptRiskResponse, ApiApplicationError> {
        let finding = build_finding_ref(
            &request.component_key,
            &request.artifact_kind,
            &request.artifact_identity,
            &request.vulnerability_id,
            &request.package_name,
            &request.package_version,
            request.package_purl.as_deref(),
        )?;
        if request.reason.trim().is_empty() {
            return Err(ApiApplicationError::InvalidRequest(
                "risk acceptance reason must not be empty".to_owned(),
            ));
        }
        let acceptance = request.until_unix_ms.map_or_else(
            || RiskAcceptance::new(request.reason.clone()),
            |until_unix_ms| {
                RiskAcceptance::new(request.reason.clone()).until_unix_ms(until_unix_ms)
            },
        );

        let result: AcceptRiskResult = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .accept_risk(finding, acceptance.clone())
                .map_err(|error| match error {
                    venom_domain::DurableStateError::MissingFinding(_) => {
                        ApiApplicationError::NotFound(error.to_string())
                    }
                    _ => ApiApplicationError::State(error.to_string()),
                })?,
            ApiStore::Postgres(postgres) => postgres
                .accept_risk(finding, acceptance.clone())
                .await
                .map_err(|error| {
                    if error == "cannot accept risk for an inactive finding" {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?,
        };

        Ok(AcceptRiskResponse {
            change: result.change.as_str().to_owned(),
            governance_state: "risk-accepted".to_owned(),
            governance_reason: result.acceptance.reason.into(),
            governance_until_unix_ms: result.acceptance.until_unix_ms,
        })
    }

    /// Accept risk for all open active findings matched inside one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the collection
    /// is unknown, or the durable write fails.
    pub async fn accept_risk_for_collection(
        &mut self,
        collection_key: &str,
        request: BulkAcceptRiskRequest,
    ) -> Result<BulkAcceptRiskResponse, ApiApplicationError> {
        if request.reason.trim().is_empty() {
            return Err(ApiApplicationError::InvalidRequest(
                "risk acceptance reason must not be empty".to_owned(),
            ));
        }
        let acceptance = request.until_unix_ms.map_or_else(
            || RiskAcceptance::new(request.reason.clone()),
            |until_unix_ms| {
                RiskAcceptance::new(request.reason.clone()).until_unix_ms(until_unix_ms)
            },
        );
        let query = build_bulk_collection_governance_query(
            request.min_severity.as_deref(),
            request.package_name.as_deref(),
        )?;

        let result: BulkAcceptRiskResult = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .accept_risk_for_collection(collection_key, &query, acceptance.clone())
                .map_err(|error| match error {
                    venom_domain::DurableStateError::MissingCollection(reason) => {
                        ApiApplicationError::NotFound(reason.into())
                    }
                    _ => ApiApplicationError::State(error.to_string()),
                })?,
            ApiStore::Postgres(postgres) => postgres
                .accept_risk_for_collection(collection_key, &query, acceptance.clone())
                .await
                .map_err(|error| {
                    if error.starts_with("unknown collection:") {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?,
        };

        Ok(BulkAcceptRiskResponse {
            collection_key: collection_key.to_owned(),
            min_severity: request.min_severity,
            package_name: request.package_name,
            targeted: result.targeted,
            accepted: result.accepted,
            unchanged: result.unchanged,
            governance_state: "risk-accepted".to_owned(),
            governance_reason: result.acceptance.reason.into(),
            governance_until_unix_ms: result.acceptance.until_unix_ms,
        })
    }

    /// Accept risk for one filtered open cohort inside one managed component tag.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the tag is
    /// unknown, or the durable write fails.
    pub async fn accept_risk_for_tag(
        &mut self,
        tag_key: &str,
        request: BulkAcceptRiskRequest,
    ) -> Result<BulkAcceptRiskByTagResponse, ApiApplicationError> {
        if request.reason.trim().is_empty() {
            return Err(ApiApplicationError::InvalidRequest(
                "risk acceptance reason must not be empty".to_owned(),
            ));
        }
        let acceptance = request.until_unix_ms.map_or_else(
            || RiskAcceptance::new(request.reason.clone()),
            |until_unix_ms| {
                RiskAcceptance::new(request.reason.clone()).until_unix_ms(until_unix_ms)
            },
        );
        let query = build_bulk_collection_governance_query(
            request.min_severity.as_deref(),
            request.package_name.as_deref(),
        )?;

        let result: BulkAcceptRiskResult = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .accept_risk_for_tag(tag_key, &query, acceptance.clone())
                .map_err(|error| match error {
                    venom_domain::DurableStateError::MissingTag(reason) => {
                        ApiApplicationError::NotFound(reason.into())
                    }
                    _ => ApiApplicationError::State(error.to_string()),
                })?,
            ApiStore::Postgres(postgres) => postgres
                .accept_risk_for_tag(tag_key, &query, acceptance.clone())
                .await
                .map_err(|error| {
                    if error.starts_with("unknown tag:") {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?,
        };

        Ok(BulkAcceptRiskByTagResponse {
            tag_key: tag_key.to_owned(),
            min_severity: request.min_severity,
            package_name: request.package_name,
            targeted: result.targeted,
            accepted: result.accepted,
            unchanged: result.unchanged,
            governance_state: "risk-accepted".to_owned(),
            governance_reason: result.acceptance.reason.into(),
            governance_until_unix_ms: result.acceptance.until_unix_ms,
        })
    }

    /// Suppress one filtered open cohort of findings inside one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the collection
    /// is unknown, or the durable write fails.
    pub async fn suppress_findings_for_collection(
        &mut self,
        collection_key: &str,
        request: BulkSuppressFindingsRequest,
    ) -> Result<BulkSuppressFindingsResponse, ApiApplicationError> {
        if request.reason.trim().is_empty() {
            return Err(ApiApplicationError::InvalidRequest(
                "suppression reason must not be empty".to_owned(),
            ));
        }
        let query = build_bulk_collection_governance_query(
            request.min_severity.as_deref(),
            request.package_name.as_deref(),
        )?;
        let suppression = Suppression::new(request.reason.clone());

        let result: BulkSuppressFindingResult = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .suppress_findings_for_collection(collection_key, &query, suppression.clone())
                .map_err(|error| match error {
                    venom_domain::DurableStateError::MissingCollection(_) => {
                        ApiApplicationError::NotFound(error.to_string())
                    }
                    _ => ApiApplicationError::State(error.to_string()),
                })?,
            ApiStore::Postgres(postgres) => postgres
                .suppress_findings_for_collection(collection_key, &query, suppression.clone())
                .await
                .map_err(|error| {
                    if error.starts_with("unknown collection:") {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?,
        };

        Ok(BulkSuppressFindingsResponse {
            collection_key: collection_key.to_owned(),
            min_severity: request.min_severity,
            package_name: request.package_name,
            targeted: result.targeted,
            suppressed: result.suppressed,
            unchanged: result.unchanged,
            governance_state: "suppressed".to_owned(),
            governance_reason: result.suppression.reason.into(),
            governance_until_unix_ms: None,
        })
    }

    /// Suppress one filtered open cohort of findings inside one managed component tag.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the tag is
    /// unknown, or the durable write fails.
    pub async fn suppress_findings_for_tag(
        &mut self,
        tag_key: &str,
        request: BulkSuppressFindingsRequest,
    ) -> Result<BulkSuppressFindingsByTagResponse, ApiApplicationError> {
        if request.reason.trim().is_empty() {
            return Err(ApiApplicationError::InvalidRequest(
                "suppression reason must not be empty".to_owned(),
            ));
        }
        let query = build_bulk_collection_governance_query(
            request.min_severity.as_deref(),
            request.package_name.as_deref(),
        )?;
        let suppression = Suppression::new(request.reason.clone());

        let result: BulkSuppressFindingResult = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .suppress_findings_for_tag(tag_key, &query, suppression.clone())
                .map_err(|error| match error {
                    venom_domain::DurableStateError::MissingTag(reason) => {
                        ApiApplicationError::NotFound(reason.into())
                    }
                    _ => ApiApplicationError::State(error.to_string()),
                })?,
            ApiStore::Postgres(postgres) => postgres
                .suppress_findings_for_tag(tag_key, &query, suppression.clone())
                .await
                .map_err(|error| {
                    if error.starts_with("unknown tag:") {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?,
        };

        Ok(BulkSuppressFindingsByTagResponse {
            tag_key: tag_key.to_owned(),
            min_severity: request.min_severity,
            package_name: request.package_name,
            targeted: result.targeted,
            suppressed: result.suppressed,
            unchanged: result.unchanged,
            governance_state: "suppressed".to_owned(),
            governance_reason: result.suppression.reason.into(),
            governance_until_unix_ms: None,
        })
    }

    /// Reopen one governed active finding back to the canonical open state.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the finding
    /// is not active, or the durable write fails.
    pub async fn reopen_finding(
        &mut self,
        request: ReopenFindingRequest,
    ) -> Result<ReopenFindingResponse, ApiApplicationError> {
        let finding = build_finding_ref(
            &request.component_key,
            &request.artifact_kind,
            &request.artifact_identity,
            &request.vulnerability_id,
            &request.package_name,
            &request.package_version,
            request.package_purl.as_deref(),
        )?;

        let result: ReopenFindingResult = match &mut self.backend {
            ApiStore::Local(local) => {
                local
                    .state
                    .reopen_finding(&finding)
                    .map_err(|error| match error {
                        venom_domain::DurableStateError::MissingFinding(_) => {
                            ApiApplicationError::NotFound(error.to_string())
                        }
                        _ => ApiApplicationError::State(error.to_string()),
                    })?
            }
            ApiStore::Postgres(postgres) => {
                postgres.reopen_finding(finding).await.map_err(|error| {
                    if error == "cannot reopen an inactive finding" {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?
            }
        };

        Ok(ReopenFindingResponse {
            change: result.change.as_str().to_owned(),
            governance_state: "open".to_owned(),
            governance_reason: None,
            governance_until_unix_ms: None,
        })
    }

    /// Suppress one currently active finding.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the finding
    /// is not active, or the durable write fails.
    pub async fn suppress_finding(
        &mut self,
        request: SuppressFindingRequest,
    ) -> Result<SuppressFindingResponse, ApiApplicationError> {
        let finding = build_finding_ref(
            &request.component_key,
            &request.artifact_kind,
            &request.artifact_identity,
            &request.vulnerability_id,
            &request.package_name,
            &request.package_version,
            request.package_purl.as_deref(),
        )?;
        if request.reason.trim().is_empty() {
            return Err(ApiApplicationError::InvalidRequest(
                "suppression reason must not be empty".to_owned(),
            ));
        }
        let suppression = Suppression::new(request.reason.clone());

        let result: SuppressFindingResult = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .suppress_finding(finding, suppression.clone())
                .map_err(|error| match error {
                    venom_domain::DurableStateError::MissingFinding(_) => {
                        ApiApplicationError::NotFound(error.to_string())
                    }
                    _ => ApiApplicationError::State(error.to_string()),
                })?,
            ApiStore::Postgres(postgres) => postgres
                .suppress_finding(finding, suppression.clone())
                .await
                .map_err(|error| {
                    if error == "cannot suppress an inactive finding" {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?,
        };

        Ok(SuppressFindingResponse {
            change: result.change.as_str().to_owned(),
            governance_state: "suppressed".to_owned(),
            governance_reason: result.suppression.reason.into(),
            governance_until_unix_ms: None,
        })
    }

    /// Reopen one filtered governed cohort of findings inside one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiApplicationError`] when the request is invalid, the collection
    /// is unknown, or the durable write fails.
    pub async fn reopen_findings_for_collection(
        &mut self,
        collection_key: &str,
        request: BulkReopenFindingsRequest,
    ) -> Result<BulkReopenFindingsResponse, ApiApplicationError> {
        let query = build_bulk_collection_reopen_query(
            request.governance_state.as_deref(),
            request.min_severity.as_deref(),
            request.package_name.as_deref(),
        )?;

        let result: BulkReopenFindingResult = match &mut self.backend {
            ApiStore::Local(local) => local
                .state
                .reopen_findings_for_collection(collection_key, &query)
                .map_err(|error| match error {
                    venom_domain::DurableStateError::MissingCollection(_) => {
                        ApiApplicationError::NotFound(error.to_string())
                    }
                    _ => ApiApplicationError::State(error.to_string()),
                })?,
            ApiStore::Postgres(postgres) => postgres
                .reopen_findings_for_collection(collection_key, &query)
                .await
                .map_err(|error| {
                    if error.starts_with("unknown collection:") {
                        ApiApplicationError::NotFound(error)
                    } else {
                        ApiApplicationError::State(error)
                    }
                })?,
        };

        Ok(BulkReopenFindingsResponse {
            collection_key: collection_key.to_owned(),
            governance_state: request.governance_state,
            min_severity: request.min_severity,
            package_name: request.package_name,
            targeted: result.targeted,
            reopened: result.reopened,
            unchanged: result.unchanged,
            result_governance_state: "open".to_owned(),
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
        let command_ids: Vec<String> = match &mut self.backend {
            ApiStore::Local(local) => {
                let batch = ScanPlanner::new(local.state.ingestion().inventory())
                    .plan_collection(collection_key, freshness)
                    .map_err(|error| {
                        ApiApplicationError::InvalidRequest(error.as_str().to_owned())
                    })?;
                local
                    .runtime
                    .enqueue_batch(batch.requests)
                    .map_err(|error| ApiApplicationError::State(error.to_string()))?
                    .into_iter()
                    .map(Into::into)
                    .collect()
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
                let due_scans = CollectionScanScheduler::new(local.state.ingestion().inventory())
                    .collect_due(now_unix_ms, max_collections);

                let processed_collections = due_scans.len();
                let mut enqueued_commands = 0_usize;
                let mut last_collection_key = None;
                for due_scan in due_scans {
                    let command_ids = local
                        .runtime
                        .enqueue_collection_batch(
                            due_scan.collection_key.as_ref(),
                            due_scan.due_at_unix_ms,
                            due_scan.requests,
                        )
                        .map_err(|error| ApiApplicationError::State(error.to_string()))?;
                    enqueued_commands += command_ids.len();
                    last_collection_key = Some(due_scan.collection_key.to_string());
                    if let Err(error) = local.state.record_collection_scan_materialization(
                        due_scan.collection_key.as_ref(),
                        due_scan.next_due_at_unix_ms,
                        now_unix_ms,
                        u32::try_from(command_ids.len()).map_err(|_| {
                            ApiApplicationError::State(
                                "collection scheduler command count overflow".to_owned(),
                            )
                        })?,
                    ) {
                        let pending_due_remaining = local
                            .state
                            .ingestion()
                            .inventory()
                            .due_collection_keys(now_unix_ms, usize::MAX)
                            .len();
                        return Ok(DrainCollectionScanWorkerResponse {
                            outcome: "partial".to_owned(),
                            processed_collections,
                            enqueued_commands,
                            pending_due_remaining,
                            last_collection_key,
                            partial_progress: true,
                            last_error: Some(error.to_string()),
                        });
                    }
                }

                let pending_due_remaining = local
                    .state
                    .ingestion()
                    .inventory()
                    .due_collection_keys(now_unix_ms, usize::MAX)
                    .len();
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
                    partial_progress: false,
                    last_error: None,
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
pub struct ContextProfileRegistrationRequest {
    pub profile_key: String,
    pub name: String,
    pub internet_exposed: Option<bool>,
    pub production: Option<bool>,
    pub mission_critical: Option<bool>,
    pub vpn_restricted: Option<bool>,
    pub non_privileged_user: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RegisterContextProfileResponse {
    pub change: String,
    pub managed_context_profiles: usize,
}

#[derive(Debug, Serialize)]
pub struct ContextProfileItem {
    pub profile_key: String,
    pub name: String,
    pub internet_exposed: Option<bool>,
    pub production: Option<bool>,
    pub mission_critical: Option<bool>,
    pub vpn_restricted: Option<bool>,
    pub non_privileged_user: Option<bool>,
}

impl From<ManagedContextProfile> for ContextProfileItem {
    fn from(value: ManagedContextProfile) -> Self {
        Self {
            profile_key: value.profile_key.into(),
            name: value.name.into(),
            internet_exposed: value.internet_exposed,
            production: value.production,
            mission_critical: value.mission_critical,
            vpn_restricted: value.vpn_restricted,
            non_privileged_user: value.non_privileged_user,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListContextProfilesResponse {
    pub managed_context_profiles: usize,
    pub profiles: Vec<ContextProfileItem>,
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
pub struct ReleaseDashboardResponse {
    pub summary: ReleaseDashboardSummaryItem,
    pub collections: Vec<ReleaseDashboardCollectionItem>,
}

impl ReleaseDashboardResponse {
    fn from_dashboard(dashboard: ReleaseDashboard) -> Self {
        Self {
            summary: ReleaseDashboardSummaryItem::from(dashboard.summary),
            collections: dashboard
                .collections
                .into_iter()
                .map(ReleaseDashboardCollectionItem::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CollectionSummary {
    pub collection_key: String,
    pub name: String,
    pub members: usize,
    pub source: Option<CollectionSourceSummaryItem>,
    pub scan_schedule: Option<CollectionScanScheduleItem>,
    pub due_now: bool,
    pub health: CollectionHealthItem,
}

#[derive(Debug, Serialize)]
pub struct CollectionDetailResponse {
    pub collection_key: String,
    pub name: String,
    pub context_profile_key: Option<String>,
    pub source: Option<CollectionSourceItem>,
    pub scan_schedule: Option<CollectionScanScheduleItem>,
    pub health: CollectionHealthItem,
    pub members: Vec<CollectionMemberItem>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureCollectionSourceRequest {
    pub kind: String,
    pub mode: String,
    pub component_keys: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigureCollectionSourceResponse {
    pub change: String,
    pub source: Option<CollectionSourceItem>,
}

#[derive(Debug, Serialize)]
pub struct MaterializeCollectionSourceResponse {
    pub change: String,
    pub members: usize,
    pub added: usize,
    pub removed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionSourceSummaryItem {
    pub kind: &'static str,
    pub mode: &'static str,
    pub component_count: usize,
}

impl From<CollectionSourceSummary> for CollectionSourceSummaryItem {
    fn from(value: CollectionSourceSummary) -> Self {
        Self {
            kind: value.kind.as_str(),
            mode: value.mode.as_str(),
            component_count: value.component_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionSourceItem {
    pub kind: &'static str,
    pub mode: &'static str,
    pub component_keys: Vec<String>,
}

impl From<CollectionSource> for CollectionSourceItem {
    fn from(value: CollectionSource) -> Self {
        Self {
            kind: value.kind().as_str(),
            mode: value.mode().as_str(),
            component_keys: value
                .component_keys()
                .iter()
                .map(ToString::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ReleaseDashboardSummaryItem {
    pub managed_collections: usize,
    pub scheduled_collections: usize,
    pub due_now_collections: usize,
    pub total_active_findings: usize,
    pub open_findings: usize,
    pub risk_accepted_findings: usize,
    pub suppressed_findings: usize,
    pub critical_risk_findings: usize,
    pub high_risk_findings: usize,
}

impl From<venom_domain::ReleaseDashboardSummary> for ReleaseDashboardSummaryItem {
    fn from(value: venom_domain::ReleaseDashboardSummary) -> Self {
        Self {
            managed_collections: value.managed_collections,
            scheduled_collections: value.scheduled_collections,
            due_now_collections: value.due_now_collections,
            total_active_findings: value.total_active_findings,
            open_findings: value.open_findings,
            risk_accepted_findings: value.risk_accepted_findings,
            suppressed_findings: value.suppressed_findings,
            critical_risk_findings: value.critical_risk_findings,
            high_risk_findings: value.high_risk_findings,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ReleaseDashboardCollectionItem {
    pub collection_key: String,
    pub name: String,
    pub members: usize,
    pub due_now: bool,
    pub scan_schedule: Option<CollectionScanScheduleItem>,
    pub health: CollectionHealthItem,
}

impl From<venom_domain::ReleaseDashboardCollection> for ReleaseDashboardCollectionItem {
    fn from(value: venom_domain::ReleaseDashboardCollection) -> Self {
        Self {
            collection_key: value.collection_key.into(),
            name: value.name.into(),
            members: value.members,
            due_now: value.due_now,
            scan_schedule: value.scan_schedule.map(CollectionScanScheduleItem::from),
            health: CollectionHealthItem::from(value.health),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CollectionHealthItem {
    pub total: usize,
    pub open: usize,
    pub risk_accepted: usize,
    pub suppressed: usize,
    pub critical_risk: usize,
    pub high_risk: usize,
}

impl From<CollectionHealthSummary> for CollectionHealthItem {
    fn from(value: CollectionHealthSummary) -> Self {
        Self {
            total: value.total,
            open: value.open,
            risk_accepted: value.risk_accepted,
            suppressed: value.suppressed,
            critical_risk: value.critical_risk,
            high_risk: value.high_risk,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CollectionMemberItem {
    pub key: String,
    pub context_profile_key: Option<String>,
    pub component_context_profile: Option<ContextProfileRefItem>,
    pub collection_context_profile: Option<ContextProfileRefItem>,
    pub tag_context_profiles: Vec<ContextProfileRefItem>,
    pub tag_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextProfileRefItem {
    pub profile_key: String,
    pub name: String,
}

impl From<venom_domain::ContextProfileRef> for ContextProfileRefItem {
    fn from(value: venom_domain::ContextProfileRef) -> Self {
        Self {
            profile_key: value.profile_key.into(),
            name: value.name.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ComponentTagItem {
    pub tag_key: String,
    pub name: String,
    pub component_keys: Vec<String>,
    pub context_profile_key: Option<String>,
}

impl From<ManagedComponentTag> for ComponentTagItem {
    fn from(value: ManagedComponentTag) -> Self {
        Self {
            tag_key: value.tag_key.into(),
            name: value.name.into(),
            component_keys: value.component_keys.into_iter().map(Into::into).collect(),
            context_profile_key: value.context_profile_key.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListComponentTagsResponse {
    pub managed_component_tags: usize,
    pub tags: Vec<ComponentTagItem>,
}

#[derive(Debug, Deserialize)]
pub struct ComponentTagRegistrationRequest {
    pub tag_key: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterComponentTagResponse {
    pub change: String,
    pub managed_component_tags: usize,
}

#[derive(Debug, Deserialize)]
pub struct ComponentTagMembershipRequest {
    pub component_key: String,
}

#[derive(Debug, Serialize)]
pub struct ComponentTagConflictItem {
    pub component_key: String,
    pub field: &'static str,
    pub existing_profile_key: String,
    pub conflicting_profile_key: String,
}

impl From<venom_domain::TagContextConflict> for ComponentTagConflictItem {
    fn from(value: venom_domain::TagContextConflict) -> Self {
        Self {
            component_key: value.component_key.into(),
            field: value.field.as_str(),
            existing_profile_key: value.existing_profile_key.into(),
            conflicting_profile_key: value.conflicting_profile_key.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ComponentTagMembershipResponse {
    pub change: String,
    pub members: usize,
    pub conflict: Option<ComponentTagConflictItem>,
}

#[derive(Debug, Deserialize)]
pub struct AssignTagContextProfileRequest {
    pub profile_key: String,
}

#[derive(Debug, Serialize)]
pub struct AssignTagContextProfileResponse {
    pub change: String,
    pub profile_key: Option<String>,
    pub conflict: Option<ComponentTagConflictItem>,
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
pub struct AssignContextProfileRequest {
    pub profile_key: String,
}

#[derive(Debug, Deserialize)]
pub struct AssignCollectionContextProfileRequest {
    pub profile_key: String,
}

#[derive(Debug, Serialize)]
pub struct AssignContextProfileResponse {
    pub change: String,
    pub profile_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssignCollectionContextProfileResponse {
    pub change: String,
    pub profile_key: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct AcceptRiskRequest {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub package_purl: Option<String>,
    pub reason: String,
    pub until_unix_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AcceptRiskResponse {
    pub change: String,
    pub governance_state: String,
    pub governance_reason: String,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct BulkAcceptRiskRequest {
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub reason: String,
    pub until_unix_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BulkAcceptRiskResponse {
    pub collection_key: String,
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub targeted: usize,
    pub accepted: usize,
    pub unchanged: usize,
    pub governance_state: String,
    pub governance_reason: String,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BulkAcceptRiskByTagResponse {
    pub tag_key: String,
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub targeted: usize,
    pub accepted: usize,
    pub unchanged: usize,
    pub governance_state: String,
    pub governance_reason: String,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct BulkSuppressFindingsRequest {
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct BulkSuppressFindingsResponse {
    pub collection_key: String,
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub targeted: usize,
    pub suppressed: usize,
    pub unchanged: usize,
    pub governance_state: String,
    pub governance_reason: String,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BulkSuppressFindingsByTagResponse {
    pub tag_key: String,
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub targeted: usize,
    pub suppressed: usize,
    pub unchanged: usize,
    pub governance_state: String,
    pub governance_reason: String,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SuppressFindingRequest {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub package_purl: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct SuppressFindingResponse {
    pub change: String,
    pub governance_state: String,
    pub governance_reason: String,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ReopenFindingRequest {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub package_purl: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReopenFindingResponse {
    pub change: String,
    pub governance_state: String,
    pub governance_reason: Option<String>,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct BulkReopenFindingsRequest {
    pub governance_state: Option<String>,
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkReopenFindingsResponse {
    pub collection_key: String,
    pub governance_state: Option<String>,
    pub min_severity: Option<String>,
    pub package_name: Option<String>,
    pub targeted: usize,
    pub reopened: usize,
    pub unchanged: usize,
    pub result_governance_state: String,
}

#[derive(Debug, Default)]
pub struct ListSystemEventsRequest {
    pub category: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListSystemEventsResponse {
    pub category: Option<String>,
    pub total: usize,
    pub returned: usize,
    pub limit: usize,
    pub events: Vec<SystemEventItem>,
}

impl ListSystemEventsResponse {
    fn from_page(page: SystemEventsPage) -> Self {
        Self {
            category: None,
            total: page.total,
            returned: page.returned,
            limit: page.limit,
            events: page
                .events
                .into_iter()
                .map(|event| SystemEventItem::from(event.as_ref()))
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SystemEventItem {
    pub event_id: String,
    pub occurred_at_unix_ms: u64,
    pub category: String,
    pub kind: String,
    pub collection_key: Option<String>,
    pub component_key: Option<String>,
    pub command_id: Option<String>,
    pub integration_event_id: Option<String>,
    pub finding_count: Option<u32>,
    pub retryable: Option<bool>,
    pub detail: Option<String>,
}

impl From<&SystemEvent> for SystemEventItem {
    fn from(value: &SystemEvent) -> Self {
        let category = value.category().as_str().to_owned();
        let kind = value.kind.as_str().to_owned();
        Self {
            event_id: value.event_id.to_string(),
            occurred_at_unix_ms: value.occurred_at_unix_ms,
            category,
            kind,
            collection_key: value.collection_key.as_deref().map(str::to_owned),
            component_key: value.component_key.as_deref().map(str::to_owned),
            command_id: value.command_id.as_deref().map(str::to_owned),
            integration_event_id: value.integration_event_id.as_deref().map(str::to_owned),
            finding_count: value.finding_count,
            retryable: value.retryable,
            detail: value.detail.as_deref().map(str::to_owned),
        }
    }
}

#[derive(Debug)]
pub struct ActiveFindingsRequest {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub min_severity: Option<String>,
    pub governance_state: Option<String>,
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
    pub governance_state: Option<String>,
    pub package_name: Option<String>,
    pub total_active_findings: usize,
    pub returned: usize,
    pub offset: usize,
    pub limit: usize,
    pub active_findings: Vec<ActiveFindingItem>,
}

#[derive(Debug, Serialize)]
pub struct ActiveFindingItem {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub package_purl: Option<String>,
    pub severity: String,
    pub contextual_risk: String,
    pub contextual_posture: String,
    pub contextual_rule: String,
    pub contextual_factors: Vec<String>,
    pub contextual_factor_provenance: Vec<ContextualFactorProvenanceItem>,
    pub context_profile_key: Option<String>,
    pub context_profile_name: Option<String>,
    pub component_context_profile: Option<ContextProfileRefItem>,
    pub collection_context_profile: Option<ContextProfileRefItem>,
    pub tag_context_profiles: Vec<ContextProfileRefItem>,
    pub governance_state: String,
    pub governance_reason: Option<String>,
    pub governance_until_unix_ms: Option<u64>,
}

impl ActiveFindingItem {
    fn from_projection(value: ContextualActiveFindingProjection) -> Self {
        Self {
            component_key: value.finding.component_key.into(),
            artifact_kind: artifact_kind_name(value.finding.artifact.kind).to_owned(),
            artifact_identity: value.finding.artifact.identity.into(),
            vulnerability_id: value.finding.vulnerability_id.into(),
            package_name: value.finding.package.name.into(),
            package_version: value.finding.package.version.into(),
            package_purl: value.finding.package.purl.map(Into::into),
            severity: severity_name(value.severity).to_owned(),
            contextual_risk: value.contextual_risk.as_str().to_owned(),
            contextual_posture: value.contextual_posture.into(),
            contextual_rule: value.contextual_rule.into(),
            contextual_factors: value
                .contextual_factors
                .into_iter()
                .map(Into::into)
                .collect(),
            contextual_factor_provenance: value
                .contextual_factor_provenance
                .into_iter()
                .map(ContextualFactorProvenanceItem::from)
                .collect(),
            context_profile_key: value.context_profile_key.map(Into::into),
            context_profile_name: value.context_profile_name.map(Into::into),
            component_context_profile: value
                .component_context_profile
                .map(ContextProfileRefItem::from),
            collection_context_profile: value
                .collection_context_profile
                .map(ContextProfileRefItem::from),
            tag_context_profiles: value
                .tag_context_profiles
                .into_iter()
                .map(ContextProfileRefItem::from)
                .collect(),
            governance_state: value.governance_state.as_str().to_owned(),
            governance_reason: value.governance_reason.map(Into::into),
            governance_until_unix_ms: value.governance_until_unix_ms,
        }
    }
}

#[derive(Debug)]
pub struct CollectionActiveFindingsRequest {
    pub min_severity: Option<String>,
    pub governance_state: Option<String>,
    pub package_name: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct CollectionActiveFindingsResponse {
    pub collection_key: String,
    pub min_severity: Option<String>,
    pub governance_state: Option<String>,
    pub package_name: Option<String>,
    pub health: CollectionHealthItem,
    pub bulk_governance: BulkGovernanceCohortItem,
    pub total_active_findings: usize,
    pub returned: usize,
    pub offset: usize,
    pub limit: usize,
    pub active_findings: Vec<CollectionActiveFindingItem>,
}

#[derive(Debug, Serialize)]
pub struct BulkGovernanceCohortItem {
    pub targeted: usize,
    pub critical_risk: usize,
    pub high_risk: usize,
}

impl From<venom_domain::BulkGovernanceCohortSummary> for BulkGovernanceCohortItem {
    fn from(value: venom_domain::BulkGovernanceCohortSummary) -> Self {
        Self {
            targeted: value.targeted,
            critical_risk: value.critical_risk,
            high_risk: value.high_risk,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CollectionActiveFindingItem {
    pub component_key: String,
    pub artifact_kind: String,
    pub artifact_identity: String,
    pub vulnerability_id: String,
    pub package_name: String,
    pub package_version: String,
    pub package_purl: Option<String>,
    pub severity: String,
    pub contextual_risk: String,
    pub contextual_posture: String,
    pub contextual_rule: String,
    pub contextual_factors: Vec<String>,
    pub contextual_factor_provenance: Vec<ContextualFactorProvenanceItem>,
    pub context_profile_key: Option<String>,
    pub context_profile_name: Option<String>,
    pub component_context_profile: Option<ContextProfileRefItem>,
    pub collection_context_profile: Option<ContextProfileRefItem>,
    pub tag_context_profiles: Vec<ContextProfileRefItem>,
    pub governance_state: String,
    pub governance_reason: Option<String>,
    pub governance_until_unix_ms: Option<u64>,
}

impl CollectionActiveFindingItem {
    fn from_projection(value: ContextualActiveFindingProjection) -> Self {
        Self {
            component_key: value.finding.component_key.into(),
            artifact_kind: artifact_kind_name(value.finding.artifact.kind).to_owned(),
            artifact_identity: value.finding.artifact.identity.into(),
            vulnerability_id: value.finding.vulnerability_id.into(),
            package_name: value.finding.package.name.into(),
            package_version: value.finding.package.version.into(),
            package_purl: value.finding.package.purl.map(Into::into),
            severity: severity_name(value.severity).to_owned(),
            contextual_risk: value.contextual_risk.as_str().to_owned(),
            contextual_posture: value.contextual_posture.into(),
            contextual_rule: value.contextual_rule.into(),
            contextual_factors: value
                .contextual_factors
                .into_iter()
                .map(Into::into)
                .collect(),
            contextual_factor_provenance: value
                .contextual_factor_provenance
                .into_iter()
                .map(ContextualFactorProvenanceItem::from)
                .collect(),
            context_profile_key: value.context_profile_key.map(Into::into),
            context_profile_name: value.context_profile_name.map(Into::into),
            component_context_profile: value
                .component_context_profile
                .map(ContextProfileRefItem::from),
            collection_context_profile: value
                .collection_context_profile
                .map(ContextProfileRefItem::from),
            tag_context_profiles: value
                .tag_context_profiles
                .into_iter()
                .map(ContextProfileRefItem::from)
                .collect(),
            governance_state: value.governance_state.as_str().to_owned(),
            governance_reason: value.governance_reason.map(Into::into),
            governance_until_unix_ms: value.governance_until_unix_ms,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ContextualFactorProvenanceItem {
    pub factor: String,
    pub source: String,
    pub identity: String,
}

impl From<venom_domain::findings::ContextualFactorProvenance> for ContextualFactorProvenanceItem {
    fn from(value: venom_domain::findings::ContextualFactorProvenance) -> Self {
        Self {
            factor: value.factor.into(),
            source: value.source.into(),
            identity: value.identity.into(),
        }
    }
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

#[derive(Debug, Clone, Deserialize)]
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
    pub partial_progress: bool,
    pub last_error: Option<String>,
}

impl From<DrainDueCollectionScansResult> for DrainCollectionScanWorkerResponse {
    fn from(value: DrainDueCollectionScansResult) -> Self {
        Self {
            outcome: value.outcome.into(),
            processed_collections: value.processed_collections,
            enqueued_commands: value.enqueued_commands,
            pending_due_remaining: value.pending_due_remaining,
            last_collection_key: value.last_collection_key.map(Into::into),
            partial_progress: value.partial_progress,
            last_error: value.last_error.map(Into::into),
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

#[derive(Debug, Clone, Deserialize)]
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

const fn artifact_kind_name(value: ArtifactKind) -> &'static str {
    match value {
        ArtifactKind::ContainerImage => "container-image",
        ArtifactKind::SbomDocument => "sbom-document",
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

fn parse_collection_source(
    request: ConfigureCollectionSourceRequest,
) -> Result<CollectionSource, ApiApplicationError> {
    let kind = match request.kind.as_str() {
        "component-list" => CollectionSourceKind::ComponentList,
        value => {
            return Err(ApiApplicationError::InvalidRequest(format!(
                "unsupported collection source kind: {value}"
            )));
        }
    };
    let mode = match request.mode.as_str() {
        "replace" => CollectionSourceMode::Replace,
        "reconcile" => CollectionSourceMode::Reconcile,
        value => {
            return Err(ApiApplicationError::InvalidRequest(format!(
                "unsupported collection source mode: {value}"
            )));
        }
    };
    let component_keys = request
        .component_keys
        .into_iter()
        .map(String::into_boxed_str)
        .collect::<Vec<_>>();

    if component_keys.is_empty() {
        return Err(ApiApplicationError::InvalidRequest(
            "collection source component_keys must not be empty".to_owned(),
        ));
    }

    match kind {
        CollectionSourceKind::ComponentList => Ok(CollectionSource::ComponentList(
            ComponentListCollectionSource::new(mode, component_keys),
        )),
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
    if let Some(governance_state) = request.governance_state.as_deref() {
        query = query.with_governance_state(parse_governance_state(governance_state)?);
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

fn build_finding_ref(
    component_key: &str,
    artifact_kind: &str,
    artifact_identity: &str,
    vulnerability_id: &str,
    package_name: &str,
    package_version: &str,
    package_purl: Option<&str>,
) -> Result<FindingRef, ApiApplicationError> {
    let mut package = PackageCoordinate::new(package_name.to_owned(), package_version.to_owned());
    if let Some(package_purl) = package_purl.filter(|value| !value.is_empty()) {
        package = package.with_purl(package_purl.to_owned());
    }

    Ok(FindingRef::new(
        component_key.to_owned(),
        ArtifactRef::new(
            parse_artifact_kind(artifact_kind)?,
            artifact_identity.to_owned(),
        ),
        vulnerability_id.to_owned(),
        package,
    ))
}

fn build_scoped_active_findings_query(
    request: &CollectionActiveFindingsRequest,
) -> Result<ScopedActiveFindingsQuery, ApiApplicationError> {
    let mut query = ScopedActiveFindingsQuery::new();
    if let Some(min_severity) = request.min_severity.as_deref() {
        query = query.with_min_severity(parse_severity(min_severity)?);
    }
    if let Some(governance_state) = request.governance_state.as_deref() {
        query = query.with_governance_state(parse_governance_state(governance_state)?);
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

fn build_bulk_collection_governance_query(
    min_severity: Option<&str>,
    package_name: Option<&str>,
) -> Result<BulkGovernanceQuery, ApiApplicationError> {
    let mut query = BulkGovernanceQuery::new(FindingGovernanceState::Open);
    if let Some(min_severity) = min_severity {
        query = query.with_min_severity(parse_severity(min_severity)?);
    }
    if let Some(package_name) = package_name {
        query = query.with_package_name(package_name);
    }
    Ok(query)
}

fn build_bulk_collection_reopen_query(
    governance_state: Option<&str>,
    min_severity: Option<&str>,
    package_name: Option<&str>,
) -> Result<BulkGovernanceQuery, ApiApplicationError> {
    let governance_state = governance_state.ok_or_else(|| {
        ApiApplicationError::InvalidRequest(
            "bulk reopen requires a governed state filter".to_owned(),
        )
    })?;
    let governance_state = parse_governance_state(governance_state)?;
    if governance_state == FindingGovernanceState::Open {
        return Err(ApiApplicationError::InvalidRequest(
            "bulk reopen does not support the open governance state".to_owned(),
        ));
    }

    let mut query = BulkGovernanceQuery::new(governance_state);
    if let Some(min_severity) = min_severity {
        query = query.with_min_severity(parse_severity(min_severity)?);
    }
    if let Some(package_name) = package_name {
        query = query.with_package_name(package_name);
    }
    Ok(query)
}

fn build_system_events_query(
    request: &ListSystemEventsRequest,
) -> Result<SystemEventsQuery, ApiApplicationError> {
    let mut query = SystemEventsQuery::new();
    if let Some(category) = request.category.as_deref() {
        query = query.with_category(parse_system_event_category(category)?);
    }
    if let Some(limit) = request.limit {
        query = query.with_limit(limit);
    }
    Ok(query)
}

fn parse_system_event_category(value: &str) -> Result<SystemEventCategory, ApiApplicationError> {
    match value {
        "scheduler" => Ok(SystemEventCategory::Scheduler),
        "command" => Ok(SystemEventCategory::Command),
        "governance" => Ok(SystemEventCategory::Governance),
        "publication" => Ok(SystemEventCategory::Publication),
        _ => Err(ApiApplicationError::InvalidRequest(format!(
            "unsupported system event category: {value}"
        ))),
    }
}

fn parse_governance_state(value: &str) -> Result<FindingGovernanceState, ApiApplicationError> {
    match value {
        "open" => Ok(FindingGovernanceState::Open),
        "risk-accepted" => Ok(FindingGovernanceState::RiskAccepted),
        "suppressed" => Ok(FindingGovernanceState::Suppressed),
        _ => Err(ApiApplicationError::InvalidRequest(format!(
            "unsupported governance state: {value}"
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

#[cfg(test)]
mod tests {
    use super::{ApiApplication, ComponentRegistrationRequest, LocalStore, RequestScanCommand};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use venom_domain::durable_state::DurableState;
    use venom_domain::scanning::ScanCommandQueue;

    fn temp_path(name: &str, suffix: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "venom-api-service-{name}-{suffix}-{nanos}-{counter}.jsonl"
        ))
    }

    #[tokio::test]
    async fn local_stale_lane_refresh_uses_tail_sync_for_inventory_changes() {
        let state_path = temp_path("tail-sync", "state");
        let runtime_path = temp_path("tail-sync", "runtime");
        let mut writer =
            ApiApplication::open_local(&state_path, &runtime_path).expect("writer should open");
        let mut follower =
            ApiApplication::open_local(&state_path, &runtime_path).expect("follower should open");

        writer
            .register_component(ComponentRegistrationRequest {
                component_key: "component:payments-api".to_owned(),
                name: "Payments API".to_owned(),
            })
            .await
            .expect("component registration should persist");

        follower
            .refresh_local_from_disk()
            .expect("follower should tail-sync local state");

        assert!(
            follower
                .inventory_snapshot_arc()
                .is_managed("component:payments-api")
        );
    }

    #[tokio::test]
    async fn local_stale_lane_refresh_uses_tail_sync_for_runtime_changes() {
        let state_path = temp_path("tail-sync-runtime", "state");
        let runtime_path = temp_path("tail-sync-runtime", "runtime");
        let mut writer =
            ApiApplication::open_local(&state_path, &runtime_path).expect("writer should open");
        let mut follower =
            ApiApplication::open_local(&state_path, &runtime_path).expect("follower should open");

        writer
            .register_component(ComponentRegistrationRequest {
                component_key: "component:payments-api".to_owned(),
                name: "Payments API".to_owned(),
            })
            .await
            .expect("component registration should persist");
        writer
            .bind_artifact(
                "component:payments-api",
                super::BindArtifactRequest {
                    artifact_kind: "container-image".to_owned(),
                    artifact_identity: "registry.example/payments@sha256:111".to_owned(),
                },
            )
            .await
            .expect("artifact binding should persist");
        writer
            .request_scan(RequestScanCommand {
                component_key: "component:payments-api".to_owned(),
                artifact_kind: "container-image".to_owned(),
                artifact_identity: "registry.example/payments@sha256:111".to_owned(),
                freshness: "deterministic".to_owned(),
            })
            .await
            .expect("scan request should persist");

        follower
            .refresh_local_from_disk()
            .expect("follower should tail-sync local runtime");

        let statuses = follower.command_statuses_snapshot_arc();
        assert_eq!(statuses.len(), 1);
    }

    #[test]
    fn local_merged_system_event_snapshot_reuses_cached_peer_window() {
        let state_path = temp_path("merged-events-cache", "state");
        let runtime_path = temp_path("merged-events-cache", "runtime");
        let state = DurableState::open(state_path.clone()).expect("state should open");
        let runtime = ScanCommandQueue::open(runtime_path.clone()).expect("runtime should open");
        let mut local = LocalStore {
            state_path,
            runtime_path,
            state,
            runtime,
            merged_system_event_snapshot_cache: std::sync::Mutex::new(None),
        };

        let _first = local.system_event_index_snapshot_arc();
        let first_cache = local
            .merged_system_event_snapshot_cache
            .lock()
            .expect("merged cache should not be poisoned")
            .as_ref()
            .expect("merged cache should exist after first read")
            .runtime
            .clone();

        local
            .state
            .register_component(venom_domain::ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("state mutation should persist");

        let _second = local.system_event_index_snapshot_arc();
        let runtime_cache = {
            let cache = local
                .merged_system_event_snapshot_cache
                .lock()
                .expect("merged cache should not be poisoned");
            let snapshot = cache
                .as_ref()
                .expect("merged cache should be refreshed after state change");
            (
                snapshot.runtime.clone(),
                snapshot.runtime_windows.recent_events.len(),
                snapshot.runtime.recent_windows().recent_events.len(),
            )
        };
        assert!(std::sync::Arc::ptr_eq(&first_cache, &runtime_cache.0));
        assert_eq!(runtime_cache.1, runtime_cache.2);
    }
}
