pub mod durable_state;
pub mod findings;
pub mod integration;
pub mod inventory;
pub mod operations;
pub mod scanning;

pub use durable_state::{DurableState, DurableStateError};
pub use findings::{
    AcceptRiskChange, AcceptRiskResult, ActiveFindingProjection, ActiveFindingsPage,
    ActiveFindingsQuery, ArtifactKind, ArtifactRef, BulkAcceptRiskResult,
    BulkGovernanceCohortSummary, BulkReopenFindingResult, BulkSuppressFindingResult,
    CollectionGovernanceOverview, CollectionHealthSummary, ContextualActiveFindingProjection,
    ContextualRiskLevel, DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT, EvidenceFreshness, FindingChangeSet,
    FindingDecision, FindingGovernance, FindingGovernanceState, FindingIngestion,
    FindingIngestionError, FindingProvider, FindingProviderError, FindingProviderErrorKind,
    FindingReadModel, FindingRef, FindingTracker, MAX_ACTIVE_FINDINGS_PAGE_LIMIT,
    PackageCoordinate, ProviderScanReport, ReleaseBoard, ReleaseBoardCollection, ReleaseDashboard,
    ReleaseDashboardCollection, ReleaseDashboardSummary, ReopenFindingChange, ReopenFindingResult,
    ReportedFinding, RiskAcceptance, ScanRequest, ScopedActiveFinding, ScopedActiveFindingsPage,
    ScopedActiveFindingsQuery, Severity, SuppressFindingChange, SuppressFindingResult, Suppression,
    build_release_board, build_release_dashboard, contextual_risk_level,
    contextualize_active_findings, query_collection_governance_overview,
    summarize_collection_health,
};
pub use integration::{
    ConfigureIntegrationRuntimeChange, ConfigureIntegrationRuntimeResult, IntegrationEvent,
    IntegrationEventPublicationFailure, IntegrationEventPublishError, IntegrationEventPublisher,
    IntegrationRuntimeConfig, PendingIntegrationEvent, PublishIntegrationEventsResult,
};
pub use inventory::{
    AddCollectionComponentChange, AddCollectionComponentResult,
    AssignCollectionContextProfileChange, AssignCollectionContextProfileResult,
    AssignComponentTagChange, AssignComponentTagResult, AssignContextProfileChange,
    AssignContextProfileResult, AssignTagContextProfileChange, AssignTagContextProfileResult,
    BindArtifactChange, BindArtifactResult, CollectionRegistration, CollectionScanSchedule,
    CollectionScopedArtifact, CollectionSource, CollectionSourceKind, CollectionSourceMode,
    CollectionSourceSummary, ComponentInventory, ComponentListCollectionSource,
    ComponentRegistration, ComponentTagRegistration, ConfigureCollectionScanScheduleChange,
    ConfigureCollectionScanScheduleResult, ConfigureCollectionSourceChange,
    ConfigureCollectionSourceResult, ConfigureProviderChange, ConfigureProviderResult,
    ContextProfileRef, ContextProfileRegistration, ContextProfileValues, EffectiveContextProfile,
    ManagedCollection, ManagedCollectionOperationsSummary, ManagedComponentTag,
    ManagedContextProfile, MaterializeCollectionSourceChange, MaterializeCollectionSourceResult,
    RegisterCollectionChange, RegisterCollectionResult, RegisterComponentChange,
    RegisterComponentResult, RegisterComponentTagChange, RegisterComponentTagResult,
    RegisterContextProfileChange, RegisterContextProfileResult, RemoveCollectionComponentChange,
    RemoveCollectionComponentResult, TagContextConflict, TagContextField,
};
pub use operations::{
    DEFAULT_SYSTEM_EVENTS_LIMIT, MAX_SYSTEM_EVENTS_LIMIT, SystemEvent, SystemEventCategory,
    SystemEventKind, SystemEventsPage, SystemEventsQuery,
};
pub use scanning::{
    CollectionScanBatch, CollectionScanPlanningError, CollectionScanScheduler,
    CompletedScanCommand, DueCollectionScan, EnqueueScanResult, FailedScanCommand,
    RunNextScanResult, ScanCommandQueue, ScanCommandQueueError, ScanCommandStatus,
    ScanExecutionError, ScanExecutionResult, ScanPlanner, ScanPlanningError, execute_scan,
};

#[must_use]
pub const fn context_name() -> &'static str {
    "vulnerability-management"
}
