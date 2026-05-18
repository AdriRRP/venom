#![allow(clippy::needless_pass_by_ref_mut, clippy::unused_async)]

use cucumber::{World as _, given, then, when};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use venom_domain::durable_state::DurableState;
use venom_domain::findings::{
    ActiveFindingsPage, ActiveFindingsQuery, ArtifactKind, ArtifactRef, EvidenceFreshness,
    FindingChangeSet, FindingGovernanceState, FindingIngestion, FindingIngestionError,
    FindingProvider, FindingProviderError, FindingProviderErrorKind, FindingRef, PackageCoordinate,
    ProviderScanReport, ReportedFinding, RiskAcceptance, ScanRequest, ScopedActiveFindingsPage,
    ScopedActiveFindingsQuery, Severity, Suppression,
};
use venom_domain::inventory::{
    AddCollectionComponentResult, BindArtifactResult, CollectionRegistration,
    ComponentRegistration, ConfigureCollectionScanScheduleResult,
    ManagedCollectionOperationsSummary, RegisterCollectionResult, RegisterComponentResult,
};
use venom_domain::scanning::{
    CollectionScanBatch, CollectionScanPlanningError, CollectionScanScheduler, DueCollectionScan,
    RunNextScanResult, ScanCommandQueue, ScanExecutionResult, ScanPlanner, ScanPlanningError,
    execute_scan,
};

#[derive(Debug, Default, cucumber::World)]
struct AcceptanceWorld {
    component_key: Option<String>,
    artifact: Option<ArtifactRef>,
    pending_report: Option<ProviderScanReport>,
    ingestion: FindingIngestion,
    last_registration: Option<RegisterComponentResult>,
    last_collection_registration: Option<RegisterCollectionResult>,
    last_collection_membership: Option<AddCollectionComponentResult>,
    last_collection_scan_schedule: Option<ConfigureCollectionScanScheduleResult>,
    last_artifact_binding: Option<BindArtifactResult>,
    last_scan_request: Option<ScanRequest>,
    last_scan_planning_error: Option<ScanPlanningError>,
    last_collection_scan_batch: Option<CollectionScanBatch>,
    last_collection_scan_planning_error: Option<CollectionScanPlanningError>,
    last_due_collection_scans: Vec<DueCollectionScan>,
    last_collection_operations_summaries: Vec<ManagedCollectionOperationsSummary>,
    provider_failure: Option<FindingProviderError>,
    last_scan_execution: Option<ScanExecutionResult>,
    last_scan_execution_error: Option<String>,
    last_change_set: Option<FindingChangeSet>,
    last_ingestion_error: Option<FindingIngestionError>,
    durable_history_path: Option<PathBuf>,
    durable_state: Option<DurableState>,
    durable_runtime_path: Option<PathBuf>,
    durable_runtime: Option<ScanCommandQueue>,
    last_durable_command_id: Option<String>,
    last_durable_runtime_result: Option<String>,
    last_durable_runtime_error: Option<String>,
    last_durable_error: Option<String>,
    last_active_findings_page: Option<ActiveFindingsPage>,
    last_scoped_active_findings_page: Option<ScopedActiveFindingsPage>,
}

#[given("no managed components")]
async fn no_managed_components(world: &mut AcceptanceWorld) {
    world.ingestion = FindingIngestion::default();
    world.last_registration = None;
    world.last_collection_registration = None;
    world.last_collection_membership = None;
    world.last_collection_scan_schedule = None;
    world.last_artifact_binding = None;
    world.last_scan_request = None;
    world.last_scan_planning_error = None;
    world.last_collection_scan_batch = None;
    world.last_collection_scan_planning_error = None;
    world.last_due_collection_scans.clear();
    world.last_collection_operations_summaries.clear();
    world.provider_failure = None;
    world.last_scan_execution = None;
    world.last_scan_execution_error = None;
    world.last_change_set = None;
    world.last_ingestion_error = None;
    world.durable_history_path = None;
    world.durable_state = None;
    world.durable_runtime_path = None;
    world.durable_runtime = None;
    world.last_durable_command_id = None;
    world.last_durable_runtime_result = None;
    world.last_durable_runtime_error = None;
    world.last_durable_error = None;
    world.last_active_findings_page = None;
    world.last_scoped_active_findings_page = None;
}

#[given("a new durable state")]
async fn a_new_durable_state(world: &mut AcceptanceWorld) {
    let path = durable_history_path("acceptance-durable-state");
    world.durable_state = Some(
        DurableState::open(&path).expect("a new durable state must be creatable for acceptance"),
    );
    world.durable_history_path = Some(path);
    world.last_durable_error = None;
}

#[given("a new durable scan runtime")]
async fn a_new_durable_scan_runtime(world: &mut AcceptanceWorld) {
    let path = durable_history_path("acceptance-durable-runtime");
    world.durable_runtime = Some(
        ScanCommandQueue::open(&path)
            .expect("a new durable scan runtime must be creatable for acceptance"),
    );
    world.durable_runtime_path = Some(path);
    world.last_durable_command_id = None;
    world.last_durable_runtime_result = None;
    world.last_durable_runtime_error = None;
}

#[given(expr = "a component {string}")]
async fn a_component(world: &mut AcceptanceWorld, component_key: String) {
    world.component_key = Some(component_key);
}

#[given(expr = "a collection {string}")]
async fn a_collection(world: &mut AcceptanceWorld, collection_key: String) {
    world.last_collection_registration = Some(world.ingestion.inventory_mut().register_collection(
        CollectionRegistration::new(collection_key, "Fixture Collection"),
    ));
}

#[given(expr = "a managed component {string} named {string}")]
async fn a_managed_component(world: &mut AcceptanceWorld, component_key: String, name: String) {
    world.component_key = Some(component_key.clone());
    let _ = world
        .ingestion
        .inventory_mut()
        .register(ComponentRegistration::new(component_key, name));
}

#[given(expr = "a managed component {string} named {string} with artifact {string}")]
async fn a_managed_component_with_artifact(
    world: &mut AcceptanceWorld,
    component_key: String,
    name: String,
    artifact_identity: String,
) {
    a_managed_component(world, component_key.clone(), name).await;
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);
    world.artifact = Some(artifact.clone());
    let _ = world
        .ingestion
        .inventory_mut()
        .bind_artifact(&component_key, artifact);
}

#[given(expr = "an artifact {string}")]
async fn an_artifact(world: &mut AcceptanceWorld, artifact_identity: String) {
    world.artifact = Some(ArtifactRef::new(
        ArtifactKind::ContainerImage,
        artifact_identity,
    ));
}

#[given(
    expr = "a provider scan report with vulnerability {string} in package {string} version {string}"
)]
#[when(
    expr = "a provider scan report with vulnerability {string} in package {string} version {string}"
)]
async fn a_provider_scan_report_with_one_finding(
    world: &mut AcceptanceWorld,
    vulnerability_id: String,
    package_name: String,
    package_version: String,
) {
    world.pending_report = Some(build_report(
        world,
        vec![build_finding(
            vulnerability_id,
            package_name,
            package_version,
        )],
    ));
    world.provider_failure = None;
}

#[given(
    expr = "a provider scan report with a critical vulnerability {string} in package {string} version {string} and a low vulnerability {string} in package {string} version {string}"
)]
#[when(
    expr = "a provider scan report with a critical vulnerability {string} in package {string} version {string} and a low vulnerability {string} in package {string} version {string}"
)]
async fn a_provider_scan_report_with_critical_and_low_findings(
    world: &mut AcceptanceWorld,
    critical_vulnerability_id: String,
    critical_package_name: String,
    critical_package_version: String,
    low_vulnerability_id: String,
    low_package_name: String,
    low_package_version: String,
) {
    world.pending_report = Some(build_report(
        world,
        vec![
            build_finding(
                critical_vulnerability_id,
                critical_package_name,
                critical_package_version,
            )
            .with_severity(Severity::Critical),
            build_finding(low_vulnerability_id, low_package_name, low_package_version)
                .with_severity(Severity::Low),
        ],
    ));
    world.provider_failure = None;
}

#[given(
    expr = "a recorded provider scan report with vulnerability {string} in package {string} version {string}"
)]
async fn a_recorded_provider_scan_report(
    world: &mut AcceptanceWorld,
    vulnerability_id: String,
    package_name: String,
    package_version: String,
) {
    let report = build_report(
        world,
        vec![build_finding(
            vulnerability_id,
            package_name,
            package_version,
        )],
    );

    let _ = world.ingestion.record_scan_report(&report);
}

#[given(
    expr = "a current provider scan report with vulnerability {string} in package {string} version {string}"
)]
#[when(
    expr = "a current provider scan report with vulnerability {string} in package {string} version {string}"
)]
async fn a_current_provider_scan_report(
    world: &mut AcceptanceWorld,
    vulnerability_id: String,
    package_name: String,
    package_version: String,
) {
    world.pending_report = Some(build_report(
        world,
        vec![build_finding(
            vulnerability_id,
            package_name,
            package_version,
        )],
    ));
}

#[given("an empty current provider scan report")]
#[when("an empty current provider scan report")]
async fn an_empty_current_provider_scan_report(world: &mut AcceptanceWorld) {
    world.pending_report = Some(build_report(world, Vec::new()));
    world.provider_failure = None;
}

#[given("the provider is temporarily unavailable")]
async fn the_provider_is_temporarily_unavailable(world: &mut AcceptanceWorld) {
    world.provider_failure = Some(FindingProviderError::new(
        FindingProviderErrorKind::Unavailable,
        true,
        "fixture provider unavailable",
    ));
}

#[when("VENOM records the provider scan report")]
async fn venom_records_the_provider_scan_report(world: &mut AcceptanceWorld) {
    let report = world
        .pending_report
        .take()
        .expect("a pending provider scan report must exist");

    match world.ingestion.record_scan_report(&report) {
        Ok(change_set) => {
            world.last_change_set = Some(change_set);
            world.last_ingestion_error = None;
        }
        Err(error) => {
            world.last_change_set = None;
            world.last_ingestion_error = Some(error);
        }
    }
}

#[when(expr = "VENOM registers component {string} named {string}")]
async fn venom_registers_component(
    world: &mut AcceptanceWorld,
    component_key: String,
    name: String,
) {
    world.last_registration = Some(
        world
            .ingestion
            .inventory_mut()
            .register(ComponentRegistration::new(component_key, name)),
    );
}

#[given(expr = "VENOM creates collection {string} named {string}")]
#[when(expr = "VENOM creates collection {string} named {string}")]
async fn venom_creates_collection(
    world: &mut AcceptanceWorld,
    collection_key: String,
    name: String,
) {
    world.last_collection_registration = Some(
        world
            .ingestion
            .inventory_mut()
            .register_collection(CollectionRegistration::new(collection_key, name)),
    );
}

#[given(expr = "VENOM durably registers component {string} named {string}")]
#[when(expr = "VENOM durably registers component {string} named {string}")]
async fn venom_durably_registers_component(
    world: &mut AcceptanceWorld,
    component_key: String,
    name: String,
) {
    let result = world
        .durable_state_mut()
        .register_component(ComponentRegistration::new(component_key, name));
    match result {
        Ok(result) => {
            world.last_registration = Some(result);
            world.last_durable_error = None;
        }
        Err(error) => {
            world.last_registration = None;
            world.last_durable_error = Some(error.as_str().to_owned());
        }
    }
}

#[given(expr = "VENOM durably creates collection {string} named {string}")]
#[when(expr = "VENOM durably creates collection {string} named {string}")]
async fn venom_durably_creates_collection(
    world: &mut AcceptanceWorld,
    collection_key: String,
    name: String,
) {
    let result = world
        .durable_state_mut()
        .register_collection(CollectionRegistration::new(collection_key, name));
    match result {
        Ok(result) => {
            world.last_collection_registration = Some(result);
            world.last_durable_error = None;
        }
        Err(error) => {
            world.last_collection_registration = None;
            world.last_durable_error = Some(error.as_str().to_owned());
        }
    }
}

#[when(expr = "VENOM binds artifact {string} to component {string}")]
async fn venom_binds_artifact_to_component(
    world: &mut AcceptanceWorld,
    artifact_identity: String,
    component_key: String,
) {
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);
    world.last_artifact_binding = Some(
        world
            .ingestion
            .inventory_mut()
            .bind_artifact(&component_key, artifact),
    );
}

#[given(expr = "VENOM durably binds artifact {string} to component {string}")]
#[when(expr = "VENOM durably binds artifact {string} to component {string}")]
async fn venom_durably_binds_artifact_to_component(
    world: &mut AcceptanceWorld,
    artifact_identity: String,
    component_key: String,
) {
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);
    let result = world
        .durable_state_mut()
        .bind_artifact(&component_key, artifact);
    match result {
        Ok(result) => {
            world.last_artifact_binding = Some(result);
            world.last_durable_error = None;
        }
        Err(error) => {
            world.last_artifact_binding = None;
            world.last_durable_error = Some(error.as_str().to_owned());
        }
    }
}

#[given(expr = "VENOM adds component {string} to collection {string}")]
#[when(expr = "VENOM adds component {string} to collection {string}")]
async fn venom_adds_component_to_collection(
    world: &mut AcceptanceWorld,
    component_key: String,
    collection_key: String,
) {
    world.last_collection_membership = Some(
        world
            .ingestion
            .inventory_mut()
            .add_component_to_collection(&collection_key, &component_key),
    );
}

#[given(expr = "VENOM durably adds component {string} to collection {string}")]
#[when(expr = "VENOM durably adds component {string} to collection {string}")]
async fn venom_durably_adds_component_to_collection(
    world: &mut AcceptanceWorld,
    component_key: String,
    collection_key: String,
) {
    let result = world
        .durable_state_mut()
        .add_component_to_collection(&collection_key, &component_key);
    match result {
        Ok(result) => {
            world.last_collection_membership = Some(result);
            world.last_durable_error = None;
        }
        Err(error) => {
            world.last_collection_membership = None;
            world.last_durable_error = Some(error.as_str().to_owned());
        }
    }
}

#[given(
    expr = "VENOM schedules a deterministic collection scan for {string} every {int} minutes due at unix ms {int}"
)]
#[when(
    expr = "VENOM schedules a deterministic collection scan for {string} every {int} minutes due at unix ms {int}"
)]
async fn venom_schedules_a_deterministic_collection_scan(
    world: &mut AcceptanceWorld,
    collection_key: String,
    cadence_minutes: usize,
    next_due_at_unix_ms: usize,
) {
    world.last_collection_scan_schedule = Some(
        world
            .ingestion
            .inventory_mut()
            .configure_collection_scan_schedule(
                &collection_key,
                u32::try_from(cadence_minutes).expect("cadence should fit u32"),
                EvidenceFreshness::Deterministic,
                u64::try_from(next_due_at_unix_ms).expect("next due should fit u64"),
            ),
    );
}

#[given(
    expr = "VENOM durably schedules a deterministic collection scan for {string} every {int} minutes due at unix ms {int}"
)]
#[when(
    expr = "VENOM durably schedules a deterministic collection scan for {string} every {int} minutes due at unix ms {int}"
)]
async fn venom_durably_schedules_a_deterministic_collection_scan(
    world: &mut AcceptanceWorld,
    collection_key: String,
    cadence_minutes: usize,
    next_due_at_unix_ms: usize,
) {
    let Some(state) = world.durable_state.as_mut() else {
        world.last_durable_error = Some("missing durable state".to_owned());
        world.last_collection_scan_schedule = None;
        return;
    };
    match state.configure_collection_scan_schedule(
        &collection_key,
        u32::try_from(cadence_minutes).expect("cadence should fit u32"),
        EvidenceFreshness::Deterministic,
        u64::try_from(next_due_at_unix_ms).expect("next due should fit u64"),
    ) {
        Ok(result) => {
            world.last_collection_scan_schedule = Some(result);
            world.last_durable_error = None;
        }
        Err(error) => {
            world.last_collection_scan_schedule = None;
            world.last_durable_error = Some(error.as_str().to_owned());
        }
    }
}

#[when(expr = "VENOM plans a deterministic scan for component {string} and artifact {string}")]
async fn venom_plans_a_deterministic_scan(
    world: &mut AcceptanceWorld,
    component_key: String,
    artifact_identity: String,
) {
    plan_scan(
        world,
        &component_key,
        artifact_identity,
        EvidenceFreshness::Deterministic,
    );
}

#[when(expr = "VENOM plans a live scan for component {string} and artifact {string}")]
async fn venom_plans_a_live_scan(
    world: &mut AcceptanceWorld,
    component_key: String,
    artifact_identity: String,
) {
    plan_scan(
        world,
        &component_key,
        artifact_identity,
        EvidenceFreshness::Live,
    );
}

#[when(expr = "VENOM plans a deterministic collection scan for {string}")]
async fn venom_plans_a_deterministic_collection_scan(
    world: &mut AcceptanceWorld,
    collection_key: String,
) {
    match ScanPlanner::new(world.ingestion.inventory())
        .plan_collection(&collection_key, EvidenceFreshness::Deterministic)
    {
        Ok(batch) => {
            world.last_collection_scan_batch = Some(batch);
            world.last_collection_scan_planning_error = None;
        }
        Err(error) => {
            world.last_collection_scan_batch = None;
            world.last_collection_scan_planning_error = Some(error);
        }
    }
}

#[when(expr = "VENOM materializes due collection scans at unix ms {int} with limit {int}")]
async fn venom_materializes_due_collection_scans(
    world: &mut AcceptanceWorld,
    now_unix_ms: usize,
    limit: usize,
) {
    let mut scheduler = CollectionScanScheduler::new(world.ingestion.inventory_mut());
    world.last_due_collection_scans = scheduler.collect_due(
        u64::try_from(now_unix_ms).expect("current time should fit u64"),
        limit,
    );
}

#[when(expr = "VENOM lists collection schedules at unix ms {int}")]
async fn venom_lists_collection_schedules(world: &mut AcceptanceWorld, now_unix_ms: usize) {
    world.last_collection_operations_summaries =
        world.ingestion.inventory().collection_operations_summaries(
            u64::try_from(now_unix_ms).expect("current time should fit u64"),
        );
}

#[when(expr = "VENOM lists durable collection schedules at unix ms {int}")]
async fn venom_lists_durable_collection_schedules(world: &mut AcceptanceWorld, now_unix_ms: usize) {
    world.last_collection_operations_summaries = world
        .durable_state_ref()
        .ingestion()
        .inventory()
        .collection_operations_summaries(
            u64::try_from(now_unix_ms).expect("current time should fit u64"),
        );
}

#[when(
    expr = "VENOM durably plans a deterministic scan for component {string} and artifact {string}"
)]
async fn venom_durably_plans_a_deterministic_scan(
    world: &mut AcceptanceWorld,
    component_key: String,
    artifact_identity: String,
) {
    let inventory = world.durable_state_ref().ingestion().inventory().clone();
    plan_scan_from_inventory(
        world,
        &inventory,
        &component_key,
        artifact_identity,
        EvidenceFreshness::Deterministic,
    );
}

#[when(expr = "VENOM durably enqueues the planned scan")]
async fn venom_durably_enqueues_the_planned_scan(world: &mut AcceptanceWorld) {
    let request = world
        .last_scan_request
        .clone()
        .expect("a scan request must exist before durable enqueue");
    match world.durable_runtime_mut().enqueue(request) {
        Ok(result) => {
            world.last_durable_command_id = Some(result.command_id.into());
            world.last_durable_runtime_result = Some("pending".to_owned());
            world.last_durable_runtime_error = None;
        }
        Err(error) => {
            world.last_durable_command_id = None;
            world.last_durable_runtime_result = None;
            world.last_durable_runtime_error = Some(error.as_str().to_owned());
        }
    }
}

#[when("VENOM executes the planned scan")]
async fn venom_executes_the_planned_scan(world: &mut AcceptanceWorld) {
    let request = world
        .last_scan_request
        .clone()
        .expect("a scan request must exist before execution");
    let provider = AcceptanceFindingProvider {
        report: world.pending_report.clone(),
        error: world.provider_failure.clone(),
    };

    match execute_scan(&mut world.ingestion, &provider, &request).await {
        Ok(result) => {
            world.last_change_set = Some(result.change_set.clone());
            world.last_scan_execution = Some(result);
            world.last_scan_execution_error = None;
        }
        Err(error) => {
            world.last_scan_execution = None;
            world.last_scan_execution_error = Some(error.as_str().to_owned());
            world.last_change_set = None;
        }
    }
}

#[when("VENOM durably records the provider scan report")]
async fn venom_durably_records_the_provider_scan_report(world: &mut AcceptanceWorld) {
    let report = world
        .pending_report
        .take()
        .expect("a pending provider scan report must exist");
    match world.durable_state_mut().record_scan_report(&report) {
        Ok(change_set) => {
            world.last_change_set = Some(change_set);
            world.last_durable_error = None;
        }
        Err(error) => {
            world.last_change_set = None;
            world.last_durable_error = Some(error.as_str().to_owned());
        }
    }
}

#[when(
    expr = "VENOM durably accepts risk for vulnerability {string} in package {string} version {string} on component {string} and artifact {string} with reason {string}"
)]
async fn venom_durably_accepts_risk(
    world: &mut AcceptanceWorld,
    vulnerability_id: String,
    package_name: String,
    package_version: String,
    component_key: String,
    artifact_identity: String,
    reason: String,
) {
    let finding = FindingRef::new(
        component_key,
        ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity),
        vulnerability_id,
        PackageCoordinate::new(package_name, package_version),
    );
    match world
        .durable_state_mut()
        .accept_risk(finding, RiskAcceptance::new(reason))
    {
        Ok(_) => world.last_durable_error = None,
        Err(error) => world.last_durable_error = Some(error.as_str().to_owned()),
    }
}

#[when(
    expr = "VENOM durably accepts risk for vulnerability {string} in package {string} version {string} on component {string} and artifact {string} with reason {string} until unix ms {int}"
)]
#[allow(clippy::too_many_arguments)]
async fn venom_durably_accepts_risk_until(
    world: &mut AcceptanceWorld,
    vulnerability_id: String,
    package_name: String,
    package_version: String,
    component_key: String,
    artifact_identity: String,
    reason: String,
    until_unix_ms: usize,
) {
    let finding = FindingRef::new(
        component_key,
        ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity),
        vulnerability_id,
        PackageCoordinate::new(package_name, package_version),
    );
    match world.durable_state_mut().accept_risk(
        finding,
        RiskAcceptance::new(reason)
            .until_unix_ms(u64::try_from(until_unix_ms).expect("until should fit u64")),
    ) {
        Ok(_) => world.last_durable_error = None,
        Err(error) => world.last_durable_error = Some(error.as_str().to_owned()),
    }
}

#[when(
    expr = "VENOM durably suppresses vulnerability {string} in package {string} version {string} on component {string} and artifact {string} with reason {string}"
)]
async fn venom_durably_suppresses_finding(
    world: &mut AcceptanceWorld,
    vulnerability_id: String,
    package_name: String,
    package_version: String,
    component_key: String,
    artifact_identity: String,
    reason: String,
) {
    let finding = FindingRef::new(
        component_key,
        ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity),
        vulnerability_id,
        PackageCoordinate::new(package_name, package_version),
    );
    match world
        .durable_state_mut()
        .suppress_finding(finding, Suppression::new(reason))
    {
        Ok(_) => world.last_durable_error = None,
        Err(error) => world.last_durable_error = Some(error.as_str().to_owned()),
    }
}

#[when("VENOM durably runs the next queued scan")]
async fn venom_durably_runs_the_next_queued_scan(world: &mut AcceptanceWorld) {
    let provider = AcceptanceFindingProvider {
        report: world.pending_report.clone(),
        error: world.provider_failure.clone(),
    };
    let mut runtime = world
        .durable_runtime
        .take()
        .expect("a durable scan runtime must exist before durable runtime operations");
    let mut state = world
        .durable_state
        .take()
        .expect("a durable state must exist before durable runtime operations");
    match runtime.run_next(&mut state, &provider).await {
        Ok(RunNextScanResult::Idle) => {
            world.last_durable_runtime_result = Some("idle".to_owned());
            world.last_durable_runtime_error = None;
        }
        Ok(RunNextScanResult::Completed(result)) => {
            world.last_change_set = Some(result.change_set.clone());
            world.last_durable_command_id = Some(result.command_id.into());
            world.last_durable_runtime_result = Some("completed".to_owned());
            world.last_durable_runtime_error = None;
        }
        Ok(RunNextScanResult::Failed(result)) => {
            world.last_durable_command_id = Some(result.command_id.into());
            world.last_durable_runtime_result = Some("failed".to_owned());
            world.last_durable_runtime_error = Some(result.error_code.into());
        }
        Err(error) => {
            world.last_durable_runtime_result = None;
            world.last_durable_runtime_error = Some(error.as_str().to_owned());
        }
    }
    world.durable_runtime = Some(runtime);
    world.durable_state = Some(state);
}

#[when("VENOM reloads the durable state")]
async fn venom_reloads_the_durable_state(world: &mut AcceptanceWorld) {
    let path = world
        .durable_history_path
        .clone()
        .expect("a durable history path must exist before reload");
    match DurableState::open(&path) {
        Ok(state) => {
            world.durable_state = Some(state);
            world.last_durable_error = None;
        }
        Err(error) => {
            world.last_durable_error = Some(error.as_str().to_owned());
        }
    }
}

#[when(
    expr = "VENOM queries active findings for component {string} and artifact {string} with minimum severity {string}, offset {int}, and limit {int}"
)]
async fn venom_queries_active_findings_for_component_and_artifact(
    world: &mut AcceptanceWorld,
    component_key: String,
    artifact_identity: String,
    min_severity: String,
    offset: usize,
    limit: usize,
) {
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);
    let query = ActiveFindingsQuery::new(component_key, artifact)
        .with_min_severity(parse_severity(&min_severity))
        .with_offset(offset)
        .with_limit(limit);
    let page = world
        .durable_state_ref()
        .read_model()
        .query_active_findings(&query);
    world.last_active_findings_page = Some(page);
}

#[when(
    expr = "VENOM queries active findings for component {string} and artifact {string} with governance state {string}, minimum severity {string}, offset {int}, and limit {int}"
)]
async fn venom_queries_active_findings_for_component_and_artifact_with_governance(
    world: &mut AcceptanceWorld,
    component_key: String,
    artifact_identity: String,
    governance_state: String,
    min_severity: String,
    offset: usize,
    limit: usize,
) {
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);
    let query = ActiveFindingsQuery::new(component_key, artifact)
        .with_governance_state(parse_governance_state(&governance_state))
        .with_min_severity(parse_severity(&min_severity))
        .with_offset(offset)
        .with_limit(limit);
    let page = world
        .durable_state_ref()
        .read_model()
        .query_active_findings(&query);
    world.last_active_findings_page = Some(page);
}

#[when(
    expr = "VENOM queries active findings for collection {string} with minimum severity {string}, offset {int}, and limit {int}"
)]
async fn venom_queries_active_findings_for_collection(
    world: &mut AcceptanceWorld,
    collection_key: String,
    min_severity: String,
    offset: usize,
    limit: usize,
) {
    let scope = world
        .durable_state_ref()
        .ingestion()
        .inventory()
        .collection_scoped_artifacts(&collection_key)
        .expect("collection scope must exist before scoped finding query");
    let query = ScopedActiveFindingsQuery::new()
        .with_min_severity(parse_severity(&min_severity))
        .with_offset(offset)
        .with_limit(limit);
    let page = world
        .durable_state_ref()
        .read_model()
        .query_scoped_active_findings(&scope, &query);
    world.last_scoped_active_findings_page = Some(page);
}

#[when(
    expr = "VENOM queries active findings for collection {string} with governance state {string}, minimum severity {string}, offset {int}, and limit {int}"
)]
async fn venom_queries_active_findings_for_collection_with_governance(
    world: &mut AcceptanceWorld,
    collection_key: String,
    governance_state: String,
    min_severity: String,
    offset: usize,
    limit: usize,
) {
    let scope = world
        .durable_state_ref()
        .ingestion()
        .inventory()
        .collection_scoped_artifacts(&collection_key)
        .expect("collection scope must exist before scoped finding query");
    let query = ScopedActiveFindingsQuery::new()
        .with_governance_state(parse_governance_state(&governance_state))
        .with_min_severity(parse_severity(&min_severity))
        .with_offset(offset)
        .with_limit(limit);
    let page = world
        .durable_state_ref()
        .read_model()
        .query_scoped_active_findings(&scope, &query);
    world.last_scoped_active_findings_page = Some(page);
}

#[then(expr = "the component {string} is under management")]
async fn the_component_is_under_management(world: &mut AcceptanceWorld, component_key: String) {
    assert!(world.ingestion.inventory().is_managed(&component_key));
}

#[then(expr = "the durable state manages component {string}")]
async fn the_durable_state_manages_component(world: &mut AcceptanceWorld, component_key: String) {
    assert!(
        world
            .durable_state_ref()
            .ingestion()
            .inventory()
            .is_managed(&component_key)
    );
}

#[then(expr = "{int} component is under management")]
#[then(expr = "{int} components are under management")]
async fn components_are_under_management(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(world.ingestion.inventory().managed_components(), expected);
}

#[then(expr = "managed collections are {int}")]
async fn managed_collections_are(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(world.ingestion.inventory().managed_collections(), expected);
}

#[then(expr = "the registration result is {string}")]
async fn the_registration_result_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(last_registration(world).change.as_str(), expected);
}

#[then(expr = "the collection result is {string}")]
async fn the_collection_result_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_collection_registration
            .as_ref()
            .expect("a collection registration must be attempted before assertions")
            .change
            .as_str(),
        expected
    );
}

#[then(expr = "the collection membership result is {string}")]
async fn the_collection_membership_result_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_collection_membership
            .as_ref()
            .expect("a collection membership must be attempted before assertions")
            .change
            .as_str(),
        expected
    );
}

#[then(expr = "the collection scan schedule result is {string}")]
async fn the_collection_scan_schedule_result_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_collection_scan_schedule
            .as_ref()
            .expect("a collection scan schedule attempt must exist")
            .change
            .as_str(),
        expected
    );
}

#[then(expr = "collection {string} has {int} members")]
async fn collection_has_members(
    world: &mut AcceptanceWorld,
    collection_key: String,
    expected: usize,
) {
    assert_eq!(
        world
            .ingestion
            .inventory()
            .collection_member_count(&collection_key),
        expected
    );
}

#[then(expr = "collection {string} contains component {string}")]
async fn collection_contains_component(
    world: &mut AcceptanceWorld,
    collection_key: String,
    component_key: String,
) {
    let members = world
        .ingestion
        .inventory()
        .collection_members(&collection_key)
        .or_else(|| {
            world.durable_state.as_ref().and_then(|state| {
                state
                    .ingestion()
                    .inventory()
                    .collection_members(&collection_key)
            })
        })
        .expect("the collection must exist before membership assertions");
    assert!(
        members
            .iter()
            .any(|member| member.as_ref() == component_key)
    );
}

#[then(expr = "the durable state manages collection {string}")]
async fn the_durable_state_manages_collection(world: &mut AcceptanceWorld, collection_key: String) {
    assert!(
        world
            .durable_state_ref()
            .ingestion()
            .inventory()
            .is_collection_managed(&collection_key)
    );
}

#[then(expr = "the artifact {string} belongs to component {string}")]
async fn the_artifact_belongs_to_component(
    world: &mut AcceptanceWorld,
    artifact_identity: String,
    component_key: String,
) {
    assert!(world.ingestion.inventory().component_owns_artifact(
        &component_key,
        &ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity),
    ));
}

#[then(expr = "the durable state shows artifact {string} belongs to component {string}")]
async fn the_durable_state_shows_artifact_belongs_to_component(
    world: &mut AcceptanceWorld,
    artifact_identity: String,
    component_key: String,
) {
    assert!(
        world
            .durable_state_ref()
            .ingestion()
            .inventory()
            .component_owns_artifact(
                &component_key,
                &ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity),
            )
    );
}

#[then(expr = "{int} artifact is bound to component {string}")]
#[then(expr = "{int} artifacts are bound to component {string}")]
async fn artifacts_are_bound_to_component(
    world: &mut AcceptanceWorld,
    expected: usize,
    component_key: String,
) {
    assert_eq!(
        world.ingestion.inventory().bound_artifacts(&component_key),
        expected
    );
}

#[then(expr = "the artifact binding result is {string}")]
async fn the_artifact_binding_result_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_artifact_binding
            .expect("an artifact binding must be attempted before assertions")
            .change
            .as_str(),
        expected
    );
}

#[then(expr = "the report is rejected as {string}")]
async fn the_report_is_rejected_as(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_ingestion_error
            .expect("a finding ingestion error must exist")
            .as_str(),
        expected
    );
}

#[then(expr = "the scan planning is rejected as {string}")]
async fn the_scan_planning_is_rejected_as(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_scan_planning_error
            .expect("a scan planning error must exist")
            .as_str(),
        expected
    );
}

#[then(expr = "the collection scan planning is rejected as {string}")]
async fn the_collection_scan_planning_is_rejected_as(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_scan_planning_error
            .expect("a collection scan planning error must exist")
            .as_str(),
        expected
    );
}

#[then(expr = "the scan request targets component {string}")]
async fn the_scan_request_targets_component(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        last_scan_request(world).component_key.as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the scan request targets artifact {string}")]
async fn the_scan_request_targets_artifact(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        last_scan_request(world).artifact.identity.as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the scan request freshness is {string}")]
async fn the_scan_request_freshness_is(world: &mut AcceptanceWorld, expected: String) {
    let actual = match last_scan_request(world).freshness {
        EvidenceFreshness::Deterministic => "deterministic",
        EvidenceFreshness::Live => "live",
    };
    assert_eq!(actual, expected);
}

#[then(expr = "the collection scan batch targets collection {string}")]
async fn the_collection_scan_batch_targets_collection(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_scan_batch
            .as_ref()
            .expect("a collection scan batch must exist before assertions")
            .collection_key
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the collection scan batch has {int} requests")]
async fn the_collection_scan_batch_has_requests(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(
        world
            .last_collection_scan_batch
            .as_ref()
            .expect("a collection scan batch must exist before assertions")
            .requests
            .len(),
        expected
    );
}

#[then(expr = "{int} due collection scans are materialized")]
async fn due_collection_scans_are_materialized(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(world.last_due_collection_scans.len(), expected);
}

#[then(expr = "{int} collection schedules are visible")]
async fn collection_schedules_are_visible(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(world.last_collection_operations_summaries.len(), expected);
}

#[then(expr = "{int} collection schedules are due now")]
async fn collection_schedules_are_due_now(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .iter()
            .filter(|summary| summary.due_now)
            .count(),
        expected
    );
}

#[then(expr = "the first collection schedule targets collection {string}")]
async fn the_first_collection_schedule_targets_collection(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .first()
            .expect("a collection schedule summary must exist")
            .collection_key
            .as_ref(),
        expected
    );
}

#[then(expr = "the second collection schedule targets collection {string}")]
async fn the_second_collection_schedule_targets_collection(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .get(1)
            .expect("a second collection schedule summary must exist")
            .collection_key
            .as_ref(),
        expected
    );
}

#[then(expr = "the third collection schedule targets collection {string}")]
async fn the_third_collection_schedule_targets_collection(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .get(2)
            .expect("a third collection schedule summary must exist")
            .collection_key
            .as_ref(),
        expected
    );
}

#[then(expr = "the first collection schedule members are {int}")]
async fn the_first_collection_schedule_members_are(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .first()
            .expect("a collection schedule summary must exist")
            .members,
        expected
    );
}

#[then(expr = "the first collection schedule is due {string}")]
async fn the_first_collection_schedule_is_due(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .first()
            .expect("a collection schedule summary must exist")
            .due_now,
        expected == "true"
    );
}

#[then(expr = "the second collection schedule is due {string}")]
async fn the_second_collection_schedule_is_due(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .get(1)
            .expect("a second collection schedule summary must exist")
            .due_now,
        expected == "true"
    );
}

#[then(expr = "the third collection schedule has no periodic cadence")]
async fn the_third_collection_schedule_has_no_periodic_cadence(world: &mut AcceptanceWorld) {
    assert!(
        world
            .last_collection_operations_summaries
            .get(2)
            .expect("a third collection schedule summary must exist")
            .scan_schedule
            .is_none()
    );
}

#[then(expr = "the first collection schedule last ran at unix ms {int}")]
async fn the_first_collection_schedule_last_ran_at_unix_ms(
    world: &mut AcceptanceWorld,
    expected: usize,
) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .first()
            .and_then(|summary| summary.scan_schedule)
            .and_then(|schedule| schedule.last_materialized_at_unix_ms),
        Some(u64::try_from(expected).expect("expected time should fit u64"))
    );
}

#[then(expr = "the first collection schedule last enqueued {int} commands")]
async fn the_first_collection_schedule_last_enqueued_commands(
    world: &mut AcceptanceWorld,
    expected: usize,
) {
    assert_eq!(
        world
            .last_collection_operations_summaries
            .first()
            .and_then(|summary| summary.scan_schedule)
            .and_then(|schedule| schedule.last_enqueued_commands),
        Some(u32::try_from(expected).expect("expected command count should fit u32"))
    );
}

#[then(expr = "the first due collection scan targets collection {string}")]
async fn the_first_due_collection_scan_targets_collection(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_due_collection_scans
            .first()
            .expect("a due collection scan must exist")
            .collection_key
            .as_ref(),
        expected
    );
}

#[then(expr = "the first due collection scan has {int} requests")]
async fn the_first_due_collection_scan_has_requests(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(
        world
            .last_due_collection_scans
            .first()
            .expect("a due collection scan must exist")
            .requests
            .len(),
        expected
    );
}

#[then(expr = "collection {string} next due is unix ms {int}")]
async fn collection_next_due_is_unix_ms(
    world: &mut AcceptanceWorld,
    collection_key: String,
    expected: usize,
) {
    let expected = u64::try_from(expected).expect("expected due should fit u64");
    if let Some(state) = world.durable_state.as_ref() {
        assert_eq!(
            state
                .ingestion()
                .inventory()
                .collection_scan_schedule(&collection_key)
                .expect("the collection must have a schedule")
                .next_due_at_unix_ms,
            expected
        );
        return;
    }

    assert_eq!(
        world
            .ingestion
            .inventory()
            .collection_scan_schedule(&collection_key)
            .expect("the collection must have a schedule")
            .next_due_at_unix_ms,
        expected
    );
}

#[then(expr = "the first collection scan request targets component {string}")]
async fn the_first_collection_scan_request_targets_component(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_scan_batch
            .as_ref()
            .expect("a collection scan batch must exist before assertions")
            .requests[0]
            .component_key
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the first collection scan request targets artifact {string}")]
async fn the_first_collection_scan_request_targets_artifact(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_scan_batch
            .as_ref()
            .expect("a collection scan batch must exist before assertions")
            .requests[0]
            .artifact
            .identity
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the second collection scan request targets component {string}")]
async fn the_second_collection_scan_request_targets_component(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_scan_batch
            .as_ref()
            .expect("a collection scan batch must exist before assertions")
            .requests[1]
            .component_key
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the second collection scan request targets artifact {string}")]
async fn the_second_collection_scan_request_targets_artifact(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        world
            .last_collection_scan_batch
            .as_ref()
            .expect("a collection scan batch must exist before assertions")
            .requests[1]
            .artifact
            .identity
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the durable runtime has {int} pending scan command")]
#[then(expr = "the durable runtime has {int} pending scan commands")]
async fn the_durable_runtime_has_pending_scan_commands(
    world: &mut AcceptanceWorld,
    expected: usize,
) {
    assert_eq!(world.durable_runtime_ref().pending_commands(), expected);
}

#[then(expr = "the durable scan command status is {string}")]
async fn the_durable_scan_command_status_is(world: &mut AcceptanceWorld, expected: String) {
    let command_id = world
        .last_durable_command_id
        .as_deref()
        .expect("a durable command id must exist before status assertions");
    assert_eq!(
        world
            .durable_runtime_ref()
            .command_status(command_id)
            .expect("the durable command must exist")
            .as_str(),
        expected.as_str()
    );
}

#[then(expr = "the durable runtime result is {string}")]
async fn the_durable_runtime_result_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_durable_runtime_result
            .as_deref()
            .expect("a durable runtime result must exist"),
        expected.as_str()
    );
}

#[then(expr = "the durable runtime error is {string}")]
async fn the_durable_runtime_error_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_durable_runtime_error
            .as_deref()
            .expect("a durable runtime error must exist"),
        expected.as_str()
    );
}

#[then(expr = "the scan execution is rejected as {string}")]
async fn the_scan_execution_is_rejected_as(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_scan_execution_error
            .as_deref()
            .expect("a scan execution error must exist"),
        expected
    );
}

#[then(expr = "the executed scan uses provider {string}")]
async fn the_executed_scan_uses_provider(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        world
            .last_scan_execution
            .as_ref()
            .expect("a scan execution result must exist")
            .provider_key
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "{int} finding is reported by the provider snapshot")]
#[then(expr = "{int} findings are reported by the provider snapshot")]
async fn findings_are_reported_by_the_provider_snapshot(
    world: &mut AcceptanceWorld,
    expected: usize,
) {
    assert_eq!(
        world
            .last_scan_execution
            .as_ref()
            .expect("a scan execution result must exist")
            .findings_reported,
        expected
    );
}

#[then(expr = "{int} finding is newly discovered")]
#[then(expr = "{int} findings are newly discovered")]
async fn findings_are_newly_discovered(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_change_set(world).discovered, expected);
}

#[then(expr = "{int} finding is repeated")]
#[then(expr = "{int} findings are repeated")]
async fn findings_are_repeated(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_change_set(world).repeated, expected);
}

#[then(expr = "{int} finding is withdrawn")]
#[then(expr = "{int} findings are withdrawn")]
async fn findings_are_withdrawn(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_change_set(world).withdrawn, expected);
}

#[then(expr = "{int} finding is active for the artifact")]
#[then(expr = "{int} findings are active for the artifact")]
async fn findings_are_active_for_the_artifact(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_change_set(world).active, expected);
}

#[then(expr = "{int} active finding is projected for component {string} and artifact {string}")]
#[then(expr = "{int} active findings are projected for component {string} and artifact {string}")]
async fn active_findings_are_projected_for_component_and_artifact(
    world: &mut AcceptanceWorld,
    expected: usize,
    component_key: String,
    artifact_identity: String,
) {
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);
    assert_eq!(
        world
            .durable_state_ref()
            .read_model()
            .active_finding_count(&component_key, &artifact),
        expected
    );
}

#[then(expr = "vulnerability {string} is active for component {string} and artifact {string}")]
async fn vulnerability_is_active_for_component_and_artifact(
    world: &mut AcceptanceWorld,
    vulnerability_id: String,
    component_key: String,
    artifact_identity: String,
) {
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);
    assert!(
        world
            .durable_state_ref()
            .read_model()
            .has_active_vulnerability(&component_key, &artifact, &vulnerability_id)
    );
}

#[then(expr = "the active findings page total is {int}")]
async fn the_active_findings_page_total_is(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_active_findings_page(world).total, expected);
}

#[then(expr = "the active findings page returned count is {int}")]
async fn the_active_findings_page_returned_count_is(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_active_findings_page(world).returned, expected);
}

#[then(expr = "the active findings page limit is {int}")]
async fn the_active_findings_page_limit_is(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_active_findings_page(world).limit, expected);
}

#[then(expr = "the first active finding vulnerability is {string}")]
async fn the_first_active_finding_vulnerability_is(world: &mut AcceptanceWorld, expected: String) {
    assert_eq!(
        last_active_findings_page(world)
            .findings
            .first()
            .expect("an active finding must exist before first-finding assertions")
            .finding
            .vulnerability_id
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the first active finding governance state is {string}")]
async fn the_first_active_finding_governance_state_is(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        last_active_findings_page(world)
            .findings
            .first()
            .expect("an active finding must exist before governance assertions")
            .governance_state
            .as_str(),
        expected.as_str()
    );
}

#[then(expr = "the first active finding governance reason is {string}")]
async fn the_first_active_finding_governance_reason_is(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        last_active_findings_page(world)
            .findings
            .first()
            .and_then(|finding| finding.governance_reason.as_deref()),
        Some(expected.as_str())
    );
}

#[then(expr = "the first active finding governance until unix ms is {int}")]
async fn the_first_active_finding_governance_until_is(
    world: &mut AcceptanceWorld,
    expected: usize,
) {
    assert_eq!(
        last_active_findings_page(world)
            .findings
            .first()
            .and_then(|finding| finding.governance_until_unix_ms),
        Some(u64::try_from(expected).expect("expected until should fit u64"))
    );
}

#[then(expr = "the scoped active findings page total is {int}")]
async fn the_scoped_active_findings_page_total_is(world: &mut AcceptanceWorld, expected: usize) {
    assert_eq!(last_scoped_active_findings_page(world).total, expected);
}

#[then(expr = "the scoped active findings page returned count is {int}")]
async fn the_scoped_active_findings_page_returned_count_is(
    world: &mut AcceptanceWorld,
    expected: usize,
) {
    assert_eq!(last_scoped_active_findings_page(world).returned, expected);
}

#[then(expr = "the first scoped active finding component is {string}")]
async fn the_first_scoped_active_finding_component_is(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        last_scoped_active_findings_page(world)
            .findings
            .first()
            .expect("a scoped active finding must exist before first-finding assertions")
            .finding
            .component_key
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the first scoped active finding artifact is {string}")]
async fn the_first_scoped_active_finding_artifact_is(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        last_scoped_active_findings_page(world)
            .findings
            .first()
            .expect("a scoped active finding must exist before first-finding assertions")
            .finding
            .artifact
            .identity
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the first scoped active finding vulnerability is {string}")]
async fn the_first_scoped_active_finding_vulnerability_is(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        last_scoped_active_findings_page(world)
            .findings
            .first()
            .expect("a scoped active finding must exist before first-finding assertions")
            .finding
            .vulnerability_id
            .as_ref(),
        expected.as_str()
    );
}

#[then(expr = "the first scoped active finding governance state is {string}")]
async fn the_first_scoped_active_finding_governance_state_is(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        last_scoped_active_findings_page(world)
            .findings
            .first()
            .expect("a scoped active finding must exist before governance assertions")
            .governance_state
            .as_str(),
        expected.as_str()
    );
}

#[then(expr = "the first scoped active finding governance reason is {string}")]
async fn the_first_scoped_active_finding_governance_reason_is(
    world: &mut AcceptanceWorld,
    expected: String,
) {
    assert_eq!(
        last_scoped_active_findings_page(world)
            .findings
            .first()
            .and_then(|finding| finding.governance_reason.as_deref()),
        Some(expected.as_str())
    );
}

fn build_finding(
    vulnerability_id: String,
    package_name: String,
    package_version: String,
) -> ReportedFinding {
    ReportedFinding::new(
        vulnerability_id,
        PackageCoordinate::new(package_name, package_version),
    )
}

fn build_report(world: &AcceptanceWorld, findings: Vec<ReportedFinding>) -> ProviderScanReport {
    let component_key = world
        .component_key
        .clone()
        .expect("a component must be defined before building a provider scan report");
    let artifact = world
        .artifact
        .clone()
        .expect("an artifact must be defined before building a provider scan report");

    ProviderScanReport::new(
        "fixture-provider",
        component_key,
        artifact,
        SystemTime::UNIX_EPOCH,
        EvidenceFreshness::Deterministic,
        findings,
    )
}

fn plan_scan(
    world: &mut AcceptanceWorld,
    component_key: &str,
    artifact_identity: String,
    freshness: EvidenceFreshness,
) {
    let inventory = world.ingestion.inventory().clone();
    plan_scan_from_inventory(
        world,
        &inventory,
        component_key,
        artifact_identity,
        freshness,
    );
}

fn plan_scan_from_inventory(
    world: &mut AcceptanceWorld,
    inventory: &venom_domain::ComponentInventory,
    component_key: &str,
    artifact_identity: String,
    freshness: EvidenceFreshness,
) {
    let planner = ScanPlanner::new(inventory);
    let artifact = ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity);

    match planner.plan(component_key, artifact, freshness) {
        Ok(scan_request) => {
            world.last_scan_request = Some(scan_request);
            world.last_scan_planning_error = None;
        }
        Err(error) => {
            world.last_scan_request = None;
            world.last_scan_planning_error = Some(error);
        }
    }
}

#[derive(Debug, Clone)]
struct AcceptanceFindingProvider {
    report: Option<ProviderScanReport>,
    error: Option<FindingProviderError>,
}

impl FindingProvider for AcceptanceFindingProvider {
    fn provider_key(&self) -> &'static str {
        "fixture-provider"
    }

    async fn scan<'a>(
        &'a self,
        request: &'a ScanRequest,
    ) -> Result<ProviderScanReport, FindingProviderError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }

        let template = self
            .report
            .clone()
            .expect("a provider report fixture must exist for execution");
        let mut report = ProviderScanReport::new(
            self.provider_key(),
            request.component_key.clone(),
            request.artifact.clone(),
            SystemTime::UNIX_EPOCH,
            request.freshness,
            template.findings,
        );
        if request.freshness == EvidenceFreshness::Deterministic {
            report = report.with_knowledge_revision("fixture-db:2026-05-14");
        }

        Ok(report)
    }
}

impl AcceptanceWorld {
    const fn durable_state_mut(&mut self) -> &mut DurableState {
        self.durable_state
            .as_mut()
            .expect("a durable state must exist before durable operations")
    }

    const fn durable_state_ref(&self) -> &DurableState {
        self.durable_state
            .as_ref()
            .expect("a durable state must exist before durable assertions")
    }

    const fn durable_runtime_mut(&mut self) -> &mut ScanCommandQueue {
        self.durable_runtime
            .as_mut()
            .expect("a durable scan runtime must exist before durable runtime operations")
    }

    const fn durable_runtime_ref(&self) -> &ScanCommandQueue {
        self.durable_runtime
            .as_ref()
            .expect("a durable scan runtime must exist before durable runtime assertions")
    }
}

fn durable_history_path(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("venom-{prefix}-{nanos}-{counter}.jsonl"))
}

const fn last_change_set(world: &AcceptanceWorld) -> &FindingChangeSet {
    world
        .last_change_set
        .as_ref()
        .expect("a provider scan report must be recorded before assertions")
}

const fn last_registration(world: &AcceptanceWorld) -> &RegisterComponentResult {
    world
        .last_registration
        .as_ref()
        .expect("a component registration must be attempted before assertions")
}

const fn last_scan_request(world: &AcceptanceWorld) -> &ScanRequest {
    world
        .last_scan_request
        .as_ref()
        .expect("a scan request must be planned before assertions")
}

const fn last_active_findings_page(world: &AcceptanceWorld) -> &ActiveFindingsPage {
    world
        .last_active_findings_page
        .as_ref()
        .expect("an active findings query must be performed before assertions")
}

const fn last_scoped_active_findings_page(world: &AcceptanceWorld) -> &ScopedActiveFindingsPage {
    world
        .last_scoped_active_findings_page
        .as_ref()
        .expect("a scoped active findings query must be performed before assertions")
}

fn parse_severity(value: &str) -> Severity {
    match value {
        "unknown" => Severity::Unknown,
        "none" => Severity::None,
        "low" => Severity::Low,
        "medium" => Severity::Medium,
        "high" => Severity::High,
        "critical" => Severity::Critical,
        _ => panic!("unsupported severity in acceptance step: {value}"),
    }
}

fn parse_governance_state(value: &str) -> FindingGovernanceState {
    match value {
        "open" => FindingGovernanceState::Open,
        "risk-accepted" => FindingGovernanceState::RiskAccepted,
        "suppressed" => FindingGovernanceState::Suppressed,
        _ => panic!("unsupported governance state in acceptance step: {value}"),
    }
}

#[tokio::main]
async fn main() {
    let base = format!("{}/../../features", env!("CARGO_MANIFEST_DIR"));
    for feature in [
        "manage-collections.feature",
        "register-component.feature",
        "request-collection-scan.feature",
        "schedule-collection-scan.feature",
        "request-scan.feature",
        "report-finding.feature",
        "suppress-finding.feature",
        "filter-governed-findings.feature",
        "view-collection-schedules.feature",
        "view-active-findings.feature",
    ] {
        AcceptanceWorld::run(format!("{base}/{feature}")).await;
    }
}
