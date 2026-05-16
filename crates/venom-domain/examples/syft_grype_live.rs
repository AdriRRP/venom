#![allow(clippy::needless_pass_by_ref_mut, clippy::unused_async)]

use std::{env, fs, path::PathBuf, time::Duration};
use venom_domain::{
    ArtifactKind, ArtifactRef, DockerSyftGrypeProvider, EvidenceFreshness, FindingProvider,
    ScanRequest, artifact_identity_from_syft_json, as_provider_error,
    validate_provider_scan_report,
};

#[tokio::main]
async fn main() {
    let artifact_identity = artifact_identity_from_syft_json(&load_text(
        "tests/contracts/syft-grype/syft-alpine-3.21.json",
    ))
    .expect("syft fixture must expose a canonical immutable identity");
    let request = ScanRequest::new(
        "component:payments-api",
        ArtifactRef::new(ArtifactKind::ContainerImage, artifact_identity),
        EvidenceFreshness::Live,
    );
    let provider = DockerSyftGrypeProvider::official().with_timeout(live_timeout());

    let report = provider
        .scan(&request)
        .await
        .expect("docker-backed syft-grype provider must produce a live report");

    if let Err(violation) =
        validate_provider_scan_report(provider.provider_key(), &request, &report)
    {
        let error = as_provider_error(violation);
        panic!("provider contract violation: {}", error.message);
    }

    println!("RESULT: PASS");
}

fn load_text(relative_path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read fixture text at {}: {error}", path.display())
    })
}

fn live_timeout() -> Duration {
    let seconds = env::var("VENOM_LIVE_PROVIDER_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(180);
    Duration::from_secs(seconds)
}
