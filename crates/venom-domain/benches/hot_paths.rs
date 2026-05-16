use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::SystemTime;
use venom_domain::{
    ActiveFindingsQuery, ArtifactKind, ArtifactRef, ComponentRegistration, EvidenceFreshness,
    FindingIngestion, FindingReadModel, PackageCoordinate, ProviderScanReport, ReportedFinding,
    Severity,
};

const COMPONENT_KEY: &str = "component:payments-api";
const ARTIFACT_IDENTITY: &str = "registry.example/payments@sha256:111";
const FINDING_COUNTS: &[usize] = &[50, 200, 500];

fn hot_path_benchmarks(criterion: &mut Criterion) {
    let mut ingestion_group = criterion.benchmark_group("finding_ingestion");
    for count in FINDING_COUNTS {
        let report = provider_scan_report(*count);
        ingestion_group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |bencher, _| {
                bencher.iter_batched(
                    seeded_ingestion,
                    |mut ingestion| {
                        let change_set = ingestion
                            .record_scan_report(black_box(&report))
                            .expect("seeded ingestion should accept the benchmark report");
                        black_box(change_set);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    ingestion_group.finish();

    let mut query_group = criterion.benchmark_group("active_findings_query");
    for count in FINDING_COUNTS {
        let report = provider_scan_report(*count);
        let mut model = FindingReadModel::new();
        model.record_scan_report(&report);
        let query = ActiveFindingsQuery::new(COMPONENT_KEY, artifact_ref())
            .with_min_severity(Severity::Medium)
            .with_offset(0)
            .with_limit(50);
        query_group.bench_with_input(BenchmarkId::from_parameter(count), count, |bencher, _| {
            bencher.iter(|| {
                let page = model.query_active_findings(black_box(&query));
                black_box(page);
            });
        });
    }
    query_group.finish();
}

fn seeded_ingestion() -> FindingIngestion {
    let mut ingestion = FindingIngestion::new();
    let _ = ingestion
        .inventory_mut()
        .register(ComponentRegistration::new(COMPONENT_KEY, "Payments API"));
    let _ = ingestion
        .inventory_mut()
        .bind_artifact(COMPONENT_KEY, artifact_ref());
    ingestion
}

fn artifact_ref() -> ArtifactRef {
    ArtifactRef::new(ArtifactKind::ContainerImage, ARTIFACT_IDENTITY)
}

fn provider_scan_report(findings: usize) -> ProviderScanReport {
    ProviderScanReport::new(
        "fixture-provider",
        COMPONENT_KEY,
        artifact_ref(),
        SystemTime::UNIX_EPOCH,
        EvidenceFreshness::Deterministic,
        (0..findings).map(reported_finding).collect(),
    )
    .with_knowledge_revision("fixture-db:2026-05-16")
}

fn reported_finding(index: usize) -> ReportedFinding {
    let mut finding = ReportedFinding::new(
        format!("CVE-2026-{index:04}"),
        PackageCoordinate::new(
            format!("package-{index:04}"),
            format!("1.{}.{}", index % 10, index % 17),
        )
        .with_purl(format!(
            "pkg:oci/package-{index:04}@1.{}.{}",
            index % 10,
            index % 17
        )),
    );
    finding.severity = match index % 5 {
        0 => Severity::Critical,
        1 => Severity::High,
        2 => Severity::Medium,
        3 => Severity::Low,
        _ => Severity::Unknown,
    };
    finding
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    targets = hot_path_benchmarks
);
criterion_main!(benches);
