use crate::FindingReadModel;
use crate::findings::{CollectionHealthSummary, summarize_collection_health};
use crate::inventory::{CollectionScanSchedule, CollectionSourceSummary, ComponentInventory};

/// One compact reusable release-scoped read projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseBoard {
    pub collections: Vec<ReleaseBoardCollection>,
}

/// One release row reusable across operator-facing release views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseBoardCollection {
    pub collection_key: Box<str>,
    pub name: Box<str>,
    pub members: usize,
    pub source: Option<CollectionSourceSummary>,
    pub scan_schedule: Option<CollectionScanSchedule>,
    pub health: CollectionHealthSummary,
}

/// One operator-facing executive dashboard over managed release collections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDashboard {
    pub summary: ReleaseDashboardSummary,
    pub collections: Vec<ReleaseDashboardCollection>,
}

/// Compact aggregate summary over the visible release dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReleaseDashboardSummary {
    pub managed_collections: usize,
    pub scheduled_collections: usize,
    pub due_now_collections: usize,
    pub total_active_findings: usize,
    pub open_findings: usize,
    pub risk_accepted_findings: usize,
    pub suppressed_findings: usize,
    pub critical_risk_findings: usize,
    pub high_risk_findings: usize,
}

/// One release card in the dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDashboardCollection {
    pub collection_key: Box<str>,
    pub name: Box<str>,
    pub members: usize,
    pub due_now: bool,
    pub scan_schedule: Option<CollectionScanSchedule>,
    pub health: CollectionHealthSummary,
}

/// Build one read-side release dashboard from managed collection summaries and
/// collection-scoped health projections.
#[must_use]
pub fn build_release_board(
    inventory: &ComponentInventory,
    read_model: &FindingReadModel,
) -> ReleaseBoard {
    let collections = inventory
        .collection_operations_summaries(0)
        .into_iter()
        .map(|collection| {
            let health = inventory
                .collection_scoped_artifacts(collection.collection_key.as_ref())
                .map(|scope| {
                    summarize_collection_health(
                        inventory,
                        read_model,
                        collection.collection_key.as_ref(),
                        &scope,
                    )
                })
                .unwrap_or_default();

            ReleaseBoardCollection {
                collection_key: collection.collection_key,
                name: collection.name,
                members: collection.members,
                source: collection.source,
                scan_schedule: collection.scan_schedule,
                health,
            }
        })
        .collect::<Vec<_>>();

    ReleaseBoard { collections }
}

/// Build one read-side release dashboard from one reusable release board.
#[must_use]
pub fn build_release_dashboard(board: &ReleaseBoard, now_unix_ms: u64) -> ReleaseDashboard {
    let collections = board
        .collections
        .iter()
        .map(|collection| ReleaseDashboardCollection {
            collection_key: collection.collection_key.clone(),
            name: collection.name.clone(),
            members: collection.members,
            due_now: collection
                .scan_schedule
                .is_some_and(|schedule| schedule.next_due_at_unix_ms <= now_unix_ms),
            scan_schedule: collection.scan_schedule,
            health: collection.health,
        })
        .collect::<Vec<_>>();

    let summary = collections.iter().fold(
        ReleaseDashboardSummary {
            managed_collections: collections.len(),
            ..ReleaseDashboardSummary::default()
        },
        |mut summary, collection| {
            if collection.scan_schedule.is_some() {
                summary.scheduled_collections += 1;
            }
            if collection.due_now {
                summary.due_now_collections += 1;
            }
            summary.total_active_findings += collection.health.total;
            summary.open_findings += collection.health.open;
            summary.risk_accepted_findings += collection.health.risk_accepted;
            summary.suppressed_findings += collection.health.suppressed;
            summary.critical_risk_findings += collection.health.critical_risk;
            summary.high_risk_findings += collection.health.high_risk;
            summary
        },
    );

    ReleaseDashboard {
        summary,
        collections,
    }
}

#[cfg(test)]
mod tests {
    use super::{ReleaseBoard, build_release_board, build_release_dashboard};
    use crate::findings::{
        FindingReadModel, FindingRef, PackageCoordinate, ProviderScanReport, ReportedFinding,
        Severity, Suppression,
    };
    use crate::inventory::{
        CollectionRegistration, ComponentInventory, ComponentRegistration,
        ContextProfileRegistration,
    };
    use crate::{ArtifactKind, ArtifactRef, EvidenceFreshness};

    fn sample_release_board() -> ReleaseBoard {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.bind_artifact(
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
        );
        let _ = inventory.register(ComponentRegistration::new(
            "component:billing-api",
            "Billing API",
        ));
        let _ = inventory.bind_artifact(
            "component:billing-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/billing@sha256:222",
            ),
        );
        let _ = inventory.register_context_profile(ContextProfileRegistration::new(
            "context:internet-prod",
            "Internet Production",
            true,
            true,
            true,
        ));
        let _ = inventory.assign_context_profile("component:payments-api", "context:internet-prod");
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:payments-api");
        let _ = inventory.configure_collection_scan_schedule(
            "release:2026.05",
            60,
            EvidenceFreshness::Deterministic,
            1_000,
        );
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.06",
            "June Release",
        ));
        let _ = inventory.add_component_to_collection("release:2026.06", "component:billing-api");

        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            std::time::SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![
                ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
                    .with_severity(Severity::Critical),
                ReportedFinding::new("CVE-2026-0002", PackageCoordinate::new("busybox", "1.36.1"))
                    .with_severity(Severity::Low),
            ],
        );

        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report);
        read_model.suppress(
            FindingRef::new(
                "component:payments-api",
                ArtifactRef::new(
                    ArtifactKind::ContainerImage,
                    "registry.example/payments@sha256:111",
                ),
                "CVE-2026-0002",
                PackageCoordinate::new("busybox", "1.36.1"),
            ),
            Suppression::new("Known upstream false alarm"),
        );

        build_release_board(&inventory, &read_model)
    }

    #[test]
    fn release_dashboard_aggregates_managed_collection_health() {
        let board = sample_release_board();
        let dashboard = build_release_dashboard(&board, 1_500);

        assert_eq!(dashboard.summary.managed_collections, 2);
        assert_eq!(dashboard.summary.scheduled_collections, 1);
        assert_eq!(dashboard.summary.due_now_collections, 1);
        assert_eq!(dashboard.summary.total_active_findings, 2);
        assert_eq!(dashboard.summary.open_findings, 1);
        assert_eq!(dashboard.summary.suppressed_findings, 1);
        assert_eq!(dashboard.summary.risk_accepted_findings, 0);
        assert_eq!(dashboard.summary.critical_risk_findings, 1);
        assert_eq!(dashboard.summary.high_risk_findings, 1);
        assert_eq!(
            dashboard.collections[0].collection_key.as_ref(),
            "release:2026.05"
        );
        assert!(dashboard.collections[0].due_now);
        assert_eq!(dashboard.collections[0].health.total, 2);
        assert_eq!(
            dashboard.collections[1].collection_key.as_ref(),
            "release:2026.06"
        );
        assert_eq!(dashboard.collections[1].health.total, 0);
        assert_eq!(board.collections[0].health.total, 2);
        assert!(board.collections[0].source.is_none());
    }
}
