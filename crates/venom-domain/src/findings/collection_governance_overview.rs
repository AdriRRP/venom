use crate::findings::{
    CollectionHealthSummary, ContextualRiskLevel, FindingGovernanceState, FindingReadModel,
    ScopedActiveFindingsPage, ScopedActiveFindingsQuery, contextual_risk_level,
    summarize_collection_health,
};
use crate::inventory::ComponentInventory;
use std::collections::BTreeMap;

/// One operator-facing release-scoped findings workbench.
///
/// The overview keeps two read-side concerns together:
/// one compact collection summary for the whole release scope, and one filtered
/// page of active findings for the operator's current query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionGovernanceOverview {
    pub health: CollectionHealthSummary,
    pub bulk_governance: BulkGovernanceCohortSummary,
    pub page: ScopedActiveFindingsPage,
}

/// One filtered open cohort summary that drives one release-scoped bulk
/// governance decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BulkGovernanceCohortSummary {
    pub targeted: usize,
    pub critical_risk: usize,
    pub high_risk: usize,
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
        bulk_governance: summarize_bulk_governance_cohort(inventory, read_model, &scope, query),
        page: read_model.query_scoped_active_findings(&scope, query),
    })
}

fn summarize_bulk_governance_cohort(
    inventory: &ComponentInventory,
    read_model: &FindingReadModel,
    scope: &[crate::CollectionScopedArtifact],
    query: &ScopedActiveFindingsQuery,
) -> BulkGovernanceCohortSummary {
    let cohort_query =
        ScopedActiveFindingsQuery::new().with_governance_state(FindingGovernanceState::Open);
    let cohort_query = if let Some(min_severity) = query.min_severity {
        cohort_query.with_min_severity(min_severity)
    } else {
        cohort_query
    };
    let cohort_query = if let Some(package_name) = query.package_name.as_deref() {
        cohort_query.with_package_name(package_name)
    } else {
        cohort_query
    };

    let mut summary = BulkGovernanceCohortSummary::default();
    let mut context_profiles = BTreeMap::new();

    for finding in read_model.collect_scoped_active_findings(scope, &cohort_query) {
        summary.targeted += 1;
        let context_profile = context_profiles
            .entry(finding.finding.component_key.clone())
            .or_insert_with(|| {
                inventory.managed_component_context_profile(finding.finding.component_key.as_ref())
            });
        match contextual_risk_level(finding.severity, context_profile.as_ref()) {
            ContextualRiskLevel::Critical => summary.critical_risk += 1,
            ContextualRiskLevel::High => summary.high_risk += 1,
            ContextualRiskLevel::Unknown
            | ContextualRiskLevel::None
            | ContextualRiskLevel::Low
            | ContextualRiskLevel::Medium => {}
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::{BulkGovernanceCohortSummary, query_collection_governance_overview};
    use crate::findings::{
        FindingGovernanceState, FindingReadModel, FindingRef, PackageCoordinate,
        ProviderScanReport, ReportedFinding, ScopedActiveFindingsQuery, Severity, Suppression,
    };
    use crate::inventory::{
        CollectionRegistration, ComponentInventory, ComponentRegistration,
        ContextProfileRegistration,
    };
    use crate::{ArtifactKind, ArtifactRef, EvidenceFreshness};

    #[test]
    fn collection_governance_overview_keeps_health_for_the_whole_scope() {
        let mut inventory = ComponentInventory::default();
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
        assert_eq!(overview.health.high_risk, 1);
        assert_eq!(
            overview.bulk_governance,
            BulkGovernanceCohortSummary {
                targeted: 1,
                critical_risk: 1,
                high_risk: 0,
            }
        );
        assert_eq!(overview.page.total, 1);
        assert_eq!(
            overview.page.findings[0].governance_state.as_str(),
            "suppressed"
        );
    }
}
