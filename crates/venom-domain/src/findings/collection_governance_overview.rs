use crate::findings::{
    CollectionHealthSummary, FindingReadModel, ScopedActiveFindingsPage, ScopedActiveFindingsQuery,
    summarize_collection_health,
};
use crate::inventory::ComponentInventory;

/// One operator-facing release-scoped findings workbench.
///
/// The overview keeps two read-side concerns together:
/// one compact collection summary for the whole release scope, and one filtered
/// page of active findings for the operator's current query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionGovernanceOverview {
    pub health: CollectionHealthSummary,
    pub page: ScopedActiveFindingsPage,
}

#[must_use]
pub fn query_collection_governance_overview(
    inventory: &ComponentInventory,
    read_model: &FindingReadModel,
    collection_key: &str,
    query: &ScopedActiveFindingsQuery,
) -> Option<CollectionGovernanceOverview> {
    let scope = inventory.collection_scoped_artifacts(collection_key)?;
    Some(CollectionGovernanceOverview {
        health: summarize_collection_health(inventory, read_model, &scope),
        page: read_model.query_scoped_active_findings(&scope, query),
    })
}

#[cfg(test)]
mod tests {
    use super::query_collection_governance_overview;
    use crate::findings::{
        FindingGovernanceState, FindingReadModel, FindingRef, PackageCoordinate,
        ProviderScanReport, ReportedFinding, ScopedActiveFindingsQuery, Severity, Suppression,
    };
    use crate::inventory::{
        CollectionRegistration, ComponentInventory, ComponentRegistration,
        ContextProfileRegistration,
    };
    use crate::{ArtifactKind, ArtifactRef};

    #[test]
    fn collection_governance_overview_keeps_health_for_the_whole_scope() {
        let mut inventory = ComponentInventory::new();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let artifact = ArtifactRef::new(
            ArtifactKind::ContainerImage,
            "registry.example/payments@sha256:111",
        );
        let _ = inventory.bind_artifact("component:payments-api", artifact.clone());
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:payments-api");
        let _ = inventory.register_context_profile(ContextProfileRegistration::new(
            "context:internet-prod",
            "Internet Production",
            true,
            true,
            true,
        ));
        let _ = inventory.assign_context_profile("component:payments-api", "context:internet-prod");

        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact.clone(),
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
                artifact,
                "CVE-2026-0002",
                PackageCoordinate::new("busybox", "1.36.1"),
            ),
            Suppression::new("Known upstream false alarm"),
        );

        let overview = query_collection_governance_overview(
            &inventory,
            &read_model,
            "release:2026.05",
            &ScopedActiveFindingsQuery::new()
                .with_governance_state(FindingGovernanceState::Suppressed),
        )
        .expect("collection overview should exist");

        assert_eq!(overview.health.total, 2);
        assert_eq!(overview.health.open, 1);
        assert_eq!(overview.health.suppressed, 1);
        assert_eq!(overview.health.risk_accepted, 0);
        assert_eq!(overview.health.critical_risk, 1);
        assert_eq!(overview.health.high_risk, 0);
        assert_eq!(overview.page.total, 1);
        assert_eq!(
            overview.page.findings[0].governance_state.as_str(),
            "suppressed"
        );
    }
}
