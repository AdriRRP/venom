use crate::app::service::{
    self, AcceptRiskRequest, AcceptRiskResponse, ActiveFindingsResponse, ApiApplication,
    ApiReadSnapshot, AssignCollectionContextProfileRequest, AssignCollectionContextProfileResponse,
    AssignContextProfileRequest, AssignContextProfileResponse, AssignTagContextProfileRequest,
    AssignTagContextProfileResponse, BindArtifactRequest, BindArtifactResponse,
    BulkAcceptRiskByTagResponse, BulkAcceptRiskRequest, BulkAcceptRiskResponse,
    BulkReopenFindingsRequest, BulkReopenFindingsResponse, BulkSuppressFindingsByTagResponse,
    BulkSuppressFindingsRequest, BulkSuppressFindingsResponse, CollectionActiveFindingsResponse,
    CollectionDetailResponse, CollectionMembershipRequest, CollectionMembershipResponse,
    CollectionRegistrationRequest, ComponentRegistrationRequest, ComponentTagMembershipRequest,
    ComponentTagMembershipResponse, ComponentTagRegistrationRequest,
    ConfigureCollectionScanScheduleRequest, ConfigureCollectionScanScheduleResponse,
    ConfigureCollectionSourceRequest, ConfigureCollectionSourceResponse,
    ConfigureIntegrationRuntimeRequest, ConfigureIntegrationRuntimeResponse,
    ConfigureProviderRequest, ConfigureProviderResponse, ContextProfileRegistrationRequest,
    DrainCollectionScanWorkerCommand, DrainCollectionScanWorkerResponse,
    DrainIntegrationWorkerCommand, DrainIntegrationWorkerResponse, DrainWorkerCommand,
    DrainWorkerResponse, ListCollectionsResponse, ListComponentTagsResponse,
    ListContextProfilesResponse, ListSystemEventsRequest, ListSystemEventsResponse,
    MaterializeCollectionSourceResponse, ProviderScanReportRequest, RecordProviderReportResponse,
    RegisterCollectionResponse, RegisterComponentResponse, RegisterComponentTagResponse,
    RegisterContextProfileResponse, ReleaseDashboardResponse, ReopenFindingRequest,
    ReopenFindingResponse, RequestCollectionScanCommand, RequestCollectionScanResponse,
    RequestScanCommand, RequestScanResponse, RunNextScanCommand, RunNextScanResponse,
    ScanCommandStatusResponse, SuppressFindingRequest, SuppressFindingResponse,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{Mutex, Notify, RwLock, watch};
use venom_domain::operations::system_event_trace::SystemEventQueryIndex;
use venom_domain::scanning::ScanCommandStatus;

type ApiMutationFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

#[derive(Clone)]
pub struct ApiState {
    inner: Arc<ApiStateInner>,
}

struct ApiStateInner {
    services: ApiServiceSet,
    write_consistency_barrier: RwLock<()>,
    local_runtime_mutation_barrier: Mutex<()>,
    local_change_epoch: AtomicU64,
    remote_change_probe: Option<service::PostgresRemoteChangeProbe>,
    remote_read_snapshot_loader: Option<service::PostgresReadSnapshotLoader>,
    remote_refresh: Mutex<()>,
    remote_snapshot_watermark: AtomicU64,
    remote_observation_degraded: AtomicBool,
    read_snapshot_tx: watch::Sender<Arc<ApiReadSnapshot>>,
    read_snapshot_rx: watch::Receiver<Arc<ApiReadSnapshot>>,
}

struct ServiceSlot {
    service: Mutex<Option<ApiApplication>>,
    ready: Notify,
    observed_local_change_epoch: AtomicU64,
}

enum ApiServiceSet {
    Partitioned(Box<PartitionedServiceSlots>),
}

struct PartitionedServiceSlots {
    state: ServiceSlot,
    runtime: ServiceSlot,
    publication: ServiceSlot,
}

#[derive(Debug, Clone, Copy)]
enum ApiMutationLane {
    State,
    Runtime,
    Publication,
}

impl ApiMutationLane {
    const fn requires_state_write_barrier(self) -> bool {
        matches!(self, Self::State)
    }

    const fn requires_state_read_barrier(self) -> bool {
        matches!(self, Self::Runtime)
    }

    const fn requires_local_runtime_mutation_barrier(self) -> bool {
        matches!(self, Self::Runtime | Self::Publication)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiHealthStatus {
    Healthy,
    Degraded,
}

impl ApiHealthStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
        }
    }
}

enum SnapshotRefresh {
    Unchanged,
    Inventory {
        inventory: Arc<venom_domain::ComponentInventory>,
    },
    ReadModel {
        read_model: Arc<venom_domain::FindingReadModel>,
    },
    SystemEvents {
        system_events: Arc<SystemEventQueryIndex>,
    },
    CombinedReadModelAndSystemEvents {
        read_model: Arc<venom_domain::FindingReadModel>,
        system_events: Arc<SystemEventQueryIndex>,
    },
    CombinedCommandStatusesAndSystemEvents {
        command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
        system_events: Arc<SystemEventQueryIndex>,
    },
    CombinedInventoryCommandStatusesAndSystemEvents {
        inventory: Arc<venom_domain::ComponentInventory>,
        command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
        system_events: Arc<SystemEventQueryIndex>,
    },
    CombinedReadModelCommandStatusesAndSystemEvents {
        read_model: Arc<venom_domain::FindingReadModel>,
        command_statuses: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
        system_events: Arc<SystemEventQueryIndex>,
    },
}

impl SnapshotRefresh {
    fn apply(self, current: &Arc<ApiReadSnapshot>) -> Arc<ApiReadSnapshot> {
        match self {
            Self::Unchanged => Arc::clone(current),
            Self::Inventory { inventory } => Arc::new(current.with_inventory_arc(inventory)),
            Self::ReadModel { read_model } => Arc::new(current.with_read_model_arc(read_model)),
            Self::SystemEvents { system_events } => {
                Arc::new(current.with_system_event_index_arc(system_events))
            }
            Self::CombinedReadModelAndSystemEvents {
                read_model,
                system_events,
            } => {
                let next = current.with_read_model_arc(read_model);
                Arc::new(next.with_system_event_index_arc(system_events))
            }
            Self::CombinedCommandStatusesAndSystemEvents {
                command_statuses,
                system_events,
            } => {
                let next = current.with_command_statuses_arc(command_statuses);
                Arc::new(next.with_system_event_index_arc(system_events))
            }
            Self::CombinedInventoryCommandStatusesAndSystemEvents {
                inventory,
                command_statuses,
                system_events,
            } => {
                let next = current.with_inventory_arc(inventory);
                let next = next.with_command_statuses_arc(command_statuses);
                Arc::new(next.with_system_event_index_arc(system_events))
            }
            Self::CombinedReadModelCommandStatusesAndSystemEvents {
                read_model,
                command_statuses,
                system_events,
            } => {
                let next = current.with_read_model_arc(read_model);
                let next = next.with_command_statuses_arc(command_statuses);
                Arc::new(next.with_system_event_index_arc(system_events))
            }
        }
    }
}

impl ApiState {
    /// Open the API state over one local durable state path.
    ///
    /// # Errors
    ///
    /// Returns an error string when the underlying durable state or runtime cannot be opened.
    pub fn open(
        state_path: impl Into<PathBuf>,
        runtime_path: impl Into<PathBuf>,
    ) -> Result<Self, String> {
        let state_path = state_path.into();
        let runtime_path = runtime_path.into();
        let state_service = ApiApplication::open_local(state_path.clone(), runtime_path.clone())
            .map_err(|error| error.to_string())?;
        let runtime_service = ApiApplication::open_local(state_path.clone(), runtime_path.clone())
            .map_err(|error| error.to_string())?;
        let publication_service = ApiApplication::open_local(state_path, runtime_path)
            .map_err(|error| error.to_string())?;
        Ok(Self::new_partitioned(
            state_service,
            runtime_service,
            publication_service,
        ))
    }

    /// Open the API state over a Postgres durable backend.
    ///
    /// # Errors
    ///
    /// Returns an error string when the Postgres durable backend cannot be opened.
    pub async fn open_postgres(database_url: &str, schema: &str) -> Result<Self, String> {
        let state_service = ApiApplication::open_postgres(database_url, schema)
            .await
            .map_err(|error| error.to_string())?;
        let runtime_service = ApiApplication::open_postgres(database_url, schema)
            .await
            .map_err(|error| error.to_string())?;
        let publication_service = ApiApplication::open_postgres(database_url, schema)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self::new_partitioned(
            state_service,
            runtime_service,
            publication_service,
        ))
    }

    fn new_partitioned(
        state_service: ApiApplication,
        runtime_service: ApiApplication,
        publication_service: ApiApplication,
    ) -> Self {
        let remote_snapshot_watermark = state_service
            .observed_remote_change_watermark()
            .unwrap_or(0);
        let remote_change_probe = state_service.remote_change_probe();
        let remote_read_snapshot_loader = state_service.remote_read_snapshot_loader();
        let snapshot = Arc::new(state_service.read_snapshot());
        let (read_snapshot_tx, read_snapshot_rx) = watch::channel(snapshot);
        Self {
            inner: Arc::new(ApiStateInner {
                services: ApiServiceSet::Partitioned(Box::new(PartitionedServiceSlots {
                    state: ServiceSlot {
                        service: Mutex::new(Some(state_service)),
                        ready: Notify::new(),
                        observed_local_change_epoch: AtomicU64::new(0),
                    },
                    runtime: ServiceSlot {
                        service: Mutex::new(Some(runtime_service)),
                        ready: Notify::new(),
                        observed_local_change_epoch: AtomicU64::new(0),
                    },
                    publication: ServiceSlot {
                        service: Mutex::new(Some(publication_service)),
                        ready: Notify::new(),
                        observed_local_change_epoch: AtomicU64::new(0),
                    },
                })),
                write_consistency_barrier: RwLock::new(()),
                local_runtime_mutation_barrier: Mutex::new(()),
                local_change_epoch: AtomicU64::new(0),
                remote_change_probe,
                remote_read_snapshot_loader,
                remote_refresh: Mutex::new(()),
                remote_snapshot_watermark: AtomicU64::new(remote_snapshot_watermark),
                remote_observation_degraded: AtomicBool::new(false),
                read_snapshot_tx,
                read_snapshot_rx,
            }),
        }
    }

    fn read_snapshot(&self) -> Arc<ApiReadSnapshot> {
        self.inner.read_snapshot_rx.borrow().clone()
    }

    fn health_status(&self) -> ApiHealthStatus {
        if self
            .inner
            .remote_observation_degraded
            .load(Ordering::Relaxed)
        {
            ApiHealthStatus::Degraded
        } else {
            ApiHealthStatus::Healthy
        }
    }

    async fn read_snapshot_fresh(&self) -> Result<Arc<ApiReadSnapshot>, ApiError> {
        if let Some(probe) = &self.inner.remote_change_probe {
            let current_change_watermark = probe
                .current_change_watermark()
                .await
                .map_err(ApiError::internal)?;
            if remote_snapshot_is_current(
                current_change_watermark,
                probe.observed_change_watermark(),
                self.inner.remote_snapshot_watermark.load(Ordering::Relaxed),
            ) {
                if current_change_watermark == probe.observed_change_watermark() {
                    self.inner
                        .remote_observation_degraded
                        .store(false, Ordering::Relaxed);
                }
                return Ok(self.read_snapshot());
            }

            let _refresh_guard = self.inner.remote_refresh.lock().await;
            let current_change_watermark = probe
                .current_change_watermark()
                .await
                .map_err(ApiError::internal)?;
            if remote_snapshot_is_current(
                current_change_watermark,
                probe.observed_change_watermark(),
                self.inner.remote_snapshot_watermark.load(Ordering::Relaxed),
            ) {
                if current_change_watermark == probe.observed_change_watermark() {
                    self.inner
                        .remote_observation_degraded
                        .store(false, Ordering::Relaxed);
                }
                return Ok(self.read_snapshot());
            }

            if let Some(loader) = &self.inner.remote_read_snapshot_loader {
                let current_snapshot = self.read_snapshot();
                let loaded = loader
                    .load(
                        self.inner.remote_snapshot_watermark.load(Ordering::Relaxed),
                        current_snapshot.inventory_arc(),
                        current_snapshot.read_model_arc(),
                        current_snapshot.system_event_index_arc(),
                        current_snapshot.command_statuses_arc(),
                    )
                    .await
                    .map_err(ApiError::internal)?;
                probe.observe_change_watermark(loaded.change_watermark);
                self.inner
                    .remote_observation_degraded
                    .store(false, Ordering::Relaxed);
                let published_watermark =
                    self.inner.remote_snapshot_watermark.load(Ordering::Relaxed);
                if !should_publish_remote_snapshot(loaded.change_watermark, published_watermark) {
                    return Ok(self.read_snapshot());
                }
                let next_snapshot = Arc::new(ApiReadSnapshot::new(
                    loaded.inventory,
                    loaded.read_model,
                    loaded.system_event_index,
                    loaded.command_statuses,
                ));
                self.publish_snapshot(Arc::clone(&next_snapshot), Some(loaded.change_watermark));
                return Ok(next_snapshot);
            }
        }
        let mut service = self.take_service(ApiMutationLane::State).await;
        let changed = match service.refresh_from_remote_if_stale().await {
            Ok(changed) => changed,
            Err(error) => {
                self.restore_service(ApiMutationLane::State, service).await;
                return Err(ApiError::from(error));
            }
        };
        let next_snapshot = changed.then(|| Arc::new(service.read_snapshot()));
        let remote_change_watermark = service.observed_remote_change_watermark();
        self.restore_service(ApiMutationLane::State, service).await;
        next_snapshot.map_or_else(
            || Ok(self.read_snapshot()),
            |next_snapshot| {
                self.publish_snapshot(Arc::clone(&next_snapshot), remote_change_watermark);
                Ok(next_snapshot)
            },
        )
    }

    fn publish_snapshot(
        &self,
        snapshot: Arc<ApiReadSnapshot>,
        remote_change_watermark: Option<u64>,
    ) {
        if let Some(change_watermark) = remote_change_watermark {
            self.inner
                .remote_snapshot_watermark
                .store(change_watermark, Ordering::Relaxed);
        }
        self.inner.read_snapshot_tx.send_replace(snapshot);
    }

    fn update_remote_observation_after_mutation(
        &self,
        refresh_from_remote_changed: bool,
        result_is_ok: bool,
        mark_remote_change_failed: bool,
    ) {
        if refresh_from_remote_changed || (result_is_ok && !mark_remote_change_failed) {
            self.inner
                .remote_observation_degraded
                .store(false, Ordering::Relaxed);
        } else if mark_remote_change_failed {
            self.inner
                .remote_observation_degraded
                .store(true, Ordering::Relaxed);
        }
    }

    async fn detached_remote_snapshot_refresh(
        &self,
        should_refresh: bool,
    ) -> Option<Result<(Arc<ApiReadSnapshot>, u64), String>> {
        let loader = self.inner.remote_read_snapshot_loader.as_ref()?;
        if !should_refresh {
            return None;
        }

        let current_snapshot = self.read_snapshot();
        Some(
            loader
                .load(
                    self.inner.remote_snapshot_watermark.load(Ordering::Relaxed),
                    current_snapshot.inventory_arc(),
                    current_snapshot.read_model_arc(),
                    current_snapshot.system_event_index_arc(),
                    current_snapshot.command_statuses_arc(),
                )
                .await
                .map(|loaded| {
                    (
                        Arc::new(ApiReadSnapshot::new(
                            loaded.inventory,
                            loaded.read_model,
                            loaded.system_event_index,
                            loaded.command_statuses,
                        )),
                        loaded.change_watermark,
                    )
                }),
        )
    }

    fn refresh_inventory_snapshot(service: &ApiApplication) -> SnapshotRefresh {
        SnapshotRefresh::Inventory {
            inventory: service.inventory_snapshot_arc(),
        }
    }

    fn refresh_read_model_snapshot(service: &ApiApplication) -> SnapshotRefresh {
        SnapshotRefresh::ReadModel {
            read_model: service.read_model_snapshot_arc(),
        }
    }

    fn refresh_system_events_snapshot(service: &ApiApplication) -> SnapshotRefresh {
        SnapshotRefresh::SystemEvents {
            system_events: service.system_event_index_snapshot_arc(),
        }
    }

    const fn unchanged_snapshot(_service: &ApiApplication) -> SnapshotRefresh {
        SnapshotRefresh::Unchanged
    }

    fn refresh_read_model_and_system_events_snapshot(service: &ApiApplication) -> SnapshotRefresh {
        let snapshot = Self::refresh_read_model_snapshot(service);
        let system_events = service.system_event_index_snapshot_arc();
        match snapshot {
            SnapshotRefresh::ReadModel { read_model } => {
                SnapshotRefresh::CombinedReadModelAndSystemEvents {
                    read_model,
                    system_events,
                }
            }
            _ => unreachable!("read-model refresh must produce a read-model lane"),
        }
    }

    fn refresh_command_status_and_system_events_snapshot(
        service: &ApiApplication,
    ) -> SnapshotRefresh {
        SnapshotRefresh::CombinedCommandStatusesAndSystemEvents {
            command_statuses: service.command_statuses_snapshot_arc(),
            system_events: service.system_event_index_snapshot_arc(),
        }
    }

    fn refresh_inventory_command_status_and_system_events_snapshot(
        service: &ApiApplication,
    ) -> SnapshotRefresh {
        SnapshotRefresh::CombinedInventoryCommandStatusesAndSystemEvents {
            inventory: service.inventory_snapshot_arc(),
            command_statuses: service.command_statuses_snapshot_arc(),
            system_events: service.system_event_index_snapshot_arc(),
        }
    }

    fn refresh_read_model_command_status_and_system_events_snapshot(
        service: &ApiApplication,
    ) -> SnapshotRefresh {
        SnapshotRefresh::CombinedReadModelCommandStatusesAndSystemEvents {
            read_model: service.read_model_snapshot_arc(),
            command_statuses: service.command_statuses_snapshot_arc(),
            system_events: service.system_event_index_snapshot_arc(),
        }
    }

    fn slot_for_lane(&self, lane: ApiMutationLane) -> &ServiceSlot {
        match &self.inner.services {
            ApiServiceSet::Partitioned(slots) => match lane {
                ApiMutationLane::State => &slots.state,
                ApiMutationLane::Runtime => &slots.runtime,
                ApiMutationLane::Publication => &slots.publication,
            },
        }
    }

    async fn take_service(&self, lane: ApiMutationLane) -> ApiApplication {
        let slot = self.slot_for_lane(lane);
        loop {
            let mut guard = slot.service.lock().await;
            if let Some(service) = guard.take() {
                return service;
            }
            drop(guard);
            slot.ready.notified().await;
        }
    }

    async fn restore_service(&self, lane: ApiMutationLane, service: ApiApplication) {
        let slot = self.slot_for_lane(lane);
        let mut guard = slot.service.lock().await;
        *guard = Some(service);
        drop(guard);
        slot.ready.notify_waiters();
    }

    fn refresh_local_service_if_stale(
        &self,
        lane: ApiMutationLane,
        service: &mut ApiApplication,
    ) -> Result<(), ApiError> {
        if self.inner.remote_read_snapshot_loader.is_some() {
            return Ok(());
        }

        let slot = self.slot_for_lane(lane);
        let observed_epoch = slot.observed_local_change_epoch.load(Ordering::Relaxed);
        let current_epoch = self.inner.local_change_epoch.load(Ordering::Relaxed);
        if observed_epoch >= current_epoch {
            return Ok(());
        }

        service.refresh_local_from_disk().map_err(ApiError::from)?;
        slot.observed_local_change_epoch
            .store(current_epoch, Ordering::Relaxed);
        Ok(())
    }

    async fn mutate<T, F, R>(&self, operation: F, refresh: R) -> Result<T, ApiError>
    where
        F: for<'a> FnOnce(&'a mut ApiApplication) -> ApiMutationFuture<'a, T>,
        R: FnOnce(&ApiApplication) -> SnapshotRefresh,
    {
        self.mutate_on_lane(ApiMutationLane::State, operation, refresh)
            .await
    }

    async fn mutate_runtime<T, F, R>(&self, operation: F, refresh: R) -> Result<T, ApiError>
    where
        F: for<'a> FnOnce(&'a mut ApiApplication) -> ApiMutationFuture<'a, T>,
        R: FnOnce(&ApiApplication) -> SnapshotRefresh,
    {
        self.mutate_on_lane(ApiMutationLane::Runtime, operation, refresh)
            .await
    }

    async fn mutate_publication<T, F, R>(&self, operation: F, refresh: R) -> Result<T, ApiError>
    where
        F: for<'a> FnOnce(&'a mut ApiApplication) -> ApiMutationFuture<'a, T>,
        R: FnOnce(&ApiApplication) -> SnapshotRefresh,
    {
        self.mutate_on_lane(ApiMutationLane::Publication, operation, refresh)
            .await
    }

    async fn mutate_on_lane<T, F, R>(
        &self,
        lane: ApiMutationLane,
        operation: F,
        refresh: R,
    ) -> Result<T, ApiError>
    where
        F: for<'a> FnOnce(&'a mut ApiApplication) -> ApiMutationFuture<'a, T>,
        R: FnOnce(&ApiApplication) -> SnapshotRefresh,
    {
        let _write_consistency_guard = if lane.requires_state_write_barrier() {
            Some(self.inner.write_consistency_barrier.write().await)
        } else {
            None
        };
        let _read_consistency_guard = if lane.requires_state_read_barrier() {
            Some(self.inner.write_consistency_barrier.read().await)
        } else {
            None
        };
        let _local_runtime_mutation_guard = if self.inner.remote_read_snapshot_loader.is_none()
            && lane.requires_local_runtime_mutation_barrier()
        {
            Some(self.inner.local_runtime_mutation_barrier.lock().await)
        } else {
            None
        };
        let mut service = self.take_service(lane).await;
        self.refresh_local_service_if_stale(lane, &mut service)?;
        let refresh_from_remote_changed = match service.refresh_from_remote_if_stale().await {
            Ok(changed) => changed,
            Err(error) => {
                self.restore_service(lane, service).await;
                return Err(ApiError::from(error));
            }
        };
        let result = operation(&mut service).await;
        let mark_remote_change_result = if result.is_ok() {
            service.mark_remote_change_observed().await.err()
        } else {
            None
        };
        self.update_remote_observation_after_mutation(
            refresh_from_remote_changed,
            result.is_ok(),
            mark_remote_change_result.is_some(),
        );
        let mut remote_change_watermark = service.observed_remote_change_watermark();
        let remote_snapshot_refresh = self
            .detached_remote_snapshot_refresh(refresh_from_remote_changed || result.is_ok())
            .await;
        let next_snapshot = if let Some(refresh_result) = remote_snapshot_refresh {
            match refresh_result {
                Ok((snapshot, change_watermark)) => {
                    remote_change_watermark = Some(change_watermark);
                    Some(snapshot)
                }
                Err(error) if result.is_ok() => {
                    self.inner
                        .remote_observation_degraded
                        .store(true, Ordering::Relaxed);
                    None
                }
                Err(error) => {
                    self.restore_service(lane, service).await;
                    return Err(ApiError::internal(error));
                }
            }
        } else if refresh_from_remote_changed {
            Some(Arc::new(service.read_snapshot()))
        } else if result.is_ok() {
            let current_snapshot = self.read_snapshot();
            Some(refresh(&service).apply(&current_snapshot))
        } else {
            None
        };
        self.restore_service(lane, service).await;
        if self.inner.remote_read_snapshot_loader.is_none() && result.is_ok() {
            let next_epoch = self
                .inner
                .local_change_epoch
                .fetch_add(1, Ordering::Relaxed)
                + 1;
            self.slot_for_lane(lane)
                .observed_local_change_epoch
                .store(next_epoch, Ordering::Relaxed);
        }
        if let Some(next_snapshot) = next_snapshot {
            self.publish_snapshot(next_snapshot, remote_change_watermark);
        }
        match result {
            Ok(value) => {
                if let Some(_error) = mark_remote_change_result {
                    // The business write is already durable at this point. Preserve the truthful
                    // success result and let later fresh-read revalidation recover the remote
                    // watermark if this observation probe failed.
                }
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    async fn drain_worker_until_idle(
        &self,
        request: DrainWorkerCommand,
    ) -> Result<DrainWorkerResponse, ApiError> {
        let max_commands = request
            .max_commands
            .ok_or_else(|| ApiError::bad_request("max_commands is required"))?;
        if max_commands == 0 {
            return Err(ApiError::bad_request(
                "max_commands must be greater than zero",
            ));
        }

        let mut response = DrainWorkerResponse {
            outcome: "idle".to_owned(),
            processed: 0,
            completed: 0,
            failed: 0,
            pending_remaining: 0,
            last_command_id: None,
            last_command_status: None,
            last_error_code: None,
            last_retryable: None,
        };

        for _ in 0..max_commands {
            let step = self
                .mutate_runtime(
                    |service| {
                        let request = DrainWorkerCommand {
                            max_commands: Some(1),
                            provider: request.provider.clone(),
                        };
                        Box::pin(async move {
                            service
                                .run_worker_until_idle(request)
                                .await
                                .map_err(ApiError::from)
                        })
                    },
                    Self::refresh_read_model_command_status_and_system_events_snapshot,
                )
                .await?;
            response.outcome = step.outcome.clone();
            response.pending_remaining = step.pending_remaining;
            response.last_command_id = step.last_command_id;
            response.last_command_status = step.last_command_status;
            response.last_error_code = step.last_error_code;
            response.last_retryable = step.last_retryable;
            response.processed += step.processed;
            response.completed += step.completed;
            response.failed += step.failed;
            if matches!(step.outcome.as_str(), "idle" | "drained") {
                break;
            }
        }

        Ok(response)
    }

    async fn drain_collection_scan_worker_until_idle(
        &self,
        request: DrainCollectionScanWorkerCommand,
    ) -> Result<DrainCollectionScanWorkerResponse, ApiError> {
        let max_collections = request
            .max_collections
            .ok_or_else(|| ApiError::bad_request("max_collections is required"))?;
        if max_collections == 0 {
            return Err(ApiError::bad_request(
                "max_collections must be greater than zero",
            ));
        }

        let mut response = DrainCollectionScanWorkerResponse {
            outcome: "idle".to_owned(),
            processed_collections: 0,
            enqueued_commands: 0,
            pending_due_remaining: 0,
            last_collection_key: None,
            partial_progress: false,
            last_error: None,
        };

        for _ in 0..max_collections {
            let step = self
                .mutate_runtime(
                    |service| {
                        let request = DrainCollectionScanWorkerCommand {
                            max_collections: Some(1),
                        };
                        Box::pin(async move {
                            service
                                .run_collection_scan_worker_until_idle(request)
                                .await
                                .map_err(ApiError::from)
                        })
                    },
                    Self::refresh_inventory_command_status_and_system_events_snapshot,
                )
                .await?;
            response.outcome = step.outcome.clone();
            response.pending_due_remaining = step.pending_due_remaining;
            response.last_collection_key = step.last_collection_key;
            response.partial_progress = step.partial_progress;
            response.last_error = step.last_error;
            response.processed_collections += step.processed_collections;
            response.enqueued_commands += step.enqueued_commands;
            if step.partial_progress || matches!(step.outcome.as_str(), "idle" | "drained") {
                break;
            }
        }

        Ok(response)
    }

    async fn drain_integration_worker_until_idle(
        &self,
        request: DrainIntegrationWorkerCommand,
    ) -> Result<DrainIntegrationWorkerResponse, ApiError> {
        let max_events = request
            .max_events
            .ok_or_else(|| ApiError::bad_request("max_events is required"))?;
        if max_events == 0 {
            return Err(ApiError::bad_request(
                "max_events must be greater than zero",
            ));
        }

        let mut response = DrainIntegrationWorkerResponse {
            outcome: "idle".to_owned(),
            attempted: 0,
            published: 0,
            pending_remaining: 0,
            last_event_id: None,
            last_event_kind: None,
            last_error: None,
            last_retryable: None,
        };

        for _ in 0..max_events {
            let step = self
                .mutate_publication(
                    |service| {
                        let request = DrainIntegrationWorkerCommand {
                            max_events: Some(1),
                            error_message: request.error_message.clone(),
                            retryable: request.retryable,
                        };
                        Box::pin(async move {
                            service
                                .publish_integration_events_until_idle(request)
                                .await
                                .map_err(ApiError::from)
                        })
                    },
                    Self::refresh_system_events_snapshot,
                )
                .await?;
            let has_error = step.last_error.is_some();
            response.outcome = step.outcome.clone();
            response.pending_remaining = step.pending_remaining;
            response.last_event_id = step.last_event_id;
            response.last_event_kind = step.last_event_kind;
            response.last_error = step.last_error;
            response.last_retryable = step.last_retryable;
            response.attempted += step.attempted;
            response.published += step.published;
            if has_error || matches!(step.outcome.as_str(), "idle" | "drained") {
                break;
            }
        }

        Ok(response)
    }
}

const fn remote_snapshot_is_current(
    current_change_watermark: u64,
    observed_change_watermark: u64,
    published_snapshot_watermark: u64,
) -> bool {
    current_change_watermark == observed_change_watermark
        || current_change_watermark == published_snapshot_watermark
}

const fn should_publish_remote_snapshot(
    loaded_change_watermark: u64,
    published_snapshot_watermark: u64,
) -> bool {
    loaded_change_watermark > published_snapshot_watermark
}

pub fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/dashboard/releases", get(release_dashboard))
        .merge(build_component_routes())
        .merge(build_collection_routes())
        .merge(build_governance_routes())
        .merge(build_runtime_routes())
        .with_state(state)
}

fn build_component_routes() -> Router<ApiState> {
    Router::new()
        .route("/components", post(register_component))
        .route(
            "/component-tags",
            post(register_component_tag).get(list_component_tags),
        )
        .route(
            "/component-tags/{tag_key}/components",
            post(add_component_to_tag),
        )
        .route(
            "/component-tags/{tag_key}/context-profile",
            post(assign_tag_context_profile),
        )
        .route(
            "/component-tags/{tag_key}/findings/risk-acceptance",
            post(accept_tag_risk),
        )
        .route(
            "/component-tags/{tag_key}/findings/suppression",
            post(suppress_tag_findings),
        )
        .route(
            "/context-profiles",
            post(register_context_profile).get(list_context_profiles),
        )
        .route("/components/{component_key}/artifacts", post(bind_artifact))
        .route(
            "/components/{component_key}/context-profile",
            post(assign_context_profile),
        )
        .route(
            "/components/{component_key}/provider-runtime",
            post(configure_provider),
        )
}

fn build_collection_routes() -> Router<ApiState> {
    Router::new()
        .route(
            "/collections",
            post(register_collection).get(list_collections),
        )
        .route("/collections/{collection_key}", get(collection_detail))
        .route(
            "/collections/{collection_key}/components",
            post(add_component_to_collection),
        )
        .route(
            "/collections/{collection_key}/components/{component_key}",
            axum::routing::delete(remove_component_from_collection),
        )
        .route(
            "/collections/{collection_key}/source",
            post(configure_collection_source),
        )
        .route(
            "/collections/{collection_key}/source/materialize",
            post(materialize_collection_source),
        )
        .route(
            "/collections/{collection_key}/scan-schedule",
            post(configure_collection_scan_schedule),
        )
        .route(
            "/collections/{collection_key}/context-profile",
            post(assign_collection_context_profile),
        )
        .route(
            "/collections/{collection_key}/scan-requests",
            post(request_collection_scan),
        )
        .route(
            "/collections/{collection_key}/findings/active",
            get(list_collection_active_findings),
        )
        .route(
            "/collections/{collection_key}/findings/risk-acceptance",
            post(accept_collection_risk),
        )
        .route(
            "/collections/{collection_key}/findings/suppression",
            post(suppress_collection_findings),
        )
        .route(
            "/collections/{collection_key}/findings/reopen",
            post(reopen_collection_findings),
        )
}

fn build_governance_routes() -> Router<ApiState> {
    Router::new()
        .route("/findings/risk-acceptance", post(accept_risk))
        .route("/findings/suppression", post(suppress_finding))
        .route("/findings/reopen", post(reopen_finding))
        .route("/findings/active", get(list_active_findings))
        .route("/system-events", get(list_system_events))
}

fn build_runtime_routes() -> Router<ApiState> {
    Router::new()
        .route("/integration-runtime", post(configure_integration_runtime))
        .route("/scan-requests", post(request_scan))
        .route("/scan-commands/{command_id}", get(scan_command_status))
        .route(
            "/collection-scan-workers/drain",
            post(drain_collection_scan_worker),
        )
        .route("/scan-workers/run-next", post(run_next_scan))
        .route("/scan-workers/drain", post(drain_worker))
        .route("/integration-workers/drain", post(drain_integration_worker))
        .route("/provider-reports", post(record_provider_report))
}

async fn health(State(state): State<ApiState>) -> &'static str {
    state.health_status().as_str()
}

async fn release_dashboard(
    State(state): State<ApiState>,
) -> Result<Json<ReleaseDashboardResponse>, ApiError> {
    let response = state
        .read_snapshot_fresh()
        .await?
        .release_dashboard()
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn list_system_events(
    State(state): State<ApiState>,
    Query(request): Query<SystemEventsQueryParams>,
) -> Result<Json<ListSystemEventsResponse>, ApiError> {
    let request = ListSystemEventsRequest {
        category: request.category,
        limit: request.limit,
    };
    Ok(Json(
        state
            .read_snapshot_fresh()
            .await?
            .list_system_events(&request)?,
    ))
}

async fn register_component(
    State(state): State<ApiState>,
    Json(request): Json<ComponentRegistrationRequest>,
) -> Result<Json<RegisterComponentResponse>, ApiError> {
    let response = state
        .mutate_runtime(
            |service| {
                Box::pin(async move {
                    service
                        .register_component(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn register_component_tag(
    State(state): State<ApiState>,
    Json(request): Json<ComponentTagRegistrationRequest>,
) -> Result<Json<RegisterComponentTagResponse>, ApiError> {
    let response = state
        .mutate_runtime(
            |service| {
                Box::pin(async move {
                    service
                        .register_component_tag(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn list_component_tags(
    State(state): State<ApiState>,
) -> Result<Json<ListComponentTagsResponse>, ApiError> {
    Ok(Json(
        state.read_snapshot_fresh().await?.list_component_tags(),
    ))
}

async fn register_context_profile(
    State(state): State<ApiState>,
    Json(request): Json<ContextProfileRegistrationRequest>,
) -> Result<Json<RegisterContextProfileResponse>, ApiError> {
    let response = state
        .mutate_runtime(
            |service| {
                Box::pin(async move {
                    service
                        .register_context_profile(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn add_component_to_tag(
    State(state): State<ApiState>,
    Path(tag_key): Path<String>,
    Json(request): Json<ComponentTagMembershipRequest>,
) -> Result<Json<ComponentTagMembershipResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .add_component_to_tag(&tag_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn assign_tag_context_profile(
    State(state): State<ApiState>,
    Path(tag_key): Path<String>,
    Json(request): Json<AssignTagContextProfileRequest>,
) -> Result<Json<AssignTagContextProfileResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .assign_context_profile_for_tag(&tag_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn list_context_profiles(
    State(state): State<ApiState>,
) -> Result<Json<ListContextProfilesResponse>, ApiError> {
    Ok(Json(
        state.read_snapshot_fresh().await?.list_context_profiles(),
    ))
}

async fn register_collection(
    State(state): State<ApiState>,
    Json(request): Json<CollectionRegistrationRequest>,
) -> Result<Json<RegisterCollectionResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .register_collection(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn list_collections(
    State(state): State<ApiState>,
) -> Result<Json<ListCollectionsResponse>, ApiError> {
    let response = state
        .read_snapshot_fresh()
        .await?
        .list_collections()
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn collection_detail(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
) -> Result<Json<CollectionDetailResponse>, ApiError> {
    let response = state
        .read_snapshot_fresh()
        .await?
        .collection_detail(&collection_key)
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn add_component_to_collection(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<CollectionMembershipRequest>,
) -> Result<Json<CollectionMembershipResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .add_component_to_collection(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn remove_component_from_collection(
    State(state): State<ApiState>,
    Path((collection_key, component_key)): Path<(String, String)>,
) -> Result<Json<CollectionMembershipResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .remove_component_from_collection(&collection_key, &component_key)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn configure_collection_source(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<ConfigureCollectionSourceRequest>,
) -> Result<Json<ConfigureCollectionSourceResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .configure_collection_source(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn materialize_collection_source(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
) -> Result<Json<MaterializeCollectionSourceResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .materialize_collection_source(&collection_key)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn configure_collection_scan_schedule(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<ConfigureCollectionScanScheduleRequest>,
) -> Result<Json<ConfigureCollectionScanScheduleResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .configure_collection_scan_schedule(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn bind_artifact(
    State(state): State<ApiState>,
    Path(component_key): Path<String>,
    Json(request): Json<BindArtifactRequest>,
) -> Result<Json<BindArtifactResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .bind_artifact(&component_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn assign_context_profile(
    State(state): State<ApiState>,
    Path(component_key): Path<String>,
    Json(request): Json<AssignContextProfileRequest>,
) -> Result<Json<AssignContextProfileResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .assign_context_profile(&component_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn assign_collection_context_profile(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<AssignCollectionContextProfileRequest>,
) -> Result<Json<AssignCollectionContextProfileResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .assign_collection_context_profile(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn configure_provider(
    State(state): State<ApiState>,
    Path(component_key): Path<String>,
    Json(request): Json<ConfigureProviderRequest>,
) -> Result<Json<ConfigureProviderResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .configure_provider(&component_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_inventory_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn configure_integration_runtime(
    State(state): State<ApiState>,
    Json(request): Json<ConfigureIntegrationRuntimeRequest>,
) -> Result<Json<ConfigureIntegrationRuntimeResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .configure_integration_runtime(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::unchanged_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn record_provider_report(
    State(state): State<ApiState>,
    Json(request): Json<ProviderScanReportRequest>,
) -> Result<Json<RecordProviderReportResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .record_provider_report(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn accept_risk(
    State(state): State<ApiState>,
    Json(request): Json<AcceptRiskRequest>,
) -> Result<Json<AcceptRiskResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move { service.accept_risk(request).await.map_err(ApiError::from) })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn accept_collection_risk(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<BulkAcceptRiskRequest>,
) -> Result<Json<BulkAcceptRiskResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .accept_risk_for_collection(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn accept_tag_risk(
    State(state): State<ApiState>,
    Path(tag_key): Path<String>,
    Json(request): Json<BulkAcceptRiskRequest>,
) -> Result<Json<BulkAcceptRiskByTagResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .accept_risk_for_tag(&tag_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn suppress_finding(
    State(state): State<ApiState>,
    Json(request): Json<SuppressFindingRequest>,
) -> Result<Json<SuppressFindingResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .suppress_finding(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn suppress_collection_findings(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<BulkSuppressFindingsRequest>,
) -> Result<Json<BulkSuppressFindingsResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .suppress_findings_for_collection(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn suppress_tag_findings(
    State(state): State<ApiState>,
    Path(tag_key): Path<String>,
    Json(request): Json<BulkSuppressFindingsRequest>,
) -> Result<Json<BulkSuppressFindingsByTagResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .suppress_findings_for_tag(&tag_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn reopen_finding(
    State(state): State<ApiState>,
    Json(request): Json<ReopenFindingRequest>,
) -> Result<Json<ReopenFindingResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .reopen_finding(request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn reopen_collection_findings(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<BulkReopenFindingsRequest>,
) -> Result<Json<BulkReopenFindingsResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .reopen_findings_for_collection(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_read_model_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn request_scan(
    State(state): State<ApiState>,
    Json(request): Json<RequestScanCommand>,
) -> Result<Json<RequestScanResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move { service.request_scan(request).await.map_err(ApiError::from) })
            },
            ApiState::refresh_command_status_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn request_collection_scan(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Json(request): Json<RequestCollectionScanCommand>,
) -> Result<Json<RequestCollectionScanResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(async move {
                    service
                        .request_collection_scan(&collection_key, request)
                        .await
                        .map_err(ApiError::from)
                })
            },
            ApiState::refresh_command_status_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn scan_command_status(
    State(state): State<ApiState>,
    Path(command_id): Path<String>,
) -> Result<Json<ScanCommandStatusResponse>, ApiError> {
    let response = state
        .read_snapshot_fresh()
        .await?
        .scan_command_status(&command_id)
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn drain_collection_scan_worker(
    State(state): State<ApiState>,
    Json(request): Json<DrainCollectionScanWorkerCommand>,
) -> Result<Json<DrainCollectionScanWorkerResponse>, ApiError> {
    let response = state
        .drain_collection_scan_worker_until_idle(request)
        .await?;
    Ok(Json(response))
}

async fn run_next_scan(
    State(state): State<ApiState>,
    Json(request): Json<RunNextScanCommand>,
) -> Result<Json<RunNextScanResponse>, ApiError> {
    let response = state
        .mutate(
            |service| {
                Box::pin(
                    async move { service.run_next_scan(request).await.map_err(ApiError::from) },
                )
            },
            ApiState::refresh_read_model_command_status_and_system_events_snapshot,
        )
        .await?;
    Ok(Json(response))
}

async fn drain_worker(
    State(state): State<ApiState>,
    Json(request): Json<DrainWorkerCommand>,
) -> Result<Json<DrainWorkerResponse>, ApiError> {
    let response = state.drain_worker_until_idle(request).await?;
    Ok(Json(response))
}

async fn drain_integration_worker(
    State(state): State<ApiState>,
    Json(request): Json<DrainIntegrationWorkerCommand>,
) -> Result<Json<DrainIntegrationWorkerResponse>, ApiError> {
    let response = state.drain_integration_worker_until_idle(request).await?;
    Ok(Json(response))
}

async fn list_active_findings(
    State(state): State<ApiState>,
    Query(query): Query<ActiveFindingsQuery>,
) -> Result<Json<ActiveFindingsResponse>, ApiError> {
    let response = state
        .read_snapshot_fresh()
        .await?
        .list_active_findings(query.into_request())
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

async fn list_collection_active_findings(
    State(state): State<ApiState>,
    Path(collection_key): Path<String>,
    Query(query): Query<CollectionActiveFindingsQuery>,
) -> Result<Json<CollectionActiveFindingsResponse>, ApiError> {
    let response = state
        .read_snapshot_fresh()
        .await?
        .list_collection_active_findings(&collection_key, query.into_request())
        .map_err(ApiError::from)?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct SystemEventsQueryParams {
    category: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ActiveFindingsQuery {
    component_key: String,
    artifact_kind: String,
    artifact_identity: String,
    min_severity: Option<String>,
    governance_state: Option<String>,
    package_name: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl ActiveFindingsQuery {
    fn into_request(self) -> service::ActiveFindingsRequest {
        service::ActiveFindingsRequest {
            component_key: self.component_key,
            artifact_kind: self.artifact_kind,
            artifact_identity: self.artifact_identity,
            min_severity: self.min_severity,
            governance_state: self.governance_state,
            package_name: self.package_name,
            offset: self.offset,
            limit: self.limit,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CollectionActiveFindingsQuery {
    min_severity: Option<String>,
    governance_state: Option<String>,
    package_name: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl CollectionActiveFindingsQuery {
    fn into_request(self) -> service::CollectionActiveFindingsRequest {
        service::CollectionActiveFindingsRequest {
            min_severity: self.min_severity,
            governance_state: self.governance_state,
            package_name: self.package_name,
            offset: self.offset,
            limit: self.limit,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<service::ApiApplicationError> for ApiError {
    fn from(value: service::ApiApplicationError) -> Self {
        match value {
            service::ApiApplicationError::InvalidRequest(message) => Self::bad_request(message),
            service::ApiApplicationError::NotFound(message) => Self {
                status: StatusCode::NOT_FOUND,
                message,
            },
            service::ApiApplicationError::State(message) => Self::internal(message),
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::ApiError;
    use super::ApiHealthStatus;
    use super::ApiMutationLane;
    use super::ApiState;
    use super::ComponentRegistrationRequest;
    use super::build_router;
    use super::remote_snapshot_is_current;
    use super::should_publish_remote_snapshot;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use sqlx::PgPool;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::util::ServiceExt;

    fn temp_path(name: &str, suffix: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("venom-api-{name}-{suffix}-{nanos}-{counter}.jsonl"))
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

    fn postgres_test_url() -> Option<String> {
        std::env::var("VENOM_TEST_POSTGRES_URL").ok()
    }

    #[test]
    fn remote_snapshot_is_current_when_published_detached_watermark_matches() {
        assert!(remote_snapshot_is_current(8, 7, 8));
    }

    #[test]
    fn remote_snapshot_is_current_when_live_store_observed_watermark_matches() {
        assert!(remote_snapshot_is_current(8, 8, 7));
    }

    #[test]
    fn stale_remote_snapshot_is_not_current() {
        assert!(!remote_snapshot_is_current(8, 7, 6));
    }

    #[test]
    fn detached_snapshot_publication_is_monotonic() {
        assert!(should_publish_remote_snapshot(9, 8));
        assert!(!should_publish_remote_snapshot(8, 8));
        assert!(!should_publish_remote_snapshot(7, 8));
    }

    #[test]
    fn publication_lane_does_not_take_the_state_consistency_barrier() {
        assert!(ApiMutationLane::State.requires_state_write_barrier());
        assert!(!ApiMutationLane::State.requires_state_read_barrier());
        assert!(ApiMutationLane::Runtime.requires_state_read_barrier());
        assert!(!ApiMutationLane::Publication.requires_state_read_barrier());
        assert!(!ApiMutationLane::Publication.requires_state_write_barrier());
        assert!(ApiMutationLane::Publication.requires_local_runtime_mutation_barrier());
    }

    #[tokio::test]
    async fn api_health_reports_degraded_when_remote_observation_is_stale() {
        let state = ApiState::open(
            temp_path("health-degraded", "state"),
            temp_path("health-degraded", "runtime"),
        )
        .expect("api state should open");
        let router = build_router(state.clone());

        assert_eq!(state.health_status(), ApiHealthStatus::Healthy);
        state
            .inner
            .remote_observation_degraded
            .store(true, Ordering::Relaxed);

        let response = router
            .oneshot(
                Request::get("/health")
                    .body(Body::empty())
                    .expect("health request should build"),
            )
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        assert_eq!(body.as_ref(), b"degraded");
    }

    #[tokio::test]
    async fn api_registers_binds_reports_and_queries_active_findings() {
        let router = build_router(
            ApiState::open(
                temp_path("integration", "state"),
                temp_path("integration", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = router
            .clone()
            .oneshot(
                Request::post("/components")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "name": "Payments API"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("register request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111",
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_queries_active_findings_with_filter_and_page_metadata() {
        let router = build_router(
            ApiState::open(
                temp_path("active-findings-query", "state"),
                temp_path("active-findings-query", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report_with_two_findings(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111&min_severity=high&limit=1&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(payload["returned"], 1);
        assert_eq!(payload["limit"], 1);
        assert_eq!(payload["offset"], 0);
        assert_eq!(
            payload["active_findings"][0]["vulnerability_id"],
            "CVE-2026-0001"
        );
    }

    #[tokio::test]
    async fn api_queries_active_findings_for_one_collection_scope() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-findings-query", "state"),
                temp_path("collection-findings-query", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_collection_schedule(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = assign_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/collections/release%3A2026.05/findings/active?package_name=openssl&limit=10&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["collection_key"], "release:2026.05");
        assert_eq!(payload["health"]["total"], 1);
        assert_eq!(payload["health"]["open"], 1);
        assert_eq!(payload["health"]["suppressed"], 0);
        assert_eq!(payload["bulk_governance"]["targeted"], 1);
        assert_eq!(payload["bulk_governance"]["critical_risk"], 1);
        assert_eq!(payload["bulk_governance"]["high_risk"], 0);
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(
            payload["active_findings"][0]["component_key"],
            "component:payments-api"
        );
        assert_eq!(
            payload["active_findings"][0]["artifact_identity"],
            "registry.example/payments@sha256:111"
        );
        assert_eq!(
            payload["active_findings"][0]["vulnerability_id"],
            "CVE-2026-0001"
        );
        assert_eq!(payload["active_findings"][0]["contextual_risk"], "critical");
        assert_eq!(
            payload["active_findings"][0]["context_profile_name"],
            "Internet Production"
        );
    }

    #[tokio::test]
    async fn api_assigns_one_default_context_profile_for_one_collection_scope() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-context-assignment", "state"),
                temp_path("collection-context-assignment", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = assign_internet_prod_context_profile_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["change"], "assigned");
        assert_eq!(payload["profile_key"], "context:internet-prod");

        let detail_response = router
            .clone()
            .oneshot(
                Request::get("/collections/release%3A2026.05")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("collection detail request should succeed");
        assert_eq!(detail_response.status(), StatusCode::OK);
        let detail_body = http_body_util::BodyExt::collect(detail_response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let detail_payload: serde_json::Value =
            serde_json::from_slice(&detail_body).expect("response should be valid json");
        assert_eq!(
            detail_payload["context_profile_key"],
            "context:internet-prod"
        );
        assert_eq!(
            detail_payload["members"][0]["context_profile_key"],
            "context:internet-prod"
        );
        assert_eq!(
            detail_payload["members"][0]["collection_context_profile"]["profile_key"],
            "context:internet-prod"
        );
    }

    #[tokio::test]
    async fn api_keeps_collection_health_when_collection_findings_are_governance_filtered() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-governance-overview", "state"),
                temp_path("collection-governance-overview", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_collection_schedule(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = assign_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report_with_two_findings(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = suppress_busybox_finding(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/collections/release%3A2026.05/findings/active?governance_state=suppressed&limit=10&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["health"]["total"], 2);
        assert_eq!(payload["health"]["open"], 1);
        assert_eq!(payload["health"]["suppressed"], 1);
        assert_eq!(payload["health"]["risk_accepted"], 0);
        assert_eq!(payload["health"]["critical_risk"], 1);
        assert_eq!(payload["health"]["high_risk"], 1);
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(
            payload["active_findings"][0]["governance_state"],
            "suppressed"
        );
    }

    #[tokio::test]
    async fn api_exposes_collection_health_overview() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-health-overview", "state"),
                temp_path("collection-health-overview", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_collection_schedule(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = assign_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report_with_two_findings(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = suppress_busybox_finding(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let list_response = router
            .clone()
            .oneshot(
                Request::get("/collections")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("list collections request should succeed");
        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body = http_body_util::BodyExt::collect(list_response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let list_payload: serde_json::Value =
            serde_json::from_slice(&list_body).expect("response should be valid json");
        assert_eq!(list_payload["collections"][0]["health"]["total"], 2);
        assert_eq!(list_payload["collections"][0]["health"]["open"], 1);
        assert_eq!(list_payload["collections"][0]["health"]["suppressed"], 1);
        assert_eq!(list_payload["collections"][0]["health"]["risk_accepted"], 0);
        assert_eq!(list_payload["collections"][0]["health"]["critical_risk"], 1);
        assert_eq!(list_payload["collections"][0]["health"]["high_risk"], 1);

        let detail_response = router
            .clone()
            .oneshot(
                Request::get("/collections/release%3A2026.05")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("collection detail request should succeed");
        assert_eq!(detail_response.status(), StatusCode::OK);
        let detail_body = http_body_util::BodyExt::collect(detail_response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let detail_payload: serde_json::Value =
            serde_json::from_slice(&detail_body).expect("response should be valid json");
        assert_eq!(detail_payload["health"]["total"], 2);
        assert_eq!(detail_payload["health"]["open"], 1);
        assert_eq!(detail_payload["health"]["suppressed"], 1);
        assert_eq!(detail_payload["health"]["critical_risk"], 1);
        assert_eq!(detail_payload["health"]["high_risk"], 1);

        let dashboard_response = router
            .clone()
            .oneshot(
                Request::get("/dashboard/releases")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("release dashboard request should succeed");
        assert_eq!(dashboard_response.status(), StatusCode::OK);
        let dashboard_body = http_body_util::BodyExt::collect(dashboard_response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let dashboard_payload: serde_json::Value =
            serde_json::from_slice(&dashboard_body).expect("response should be valid json");
        assert_eq!(dashboard_payload["summary"]["managed_collections"], 1);
        assert_eq!(dashboard_payload["summary"]["scheduled_collections"], 1);
        assert_eq!(dashboard_payload["summary"]["total_active_findings"], 2);
        assert_eq!(dashboard_payload["summary"]["open_findings"], 1);
        assert_eq!(dashboard_payload["summary"]["suppressed_findings"], 1);
        assert_eq!(dashboard_payload["summary"]["critical_risk_findings"], 1);
        assert_eq!(dashboard_payload["summary"]["high_risk_findings"], 1);
        assert_eq!(
            dashboard_payload["collections"][0]["collection_key"],
            "release:2026.05"
        );
        assert_eq!(dashboard_payload["collections"][0]["members"], 1);
        assert_eq!(dashboard_payload["collections"][0]["health"]["total"], 2);
    }

    #[tokio::test]
    async fn api_suppresses_one_active_finding_and_projects_the_state() {
        let router = build_router(
            ApiState::open(
                temp_path("suppress-finding", "state"),
                temp_path("suppress-finding", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/findings/suppression")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0001",
                            "package_name": "openssl",
                            "package_version": "3.0.0",
                            "package_purl": null,
                            "reason": "Known upstream false alarm"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("suppression request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111&governance_state=suppressed",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(
            payload["active_findings"][0]["governance_state"],
            "suppressed"
        );
        assert_eq!(
            payload["active_findings"][0]["governance_reason"],
            "Known upstream false alarm"
        );
        assert_eq!(payload["governance_state"], "suppressed");
    }

    #[tokio::test]
    async fn api_bulk_accepts_risk_for_open_collection_findings() {
        let router = build_router(
            ApiState::open(
                temp_path("bulk-accept-risk", "state"),
                temp_path("bulk-accept-risk", "runtime"),
            )
            .expect("api state should open"),
        );

        assert_eq!(
            register_payments_component(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            bind_owned_artifact(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            register_release_collection(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            add_payments_component_to_collection(router.clone())
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            record_provider_report_with_two_findings(router.clone())
                .await
                .status(),
            StatusCode::OK
        );

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release%3A2026.05/findings/risk-acceptance")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "min_severity": "high",
                            "reason": "Accepted for this release",
                            "until_unix_ms": 1_760_000_000_000_u64
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("bulk acceptance request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["targeted"], 1);
        assert_eq!(payload["accepted"], 1);
        assert_eq!(payload["unchanged"], 0);
        assert_eq!(payload["governance_state"], "risk-accepted");

        let response = router
            .oneshot(
                Request::get(
                    "/collections/release%3A2026.05/findings/active?governance_state=risk-accepted&limit=10&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(payload["health"]["open"], 1);
        assert_eq!(payload["health"]["risk_accepted"], 1);
        assert_eq!(
            payload["active_findings"][0]["vulnerability_id"],
            "CVE-2026-0001"
        );
    }

    #[tokio::test]
    async fn api_bulk_suppresses_open_collection_findings() {
        let router = build_router(
            ApiState::open(
                temp_path("bulk-suppress-findings", "state"),
                temp_path("bulk-suppress-findings", "runtime"),
            )
            .expect("api state should open"),
        );

        assert_eq!(
            register_payments_component(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            bind_owned_artifact(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            register_release_collection(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            add_payments_component_to_collection(router.clone())
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            record_provider_report_with_two_findings(router.clone())
                .await
                .status(),
            StatusCode::OK
        );

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release%3A2026.05/findings/suppression")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "min_severity": "high",
                            "reason": "Known upstream false alarm"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("bulk suppression request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["targeted"], 1);
        assert_eq!(payload["suppressed"], 1);
        assert_eq!(payload["unchanged"], 0);
        assert_eq!(payload["governance_state"], "suppressed");

        let response = router
            .oneshot(
                Request::get(
                    "/collections/release%3A2026.05/findings/active?governance_state=suppressed&limit=10&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(payload["health"]["open"], 1);
        assert_eq!(payload["health"]["suppressed"], 1);
        assert_eq!(
            payload["active_findings"][0]["governance_state"],
            "suppressed"
        );
        assert_eq!(
            payload["active_findings"][0]["governance_reason"],
            "Known upstream false alarm"
        );
    }

    #[tokio::test]
    async fn api_reopens_one_governed_finding_and_projects_the_state() {
        let router = build_router(
            ApiState::open(
                temp_path("reopen-finding", "state"),
                temp_path("reopen-finding", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/findings/suppression")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0001",
                            "package_name": "openssl",
                            "package_version": "3.0.0",
                            "package_purl": null,
                            "reason": "Known upstream false alarm"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("suppression request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/findings/reopen")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0001",
                            "package_name": "openssl",
                            "package_version": "3.0.0",
                            "package_purl": null
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("reopen request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111&governance_state=open",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["active_findings"][0]["governance_state"], "open");
        assert!(payload["active_findings"][0]["governance_reason"].is_null());
        assert_eq!(payload["governance_state"], "open");
    }

    #[tokio::test]
    async fn api_bulk_reopens_governed_collection_findings() {
        let router = build_router(
            ApiState::open(
                temp_path("bulk-reopen-findings", "state"),
                temp_path("bulk-reopen-findings", "runtime"),
            )
            .expect("api state should open"),
        );

        assert_eq!(
            register_payments_component(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            bind_owned_artifact(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            register_release_collection(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            add_payments_component_to_collection(router.clone())
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            record_provider_report_with_two_findings(router.clone())
                .await
                .status(),
            StatusCode::OK
        );

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release%3A2026.05/findings/risk-acceptance")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "min_severity": "high",
                            "reason": "Accepted for this release"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("bulk acceptance request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release%3A2026.05/findings/reopen")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "governance_state": "risk-accepted",
                            "min_severity": "high"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("bulk reopen request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["targeted"], 1);
        assert_eq!(payload["reopened"], 1);
        assert_eq!(payload["unchanged"], 0);
        assert_eq!(payload["result_governance_state"], "open");

        let response = router
            .oneshot(
                Request::get(
                    "/collections/release%3A2026.05/findings/active?governance_state=open&min_severity=high&limit=10&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(payload["health"]["open"], 2);
        assert_eq!(payload["health"]["risk_accepted"], 0);
        assert_eq!(payload["active_findings"][0]["governance_state"], "open");
    }

    #[tokio::test]
    async fn api_creates_release_collections_and_tracks_membership() {
        let router = build_router(
            ApiState::open(
                temp_path("collections", "state"),
                temp_path("collections", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::get("/collections")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("list collections request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["managed_collections"], 1);
        assert_eq!(
            payload["collections"][0]["collection_key"],
            "release:2026.05"
        );
        assert_eq!(payload["collections"][0]["members"], 1);
        assert_eq!(payload["collections"][0]["source"], serde_json::Value::Null);
        assert_eq!(
            payload["collections"][0]["scan_schedule"],
            serde_json::Value::Null
        );
        assert_eq!(payload["collections"][0]["due_now"], false);

        let response = router
            .oneshot(
                Request::get("/collections/release%3A2026.05")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("collection detail request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["members"][0]["key"], "component:payments-api");
        assert_eq!(payload["source"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn api_configures_and_materializes_collection_sources() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-sources", "state"),
                temp_path("collection-sources", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release%3A2026.05/source")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"kind":"component-list","mode":"replace","component_keys":["component:payments-api"]}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("collection source request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release%3A2026.05/source/materialize")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("collection source materialization should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["change"], "materialized");
        assert_eq!(payload["members"], 1);
        assert_eq!(payload["added"], 1);
        assert_eq!(payload["removed"], 0);

        let response = router
            .oneshot(
                Request::get("/collections/release%3A2026.05")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("collection detail request should succeed");
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["source"]["kind"], "component-list");
        assert_eq!(payload["source"]["mode"], "replace");
        assert_eq!(
            payload["source"]["component_keys"][0],
            "component:payments-api"
        );
    }

    #[tokio::test]
    async fn api_registers_context_profiles_and_assigns_one_to_one_component() {
        let router = build_router(
            ApiState::open(
                temp_path("context-profiles", "state"),
                temp_path("context-profiles", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = assign_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["change"], "assigned");
        assert_eq!(payload["profile_key"], "context:internet-prod");

        let response = router
            .clone()
            .oneshot(
                Request::get("/context-profiles")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("context profiles request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["managed_context_profiles"], 1);
        assert_eq!(
            payload["profiles"][0]["profile_key"],
            "context:internet-prod"
        );
        assert_eq!(payload["profiles"][0]["internet_exposed"], true);
        assert_eq!(payload["profiles"][0]["production"], true);
        assert_eq!(payload["profiles"][0]["mission_critical"], true);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("contextual active findings request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["active_findings"][0]["severity"], "high");
        assert_eq!(payload["active_findings"][0]["contextual_risk"], "critical");
        assert_eq!(
            payload["active_findings"][0]["context_profile_key"],
            "context:internet-prod"
        );
        assert_eq!(
            payload["active_findings"][0]["context_profile_name"],
            "Internet Production"
        );
    }

    #[tokio::test]
    async fn api_requests_collection_scan_batch() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-scan", "state"),
                temp_path("collection-scan", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_collection_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["collection_key"], "release:2026.05");
        assert_eq!(payload["freshness"], "deterministic");
        assert_eq!(payload["enqueued"], 1);
        assert_eq!(payload["command_ids"].as_array().map_or(0, Vec::len), 1);
    }

    #[tokio::test]
    async fn api_contextual_risk_reflects_vpn_restricted_non_privileged_context() {
        let router = build_router(
            ApiState::open(
                temp_path("contextual-risk-mitigated", "state"),
                temp_path("contextual-risk-mitigated", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_corp_api_private_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = assign_corp_api_private_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/provider-reports")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "provider_key": "fixture-provider",
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "observed_at_unix_ms": 1_763_232_000_000u64,
                            "freshness": "deterministic",
                            "knowledge_revision": "fixture-db:2026-05-16",
                            "findings": [
                                {
                                    "vulnerability_id": "CVE-2026-0001",
                                    "package_name": "openssl",
                                    "package_version": "3.0.0",
                                    "severity": "high"
                                }
                            ]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("provider report request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("contextual active findings request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["active_findings"][0]["severity"], "high");
        assert_eq!(payload["active_findings"][0]["contextual_risk"], "medium");
        assert_eq!(
            payload["active_findings"][0]["contextual_rule"],
            "mitigated-private-downgrade"
        );
        assert_eq!(
            payload["active_findings"][0]["context_profile_key"],
            "context:corp-api-private"
        );
        assert_eq!(
            payload["active_findings"][0]["context_profile_name"],
            "Corporate Private API"
        );
    }

    #[tokio::test]
    async fn api_requests_collection_scan_batch_for_multiple_members() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-scan-multi", "state"),
                temp_path("collection-scan-multi", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/components")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:billing-api",
                            "name": "Billing API"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("billing component request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/components/component:billing-api/artifacts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/billing@sha256:222"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("billing artifact request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release:2026.05/components")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "component_key": "component:billing-api" }).to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("billing membership request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_collection_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["collection_key"], "release:2026.05");
        assert_eq!(payload["freshness"], "deterministic");
        assert_eq!(payload["enqueued"], 2);
        assert_eq!(payload["command_ids"].as_array().map_or(0, Vec::len), 2);
    }

    #[tokio::test]
    async fn api_configures_collection_scan_schedule_and_exposes_it_in_detail() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-schedule", "state"),
                temp_path("collection-schedule", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_collection_schedule(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["change"], "configured");
        assert_eq!(payload["cadence_minutes"], 60);
        assert_eq!(payload["freshness"], "deterministic");

        let response = router
            .oneshot(
                Request::get("/collections/release%3A2026.05")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("collection detail request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["scan_schedule"]["cadence_minutes"], 60);
        assert_eq!(payload["scan_schedule"]["freshness"], "deterministic");
    }

    #[tokio::test]
    async fn api_lists_scheduled_collections_with_due_state() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-operations", "state"),
                temp_path("collection-operations", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"collection_key":"release:2026.07","name":"July Release"}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("second collection request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_collection_schedule(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .oneshot(
                Request::get("/collections")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("list collections request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(
            payload["collections"][0]["collection_key"],
            "release:2026.05"
        );
        assert_eq!(payload["collections"][0]["due_now"], true);
        assert_eq!(
            payload["collections"][0]["scan_schedule"]["cadence_minutes"],
            60
        );
        assert_eq!(
            payload["collections"][1]["collection_key"],
            "release:2026.07"
        );
        assert_eq!(
            payload["collections"][1]["scan_schedule"],
            serde_json::Value::Null
        );
        assert_eq!(payload["collections"][1]["due_now"], false);
    }

    #[tokio::test]
    async fn api_drains_due_collection_scan_schedules_into_pending_commands() {
        let router = build_router(
            ApiState::open(
                temp_path("collection-scheduler", "state"),
                temp_path("collection-scheduler", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_collection_schedule(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_collection_scheduler(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["processed_collections"], 1);
        assert_eq!(payload["enqueued_commands"], 1);
        assert_eq!(payload["pending_due_remaining"], 0);

        let response = router
            .oneshot(
                Request::get("/collections")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("list collections request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert!(
            payload["collections"][0]["scan_schedule"]["last_materialized_at_unix_ms"]
                .as_u64()
                .is_some()
        );
        assert_eq!(
            payload["collections"][0]["scan_schedule"]["last_enqueued_commands"],
            1
        );
    }

    #[tokio::test]
    async fn api_enqueues_scan_requests_and_exposes_pending_status() {
        let router = build_router(
            ApiState::open(
                temp_path("scan-request", "state"),
                temp_path("scan-request", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        let command_id = payload["command_id"]
            .as_str()
            .expect("command id should be present")
            .to_owned();
        assert_eq!(payload["status"], "pending");

        let response = router
            .oneshot(
                Request::get(format!("/scan-commands/{command_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("status request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_runs_next_scan_and_marks_command_completed() {
        let router = build_router(
            ApiState::open(
                temp_path("run-next", "state"),
                temp_path("run-next", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        let command_id = payload["command_id"]
            .as_str()
            .expect("command id should be present")
            .to_owned();

        let response = run_next_scan_with_fixture(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::get(format!("/scan-commands/{command_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("status request should succeed");
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["status"], "completed");
    }

    #[tokio::test]
    async fn api_drains_pending_scan_commands_until_idle() {
        let router = build_router(
            ApiState::open(
                temp_path("drain-worker", "state"),
                temp_path("drain-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["processed"], 2);
        assert_eq!(payload["completed"], 2);
        assert_eq!(payload["failed"], 0);
        assert_eq!(payload["pending_remaining"], 0);
    }

    #[tokio::test]
    async fn api_drains_pending_integration_events_from_state_and_runtime() {
        let router = build_router(
            ApiState::open(
                temp_path("drain-integration-worker", "state"),
                temp_path("drain-integration-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_success(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["attempted"], 2);
        assert_eq!(payload["published"], 2);
        assert_eq!(payload["pending_remaining"], 0);
        assert_eq!(payload["last_event_kind"], "scan-command-completed");
        assert!(payload["last_error"].is_null());
    }

    #[tokio::test]
    async fn api_keeps_pending_integration_events_on_publication_failure() {
        let router = build_router(
            ApiState::open(
                temp_path("fail-integration-worker", "state"),
                temp_path("fail-integration-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_failure(
            router.clone(),
            8,
            "fixture publish failed",
            true,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 0);
        assert_eq!(payload["pending_remaining"], 2);
        assert_eq!(payload["last_event_kind"], "finding-changes-observed");
        assert_eq!(payload["last_error"], "fixture publish failed");
        assert_eq!(payload["last_retryable"], true);
    }

    #[tokio::test]
    async fn api_exposes_http_publisher_transport_failure_explicitly() {
        let router = build_router(
            ApiState::open(
                temp_path("http-integration-worker", "state"),
                temp_path("http-integration-worker", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response =
            configure_http_integration_runtime(router.clone(), "http://127.0.0.1:9/publish", 3_000)
                .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_success(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 0);
        assert_eq!(payload["pending_remaining"], 2);
        assert_eq!(payload["last_retryable"], true);
        assert!(payload["last_error"].as_str().is_some());
    }

    #[tokio::test]
    async fn api_rejects_fixture_failure_controls_for_http_publisher() {
        let router = build_router(
            ApiState::open(
                temp_path("http-integration-failure", "state"),
                temp_path("http-integration-failure", "runtime"),
            )
            .expect("api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response =
            configure_http_integration_runtime(router.clone(), "http://127.0.0.1:9/publish", 3_000)
                .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_failure(
            router.clone(),
            8,
            "fixture publish failed",
            true,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(
            payload["error"],
            "http publisher does not accept fixture failure controls"
        );
    }

    #[tokio::test]
    async fn postgres_backend_reloads_findings_and_scan_status() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("reload");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        let command_id = payload["command_id"]
            .as_str()
            .expect("command id should be present")
            .to_owned();

        let response = run_next_scan_with_fixture(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = reloaded
            .clone()
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["active_findings"].as_array().map_or(0, Vec::len), 1);

        let response = reloaded
            .oneshot(
                Request::get(format!("/scan-commands/{command_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("status request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["status"], "completed");
    }

    #[tokio::test]
    async fn postgres_backend_reloads_suppressed_finding_state() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("suppression_reload");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/findings/suppression")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0001",
                            "package_name": "openssl",
                            "package_version": "3.0.0",
                            "package_purl": null,
                            "reason": "Known upstream false alarm"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("suppression request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = reloaded
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(
            payload["active_findings"][0]["governance_state"],
            "suppressed"
        );
        assert_eq!(
            payload["active_findings"][0]["governance_reason"],
            "Known upstream false alarm"
        );
    }

    #[tokio::test]
    async fn postgres_write_path_refreshes_remote_findings_before_governance_mutation() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("write_refresh");
        let primary = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(primary.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(primary.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let stale = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("second postgres api state should open"),
        );

        let response = configure_fixture_provider(primary.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(primary.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(primary.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = accept_openssl_risk(stale.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = stale
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111&governance_state=risk-accepted",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["active_findings"].as_array().map_or(0, Vec::len), 1);
        assert_eq!(
            payload["active_findings"][0]["governance_reason"],
            "Accepted after remote refresh"
        );
    }

    #[tokio::test]
    async fn detached_postgres_fresh_read_promotes_the_observed_change_watermark() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("detached_fresh_read_watermark");
        let primary = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );
        let stale = ApiState::open_postgres(&database_url, &schema)
            .await
            .expect("second postgres api state should open");

        let response = register_payments_component(primary.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let current_watermark = stale
            .inner
            .remote_change_probe
            .as_ref()
            .expect("postgres state should expose one remote probe")
            .current_change_watermark()
            .await
            .expect("current watermark should be readable");
        assert!(
            current_watermark
                > stale
                    .inner
                    .remote_change_probe
                    .as_ref()
                    .expect("postgres state should expose one remote probe")
                    .observed_change_watermark()
        );

        match stale.read_snapshot_fresh().await {
            Ok(_) => {}
            Err(error) => panic!("fresh read should succeed: {}", error.message),
        }

        assert_eq!(
            stale
                .inner
                .remote_change_probe
                .as_ref()
                .expect("postgres state should expose one remote probe")
                .observed_change_watermark(),
            current_watermark
        );
    }

    #[tokio::test]
    async fn postgres_mutation_returns_success_after_committed_write_even_if_watermark_probe_fails()
    {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("write_success_veracity");
        let state = ApiState::open_postgres(&database_url, &schema)
            .await
            .expect("postgres api state should open");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("postgres pool should connect");
        let operation_pool = pool.clone();
        let change_watermark_table = format!("{schema}.change_watermark");
        let components_table = format!("{schema}.components");

        let response = match state
            .mutate(
                move |service| {
                    let pool = operation_pool.clone();
                    Box::pin(async move {
                        let response = service
                            .register_component(ComponentRegistrationRequest {
                                component_key: "component:payments-api".to_owned(),
                                name: "Payments API".to_owned(),
                            })
                            .await
                            .map_err(ApiError::from)?;
                        sqlx::query(&format!("DROP TABLE {change_watermark_table}"))
                            .execute(&pool)
                            .await
                            .expect("dropping change watermark table should succeed");
                        Ok(response)
                    })
                },
                ApiState::refresh_inventory_snapshot,
            )
            .await
        {
            Ok(response) => response,
            Err(error) => panic!(
                "committed write must still return success: {}",
                error.message
            ),
        };
        assert_eq!(response.change, "registered");
        assert_eq!(response.managed_components, 1);

        let persisted_components: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {components_table} WHERE component_key = $1"
        ))
        .bind("component:payments-api")
        .fetch_one(&pool)
        .await
        .expect("component row count should load");
        assert_eq!(persisted_components, 1);
    }

    #[tokio::test]
    async fn postgres_backend_reloads_reopened_finding_as_open() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("reopen_reload");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = record_provider_report(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/findings/suppression")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0001",
                            "package_name": "openssl",
                            "package_version": "3.0.0",
                            "package_purl": null,
                            "reason": "Known upstream false alarm"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("suppression request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = router
            .clone()
            .oneshot(
                Request::post("/findings/reopen")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0001",
                            "package_name": "openssl",
                            "package_version": "3.0.0",
                            "package_purl": null
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("reopen request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = reloaded
            .oneshot(
                Request::get(
                    "/findings/active?component_key=component:payments-api&artifact_kind=container-image&artifact_identity=registry.example/payments@sha256:111&governance_state=open",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["active_findings"][0]["governance_state"], "open");
        assert!(payload["active_findings"][0]["governance_reason"].is_null());
    }

    #[tokio::test]
    async fn postgres_backend_reloads_bulk_risk_accepted_collection_findings() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("bulk_risk_acceptance_reload");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        assert_eq!(
            register_payments_component(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            bind_owned_artifact(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            register_release_collection(router.clone()).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            add_payments_component_to_collection(router.clone())
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            record_provider_report_with_two_findings(router.clone())
                .await
                .status(),
            StatusCode::OK
        );

        let response = router
            .clone()
            .oneshot(
                Request::post("/collections/release%3A2026.05/findings/risk-acceptance")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "min_severity": "high",
                            "reason": "Accepted for this release"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("bulk acceptance request should succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("reloaded postgres api state should open"),
        );
        let response = reloaded
            .oneshot(
                Request::get(
                    "/collections/release%3A2026.05/findings/active?governance_state=risk-accepted&limit=10&offset=0",
                )
                .body(Body::empty())
                .expect("request should build"),
            )
            .await
            .expect("query request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["total_active_findings"], 1);
        assert_eq!(payload["health"]["open"], 1);
        assert_eq!(payload["health"]["risk_accepted"], 1);
        assert_eq!(
            payload["active_findings"][0]["governance_state"],
            "risk-accepted"
        );
    }

    #[tokio::test]
    async fn postgres_backend_reloads_component_context_profiles() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("context_profile_reload");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = register_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = assign_internet_prod_context_profile(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = reloaded
            .oneshot(
                Request::get("/context-profiles")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("context profiles request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["managed_context_profiles"], 1);
        assert_eq!(
            payload["profiles"][0]["profile_key"],
            "context:internet-prod"
        );
        assert_eq!(payload["profiles"][0]["internet_exposed"], true);
        assert_eq!(payload["profiles"][0]["production"], true);
        assert_eq!(payload["profiles"][0]["mission_critical"], true);
    }

    #[tokio::test]
    async fn postgres_worker_loop_drains_until_idle() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("drain");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["completed"], 2);
        assert_eq!(payload["pending_remaining"], 0);
    }

    #[tokio::test]
    async fn postgres_collection_scan_request_reloads_pending_commands() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("collection_scan");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_collection_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        let command_id = payload["command_ids"][0]
            .as_str()
            .expect("command id should be present")
            .to_owned();

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = reloaded
            .oneshot(
                Request::get(format!("/scan-commands/{command_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("status request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["status"], "pending");
    }

    #[tokio::test]
    async fn postgres_collection_schedule_reloads_and_drains_due_commands() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("collection_schedule");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = register_release_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = add_payments_component_to_collection(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_collection_schedule(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = drain_collection_scheduler(reloaded.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["processed_collections"], 1);
        assert_eq!(payload["enqueued_commands"], 1);
        assert_eq!(payload["pending_due_remaining"], 0);
    }

    #[tokio::test]
    async fn postgres_integration_publication_worker_is_bounded_and_durable() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = configure_fixture_integration_runtime(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = drain_integration_worker_with_success(router.clone(), 1).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 1);
        assert_eq!(payload["pending_remaining"], 1);
        assert_eq!(payload["last_event_kind"], "finding-changes-observed");

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = drain_integration_worker_with_success(reloaded.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "drained");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 1);
        assert_eq!(payload["pending_remaining"], 0);
        assert_eq!(payload["last_event_kind"], "scan-command-completed");
    }

    #[tokio::test]
    async fn postgres_integration_runtime_reloads_and_publishes_over_http() {
        let Some(database_url) = postgres_test_url() else {
            return;
        };
        let schema = temp_schema("publish_http");
        let router = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should open"),
        );

        let response = register_payments_component(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = bind_owned_artifact(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = configure_fixture_provider(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response =
            configure_http_integration_runtime(router.clone(), "http://127.0.0.1:9/publish", 3_000)
                .await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = enqueue_scan_request(router.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let response = drain_worker_with_fixture(router.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);

        let reloaded = build_router(
            ApiState::open_postgres(&database_url, &schema)
                .await
                .expect("postgres api state should reopen"),
        );

        let response = drain_integration_worker_with_success(reloaded.clone(), 8).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .expect("response body should collect")
            .to_bytes();
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("response should be valid json");
        assert_eq!(payload["outcome"], "limited");
        assert_eq!(payload["attempted"], 1);
        assert_eq!(payload["published"], 0);
        assert_eq!(payload["pending_remaining"], 2);
        assert_eq!(payload["last_retryable"], true);
        assert!(payload["last_error"].as_str().is_some());
    }

    async fn bind_owned_artifact(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/components/component:payments-api/artifacts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("bind request should succeed")
    }

    async fn register_payments_component(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/components")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "name": "Payments API"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("register request should succeed")
    }

    async fn register_release_collection(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/collections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "collection_key": "release:2026.05",
                            "name": "May Release"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("register collection request should succeed")
    }

    async fn add_payments_component_to_collection(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/collections/release%3A2026.05/components")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("collection membership request should succeed")
    }

    async fn configure_fixture_provider(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/components/component:payments-api/provider-runtime")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "provider_key": "fixture-provider"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("configure provider request should succeed")
    }

    async fn register_internet_prod_context_profile(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/context-profiles")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "profile_key": "context:internet-prod",
                            "name": "Internet Production",
                            "internet_exposed": true,
                            "production": true,
                            "mission_critical": true
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("register context profile request should succeed")
    }

    async fn register_corp_api_private_context_profile(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/context-profiles")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "profile_key": "context:corp-api-private",
                            "name": "Corporate Private API",
                            "vpn_restricted": true,
                            "non_privileged_user": true
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("context profile request should succeed")
    }

    async fn assign_internet_prod_context_profile(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/components/component:payments-api/context-profile")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "profile_key": "context:internet-prod"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("assign context profile request should succeed")
    }

    async fn assign_corp_api_private_context_profile(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/components/component:payments-api/context-profile")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "profile_key": "context:corp-api-private"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("assign context profile request should succeed")
    }

    async fn assign_internet_prod_context_profile_to_collection(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/collections/release%3A2026.05/context-profile")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "profile_key": "context:internet-prod"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("assign collection context profile request should succeed")
    }

    async fn configure_collection_schedule(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/collections/release%3A2026.05/scan-schedule")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "cadence_minutes": 60,
                            "freshness": "deterministic"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("configure collection schedule request should succeed")
    }

    async fn configure_fixture_integration_runtime(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-runtime")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "publisher_key": "fixture-publisher"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("configure integration runtime request should succeed")
    }

    async fn configure_http_integration_runtime(
        router: axum::Router,
        endpoint_url: &str,
        timeout_ms: u32,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-runtime")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "publisher_key": "http-publisher",
                            "endpoint_url": endpoint_url,
                            "timeout_ms": timeout_ms
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("configure integration runtime request should succeed")
    }

    async fn record_provider_report(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/provider-reports")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "provider_key": "fixture-provider",
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "freshness": "deterministic",
                            "knowledge_revision": "fixture-db:2026-05-14",
                            "findings": [
                                {
                                    "vulnerability_id": "CVE-2026-0001",
                                    "package_name": "openssl",
                                    "package_version": "3.0.0",
                                    "severity": "high"
                                }
                            ]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("provider report request should succeed")
    }

    async fn record_provider_report_with_two_findings(
        router: axum::Router,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/provider-reports")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "provider_key": "fixture-provider",
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "freshness": "deterministic",
                            "knowledge_revision": "fixture-db:2026-05-16",
                            "findings": [
                                {
                                    "vulnerability_id": "CVE-2026-0001",
                                    "package_name": "openssl",
                                    "package_version": "3.0.0",
                                    "severity": "critical"
                                },
                                {
                                    "vulnerability_id": "CVE-2026-0002",
                                    "package_name": "busybox",
                                    "package_version": "1.36.0",
                                    "severity": "low"
                                }
                            ]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("provider report request should succeed")
    }

    async fn suppress_busybox_finding(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/findings/suppression")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0002",
                            "package_name": "busybox",
                            "package_version": "1.36.0",
                            "reason": "Known local suppression"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("suppression request should succeed")
    }

    async fn accept_openssl_risk(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/findings/risk-acceptance")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "vulnerability_id": "CVE-2026-0001",
                            "package_name": "openssl",
                            "package_version": "3.0.0",
                            "package_purl": null,
                            "reason": "Accepted after remote refresh"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("risk acceptance request should succeed")
    }

    async fn enqueue_scan_request(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/scan-requests")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "component_key": "component:payments-api",
                            "artifact_kind": "container-image",
                            "artifact_identity": "registry.example/payments@sha256:111",
                            "freshness": "deterministic"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("scan request should succeed")
    }

    async fn enqueue_collection_scan_request(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/collections/release%3A2026.05/scan-requests")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "freshness": "deterministic"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("collection scan request should succeed")
    }

    async fn run_next_scan_with_fixture(router: axum::Router) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/scan-workers/run-next")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "knowledge_revision": "fixture-db:2026-05-14",
                            "findings": [
                                {
                                    "vulnerability_id": "CVE-2026-0001",
                                    "package_name": "openssl",
                                    "package_version": "3.0.0",
                                    "severity": "high"
                                }
                            ]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("run-next request should succeed")
    }

    async fn drain_worker_with_fixture(
        router: axum::Router,
        max_commands: usize,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/scan-workers/drain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "max_commands": max_commands,
                            "knowledge_revision": "fixture-db:2026-05-14",
                            "findings": [
                                {
                                    "vulnerability_id": "CVE-2026-0001",
                                    "package_name": "openssl",
                                    "package_version": "3.0.0",
                                    "severity": "high"
                                }
                            ]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("drain request should succeed")
    }

    async fn drain_collection_scheduler(
        router: axum::Router,
        max_collections: usize,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/collection-scan-workers/drain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "max_collections": max_collections
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("collection scan drain request should succeed")
    }

    async fn drain_integration_worker_with_success(
        router: axum::Router,
        max_events: usize,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-workers/drain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "max_events": max_events
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("integration drain request should succeed")
    }

    async fn drain_integration_worker_with_failure(
        router: axum::Router,
        max_events: usize,
        error_message: &str,
        retryable: bool,
    ) -> axum::response::Response {
        router
            .oneshot(
                Request::post("/integration-workers/drain")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "max_events": max_events,
                            "error_message": error_message,
                            "retryable": retryable
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("integration drain request should succeed")
    }
}
