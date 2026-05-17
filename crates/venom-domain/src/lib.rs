pub mod durable_state;
pub mod findings;
pub mod integration;
pub mod inventory;
pub mod scanning;

pub use durable_state::{DurableState, DurableStateError};
pub use findings::{
    ActiveFindingsPage, ActiveFindingsQuery, ArtifactKind, ArtifactRef,
    DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT, EvidenceFreshness, FindingChangeSet, FindingIngestion,
    FindingIngestionError, FindingProvider, FindingProviderContractViolation, FindingProviderError,
    FindingProviderErrorKind, FindingReadModel, FindingTracker, MAX_ACTIVE_FINDINGS_PAGE_LIMIT,
    PackageCoordinate, ProviderScanReport, ReportedFinding, ScanRequest, Severity,
    as_provider_error, validate_provider_scan_report,
};
pub use integration::{
    ConfigureIntegrationRuntimeChange, ConfigureIntegrationRuntimeResult, IntegrationEvent,
    IntegrationEventPublicationFailure, IntegrationEventPublishError, IntegrationEventPublisher,
    IntegrationRuntimeConfig, PendingIntegrationEvent, PublishIntegrationEventsResult,
};
pub use inventory::{
    AddCollectionComponentChange, AddCollectionComponentResult, BindArtifactChange,
    BindArtifactResult, CollectionRegistration, ComponentInventory, ComponentRegistration,
    ConfigureProviderChange, ConfigureProviderResult, ManagedCollection, RegisterCollectionChange,
    RegisterCollectionResult, RegisterComponentChange, RegisterComponentResult,
    RemoveCollectionComponentChange, RemoveCollectionComponentResult,
};
pub use scanning::{
    CompletedScanCommand, DockerSyftGrypeProvider, DurableScanRuntime, DurableScanRuntimeError,
    EnqueueScanResult, FailedScanCommand, FixtureBundle, FixtureSyftGrypeProvider,
    OFFICIAL_GRYPE_IMAGE, OFFICIAL_SYFT_IMAGE, RunNextScanResult, SYFT_GRYPE_PROVIDER_KEY,
    ScanCommandStatus, ScanExecutionError, ScanExecutionResult, ScanPlanner, ScanPlanningError,
    artifact_identity_from_syft_json, execute_scan,
};

#[must_use]
pub const fn context_name() -> &'static str {
    "vulnerability-management"
}
