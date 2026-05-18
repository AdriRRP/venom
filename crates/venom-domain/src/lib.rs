pub mod durable_state;
pub mod findings;
pub mod integration;
pub mod inventory;
pub mod scanning;

pub use durable_state::{DurableState, DurableStateError};
pub use findings::{
    AcceptRiskChange, AcceptRiskResult, ActiveFindingProjection, ActiveFindingsPage,
    ActiveFindingsQuery, ArtifactKind, ArtifactRef,
    DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT, EvidenceFreshness, FindingChangeSet, FindingIngestion,
    FindingDecision, FindingGovernance, FindingGovernanceState, FindingIngestionError, FindingProvider,
    FindingProviderError, FindingProviderErrorKind, FindingReadModel, FindingRef, FindingTracker,
    MAX_ACTIVE_FINDINGS_PAGE_LIMIT, PackageCoordinate, ProviderScanReport, ReportedFinding,
    RiskAcceptance, ScanRequest, ScopedActiveFinding,
    ScopedActiveFindingsPage, ScopedActiveFindingsQuery, Severity,
};
pub use integration::{
    ConfigureIntegrationRuntimeChange, ConfigureIntegrationRuntimeResult, IntegrationEvent,
    IntegrationEventPublicationFailure, IntegrationEventPublishError, IntegrationEventPublisher,
    IntegrationRuntimeConfig, PendingIntegrationEvent, PublishIntegrationEventsResult,
};
pub use inventory::{
    AddCollectionComponentChange, AddCollectionComponentResult, BindArtifactChange,
    BindArtifactResult, CollectionRegistration, CollectionScanSchedule, CollectionScopedArtifact,
    ComponentInventory, ComponentRegistration, ConfigureCollectionScanScheduleChange,
    ConfigureCollectionScanScheduleResult, ConfigureProviderChange, ConfigureProviderResult,
    ManagedCollection, ManagedCollectionOperationsSummary, RegisterCollectionChange,
    RegisterCollectionResult, RegisterComponentChange, RegisterComponentResult,
    RemoveCollectionComponentChange, RemoveCollectionComponentResult,
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
