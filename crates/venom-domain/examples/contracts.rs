#![allow(clippy::needless_pass_by_ref_mut, clippy::unused_async)]

use std::{fs, path::PathBuf};
use venom_domain::{
    ArtifactKind, ArtifactRef, EvidenceFreshness, FindingProvider, FixtureSyftGrypeProvider,
    ScanRequest, artifact_identity_from_syft_json, as_provider_error,
    validate_provider_scan_report,
};

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
