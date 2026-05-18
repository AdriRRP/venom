use crate::findings::{ActiveFindingProjection, Severity};
use crate::inventory::{ComponentInventory, ManagedContextProfile};
use std::collections::BTreeMap;

/// Deterministic operator-facing risk level after execution context is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextualRiskLevel {
    Unknown,
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl ContextualRiskLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Active-finding projection enriched with deterministic contextual meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualActiveFindingProjection {
    pub finding: crate::FindingRef,
    pub severity: Severity,
    pub contextual_risk: ContextualRiskLevel,
    pub context_profile_key: Option<Box<str>>,
    pub context_profile_name: Option<Box<str>>,
    pub governance_state: crate::FindingGovernanceState,
    pub governance_reason: Option<Box<str>>,
    pub governance_until_unix_ms: Option<u64>,
}

impl ContextualActiveFindingProjection {
    #[must_use]
    pub fn from_active_finding(
        finding: ActiveFindingProjection,
        context_profile: Option<ManagedContextProfile>,
    ) -> Self {
        let contextual_risk = contextual_risk_level(finding.severity, context_profile.as_ref());
        Self {
            finding: finding.finding,
            severity: finding.severity,
            contextual_risk,
            context_profile_key: context_profile
                .as_ref()
                .map(|profile| profile.profile_key.clone()),
            context_profile_name: context_profile.map(|profile| profile.name),
            governance_state: finding.governance_state,
            governance_reason: finding.governance_reason,
            governance_until_unix_ms: finding.governance_until_unix_ms,
        }
    }
}

#[must_use]
pub fn contextualize_active_findings(
    inventory: &ComponentInventory,
    findings: Vec<ActiveFindingProjection>,
) -> Vec<ContextualActiveFindingProjection> {
    let mut context_cache: BTreeMap<Box<str>, Option<ManagedContextProfile>> = BTreeMap::new();
    findings
        .into_iter()
        .map(|finding| {
            let component_key = finding.finding.component_key.clone();
            let context_profile = context_cache
                .entry(component_key.clone())
                .or_insert_with(|| {
                    inventory.managed_component_context_profile(component_key.as_ref())
                })
                .clone();
            ContextualActiveFindingProjection::from_active_finding(finding, context_profile)
        })
        .collect()
}

#[must_use]
pub fn contextual_risk_level(
    severity: Severity,
    context_profile: Option<&ManagedContextProfile>,
) -> ContextualRiskLevel {
    let Some(context_profile) = context_profile else {
        return match severity {
            Severity::Unknown => ContextualRiskLevel::Unknown,
            Severity::None => ContextualRiskLevel::None,
            Severity::Low => ContextualRiskLevel::Low,
            Severity::Medium => ContextualRiskLevel::Medium,
            Severity::High => ContextualRiskLevel::High,
            Severity::Critical => ContextualRiskLevel::Critical,
        };
    };

    let context_pressure = u8::from(context_profile.internet_exposed)
        + u8::from(context_profile.production)
        + u8::from(context_profile.mission_critical);

    match (severity, context_pressure) {
        (Severity::Unknown, _) => ContextualRiskLevel::Unknown,
        (Severity::None, _) => ContextualRiskLevel::None,
        (Severity::Critical, _) => ContextualRiskLevel::Critical,
        (Severity::High, 0) => ContextualRiskLevel::High,
        (Severity::High, _) => ContextualRiskLevel::Critical,
        (Severity::Medium, 0) => ContextualRiskLevel::Medium,
        (Severity::Medium, 1) => ContextualRiskLevel::High,
        (Severity::Medium, _) => ContextualRiskLevel::Critical,
        (Severity::Low, 0) => ContextualRiskLevel::Low,
        (Severity::Low, 1 | 2) => ContextualRiskLevel::Medium,
        (Severity::Low, _) => ContextualRiskLevel::High,
    }
}

#[cfg(test)]
mod tests {
    use super::{ContextualActiveFindingProjection, ContextualRiskLevel, contextual_risk_level};
    use crate::{
        ActiveFindingProjection, ArtifactKind, ArtifactRef, FindingGovernanceState, FindingRef,
        ManagedContextProfile, PackageCoordinate, Severity,
    };

    #[test]
    fn medium_finding_in_internet_production_context_becomes_critical() {
        let profile = ManagedContextProfile {
            profile_key: "context:internet-prod".into(),
            name: "Internet Production".into(),
            internet_exposed: true,
            production: true,
            mission_critical: true,
        };

        assert_eq!(
            contextual_risk_level(Severity::Medium, Some(&profile)),
            ContextualRiskLevel::Critical
        );
    }

    #[test]
    fn high_finding_without_context_stays_high() {
        assert_eq!(
            contextual_risk_level(Severity::High, None),
            ContextualRiskLevel::High
        );
    }

    #[test]
    fn contextual_projection_keeps_context_profile_identity() {
        let projection = ActiveFindingProjection {
            finding: FindingRef::new(
                "component:payments-api",
                ArtifactRef::new(
                    ArtifactKind::ContainerImage,
                    "registry.example/payments@sha256:111",
                ),
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            ),
            severity: Severity::Medium,
            governance_state: FindingGovernanceState::Open,
            governance_reason: None,
            governance_until_unix_ms: None,
        };
        let profile = ManagedContextProfile {
            profile_key: "context:internet-prod".into(),
            name: "Internet Production".into(),
            internet_exposed: true,
            production: true,
            mission_critical: true,
        };

        let contextual =
            ContextualActiveFindingProjection::from_active_finding(projection, Some(profile));

        assert_eq!(contextual.contextual_risk, ContextualRiskLevel::Critical);
        assert_eq!(
            contextual.context_profile_key.as_deref(),
            Some("context:internet-prod")
        );
        assert_eq!(
            contextual.context_profile_name.as_deref(),
            Some("Internet Production")
        );
    }
}
