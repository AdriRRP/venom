#![allow(clippy::needless_pass_by_ref_mut, clippy::unused_async)]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use venom_domain::DurableState;
use venom_domain::findings::finding_provider_contract::{
    as_provider_error, validate_provider_scan_report,
};
use venom_domain::findings::{
    ArtifactKind, ArtifactRef, EvidenceFreshness, FindingProvider, PackageCoordinate,
    ProviderScanReport, ReportedFinding, ScanRequest,
};
use venom_domain::integration::{IntegrationEventPublishError, IntegrationEventPublisher};
use venom_domain::inventory::ComponentRegistration;
use venom_domain::scanning::syft_grype::{
    FixtureSyftGrypeProvider, artifact_identity_from_syft_json,
};
use venom_domain::scanning::{ScanCommandQueue, ScanPlanner};

#[tokio::main]
async fn main() {
    let provider = FixtureSyftGrypeProvider::from_paths(
        repo_path("tests/contracts/syft-grype/syft-alpine-3.21.json"),
        repo_path("tests/contracts/syft-grype/grype-alpine-3.21.json"),
        repo_path("tests/contracts/syft-grype/syft-alpine-3.21.json"),
        repo_path("tests/contracts/syft-grype/grype-alpine-3.21.json"),
    )
    .expect("syft-grype fixture provider must load");

    let artifact_identity = artifact_identity_from_syft_json(&load_text(
        "tests/contracts/syft-grype/syft-alpine-3.21.json",
    ))
    .expect("syft fixture must expose a canonical immutable identity");

    for freshness in [EvidenceFreshness::Deterministic, EvidenceFreshness::Live] {
        let request = ScanRequest::new(
            "component:payments-api",
            ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity.clone()),
            freshness,
        );
        run_contract_case(&provider, &request).await;
    }

    run_outbox_contracts().await;

    println!("RESULT: PASS");
}

async fn run_contract_case(provider: &(impl FindingProvider + Sync), request: &ScanRequest) {
    let report = provider
        .scan(request)
        .await
        .expect("fixture provider must produce a scan report");

    if let Err(violation) = validate_provider_scan_report(provider.provider_key(), request, &report)
    {
        let error = as_provider_error(violation);
        panic!("provider contract violation: {}", error.message);
    }
}

async fn run_outbox_contracts() {
    let state_path = temp_path("contracts-state");
    let runtime_path = temp_path("contracts-runtime");
    let artifact = ArtifactRef::new(
        ArtifactKind::ContainerImage,
        "registry.example/payments@sha256:111",
    );

    let mut state = DurableState::open(&state_path).expect("durable state should open");
    let _ = state
        .register_component(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ))
        .expect("registration should persist");
    let _ = state
        .bind_artifact("component:payments-api", artifact.clone())
        .expect("artifact binding should persist");
    let report = ProviderScanReport::new(
        "fixture-provider",
        "component:payments-api",
        artifact.clone(),
        SystemTime::UNIX_EPOCH,
        EvidenceFreshness::Deterministic,
        vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )],
    )
    .with_knowledge_revision("fixture-db:2026-05-16");
    let _ = state
        .record_scan_report(&report)
        .expect("provider report should append one pending integration event");
    assert_eq!(state.pending_integration_events().len(), 1);
    let publish_result = state
        .publish_pending_integration_events(1, &SuccessPublisher)
        .await
        .expect("successful publication must persist explicitly");
    assert_eq!(publish_result.published, 1);
    let rebuilt_state = DurableState::open(&state_path).expect("durable state should replay");
    assert_eq!(rebuilt_state.pending_integration_events().len(), 0);

    let mut runtime = ScanCommandQueue::open(&runtime_path).expect("runtime should open");
    let request = ScanPlanner::new(state.ingestion().inventory())
        .plan(
            "component:payments-api",
            artifact,
            EvidenceFreshness::Deterministic,
        )
        .expect("planner should create request");
    let _ = runtime.enqueue(request).expect("enqueue should persist");
    let _ = runtime
        .run_next(&mut state, &FixtureFindingProvider)
        .await
        .expect("runtime should append one pending completion integration event");
    assert_eq!(runtime.pending_integration_events().len(), 1);
    let failed_publish = runtime
        .publish_pending_integration_events(1, &FailingPublisher)
        .await
        .expect("failed publication outcome must persist explicitly");
    assert_eq!(failed_publish.published, 0);
    let rebuilt_runtime = ScanCommandQueue::open(&runtime_path).expect("runtime should replay");
    assert_eq!(rebuilt_runtime.pending_integration_events().len(), 1);
}

#[derive(Debug)]
struct SuccessPublisher;

impl IntegrationEventPublisher for SuccessPublisher {
    fn publisher_key(&self) -> &'static str {
        "fixture-publisher"
    }

    async fn publish<'a>(
        &'a self,
        _event: &'a venom_domain::PendingIntegrationEvent,
    ) -> Result<(), IntegrationEventPublishError> {
        Ok(())
    }
}

#[derive(Debug)]
struct FailingPublisher;

impl IntegrationEventPublisher for FailingPublisher {
    fn publisher_key(&self) -> &'static str {
        "fixture-publisher"
    }

    async fn publish<'a>(
        &'a self,
        _event: &'a venom_domain::PendingIntegrationEvent,
    ) -> Result<(), IntegrationEventPublishError> {
        Err(IntegrationEventPublishError::new(
            true,
            "publisher unavailable",
        ))
    }
}

#[derive(Debug)]
struct FixtureFindingProvider;

impl FindingProvider for FixtureFindingProvider {
    fn provider_key(&self) -> &'static str {
        "fixture-provider"
    }

    async fn scan<'a>(
        &'a self,
        request: &'a ScanRequest,
    ) -> Result<ProviderScanReport, venom_domain::FindingProviderError> {
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

fn load_text(relative_path: &str) -> String {
    let path = repo_path(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read fixture text at {}: {error}", path.display())
    })
}

fn repo_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(relative_path)
}

fn temp_path(name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("venom-contracts-{name}-{nanos}-{counter}.jsonl"))
}
