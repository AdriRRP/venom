pub mod component_inventory;
pub mod durable_scan_runtime;
pub mod durable_state;
pub mod finding_ingestion;
pub mod finding_provider;
pub mod finding_provider_contract;
pub mod finding_read_model;
pub mod finding_tracker;
pub mod scan_execution;
pub mod scan_planning;
pub mod syft_grype;

pub use component_inventory::{
    BindArtifactChange, BindArtifactResult, ComponentInventory, ComponentRegistration,
    RegisterComponentChange, RegisterComponentResult,
};
pub use durable_scan_runtime::{
    CompletedScanCommand, DurableScanRuntime, DurableScanRuntimeError, EnqueueScanResult,
    FailedScanCommand, RunNextScanResult, ScanCommandStatus,
};
pub use durable_state::{DurableState, DurableStateError};
pub use finding_ingestion::{FindingIngestion, FindingIngestionError};
pub use finding_provider::{
    ArtifactKind, ArtifactRef, EvidenceFreshness, FindingProvider, FindingProviderError,
    FindingProviderErrorKind, PackageCoordinate, ProviderScanReport, ReportedFinding, ScanRequest,
    Severity,
};
pub use finding_provider_contract::{
    FindingProviderContractViolation, as_provider_error, validate_provider_scan_report,
};
pub use finding_read_model::FindingReadModel;
pub use finding_tracker::{FindingChangeSet, FindingTracker};
pub use scan_execution::{ScanExecutionError, ScanExecutionResult, execute_scan};
pub use scan_planning::{ScanPlanner, ScanPlanningError};
pub use syft_grype::{
    DockerSyftGrypeProvider, FixtureBundle, FixtureSyftGrypeProvider, OFFICIAL_GRYPE_IMAGE,
    OFFICIAL_SYFT_IMAGE, SYFT_GRYPE_PROVIDER_KEY, artifact_identity_from_syft_json,
};

#[must_use]
pub const fn context_name() -> &'static str {
    "vulnerability-management"
}
