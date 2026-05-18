use crate::findings::{ContextualRiskLevel, contextual_risk_level};
use crate::{
    CollectionScopedArtifact, ComponentInventory, FindingGovernanceState, FindingReadModel,
};
use std::collections::BTreeMap;

/// Compact operator-facing summary of one closed collection health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CollectionHealthSummary {
    pub total: usize,
    pub open: usize,
    pub risk_accepted: usize,
    pub suppressed: usize,
    pub critical_risk: usize,
    pub high_risk: usize,
}

/// Derive one compact collection health summary from scoped active findings,
/// contextual risk, and governance state without widening the write model.
#[must_use]
pub fn summarize_collection_health(
    inventory: &ComponentInventory,
    read_model: &FindingReadModel,
    scope: &[CollectionScopedArtifact],
) -> CollectionHealthSummary {
    let mut summary = CollectionHealthSummary::default();
    let mut context_profiles = BTreeMap::new();

    read_model.visit_scoped_active_findings(scope, |finding| {
        summary.total += 1;

        match finding.governance_state {
            FindingGovernanceState::Open => summary.open += 1,
            FindingGovernanceState::RiskAccepted => summary.risk_accepted += 1,
            FindingGovernanceState::Suppressed => summary.suppressed += 1,
        }

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
    });

    summary
}

#[cfg(test)]
mod tests {
    use super::{CollectionHealthSummary, summarize_collection_health};
    use crate::{
        ArtifactKind, ArtifactRef, CollectionRegistration, ComponentInventory,
        ComponentRegistration, ContextProfileRegistration, FindingReadModel, FindingRef,
        PackageCoordinate, ProviderScanReport, ReportedFinding, Severity, Suppression,
    };

    fn inventory_with_collection() -> ComponentInventory {
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
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:payments-api");
        inventory
    }

    #[test]
    fn collection_health_counts_contextual_and_governed_findings() {
        let mut inventory = inventory_with_collection();
        let mut read_model = FindingReadModel::new();
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
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            std::time::SystemTime::UNIX_EPOCH,
            crate::EvidenceFreshness::Deterministic,
            vec![
                ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
                    .with_severity(Severity::Medium),
                ReportedFinding::new("CVE-2026-0002", PackageCoordinate::new("busybox", "1.36.0"))
                    .with_severity(Severity::Low),
            ],
        );
        read_model.record_scan_report(&report);
        read_model.suppress(
            FindingRef::new(
                "component:payments-api",
                ArtifactRef::new(
                    ArtifactKind::ContainerImage,
                    "registry.example/payments@sha256:111",
                ),
                "CVE-2026-0002",
                PackageCoordinate::new("busybox", "1.36.0"),
            ),
            Suppression::new("Known local suppression"),
        );
        let scope = inventory
            .collection_scoped_artifacts("release:2026.05")
            .expect("the collection scope must exist");
        let summary = summarize_collection_health(&inventory, &read_model, &scope);

        assert_eq!(
            summary,
            CollectionHealthSummary {
                total: 2,
                open: 1,
                risk_accepted: 0,
                suppressed: 1,
                critical_risk: 1,
                high_risk: 1,
            }
        );
    }
}
