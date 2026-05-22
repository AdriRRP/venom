use crate::findings::{ActiveFindingProjection, Severity};
use crate::inventory::{
    ComponentInventory, ContextProfileRef, ContextProfileValues, EffectiveContextProfile,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextualPosture {
    Unspecified,
    InternalRestricted,
    HardenedPrivate,
    ProductionService,
    CriticalInternal,
    PublicEdge,
    PublicCritical,
}

impl ContextualPosture {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::InternalRestricted => "internal-restricted",
            Self::HardenedPrivate => "hardened-private",
            Self::ProductionService => "production-service",
            Self::CriticalInternal => "critical-internal",
            Self::PublicEdge => "public-edge",
            Self::PublicCritical => "public-critical",
        }
    }
}

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
    pub contextual_posture: Box<str>,
    pub contextual_rule: Box<str>,
    pub contextual_factors: Vec<Box<str>>,
    pub contextual_factor_provenance: Vec<ContextualFactorProvenance>,
    pub context_profile_key: Option<Box<str>>,
    pub context_profile_name: Option<Box<str>>,
    pub component_context_profile: Option<ContextProfileRef>,
    pub collection_context_profile: Option<ContextProfileRef>,
    pub tag_context_profiles: Vec<ContextProfileRef>,
    pub governance_state: crate::FindingGovernanceState,
    pub governance_reason: Option<Box<str>>,
    pub governance_until_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualFactorProvenance {
    pub factor: Box<str>,
    pub source: Box<str>,
    pub identity: Box<str>,
}

impl ContextualActiveFindingProjection {
    #[must_use]
    pub fn from_active_finding(
        finding: ActiveFindingProjection,
        effective_context: Option<EffectiveContextProfile>,
    ) -> Self {
        let posture = effective_context
            .as_ref()
            .map_or(ContextualPosture::Unspecified, |context| {
                contextual_posture(context.values)
            });
        let contextual_risk = contextual_risk_level(
            finding.severity,
            effective_context.as_ref().map(|context| &context.values),
        );
        let contextual_rule = contextual_risk_rule(
            finding.severity,
            effective_context.as_ref().map(|context| &context.values),
        );
        let contextual_factors = effective_context
            .as_ref()
            .map_or_else(Vec::new, |context| contextual_factor_labels(context.values));
        let contextual_factor_provenance = effective_context
            .as_ref()
            .map_or_else(Vec::new, contextual_factor_provenance);
        let singular_profile = effective_context
            .as_ref()
            .and_then(EffectiveContextProfile::singular_profile)
            .cloned();
        Self {
            finding: finding.finding,
            severity: finding.severity,
            contextual_risk,
            contextual_posture: posture.as_str().into(),
            contextual_rule: contextual_rule.into(),
            contextual_factors,
            contextual_factor_provenance,
            context_profile_key: singular_profile
                .as_ref()
                .map(|profile| profile.profile_key.clone()),
            context_profile_name: singular_profile.map(|profile| profile.name),
            component_context_profile: effective_context
                .as_ref()
                .and_then(|context| context.component_profile.clone()),
            collection_context_profile: effective_context
                .as_ref()
                .and_then(|context| context.collection_profile.clone()),
            tag_context_profiles: effective_context
                .map(|context| context.tag_profiles)
                .unwrap_or_default(),
            governance_state: finding.governance_state,
            governance_reason: finding.governance_reason,
            governance_until_unix_ms: finding.governance_until_unix_ms,
        }
    }
}

fn contextual_factor_labels(values: ContextProfileValues) -> Vec<Box<str>> {
    let mut factors = Vec::new();
    push_context_factor(&mut factors, "internet-exposed", values.internet_exposed);
    push_context_factor(&mut factors, "production", values.production);
    push_context_factor(&mut factors, "mission-critical", values.mission_critical);
    push_context_factor(&mut factors, "vpn-restricted", values.vpn_restricted);
    push_context_factor(
        &mut factors,
        "non-privileged-user",
        values.non_privileged_user,
    );
    factors
}

fn contextual_factor_provenance(
    effective_context: &EffectiveContextProfile,
) -> Vec<ContextualFactorProvenance> {
    let values = effective_context.values;
    let sources = &effective_context.factor_sources;
    let mut factors = Vec::new();

    push_factor_provenance(
        &mut factors,
        "internet-exposed",
        values.internet_exposed,
        sources.internet_exposed.clone(),
    );
    push_factor_provenance(
        &mut factors,
        "production",
        values.production,
        sources.production.clone(),
    );
    push_factor_provenance(
        &mut factors,
        "mission-critical",
        values.mission_critical,
        sources.mission_critical.clone(),
    );
    push_factor_provenance(
        &mut factors,
        "vpn-restricted",
        values.vpn_restricted,
        sources.vpn_restricted.clone(),
    );
    push_factor_provenance(
        &mut factors,
        "non-privileged-user",
        values.non_privileged_user,
        sources.non_privileged_user.clone(),
    );

    factors
}

fn push_factor_provenance(
    factors: &mut Vec<ContextualFactorProvenance>,
    factor: &'static str,
    value: Option<bool>,
    source: Option<crate::ContextFactorOrigin>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(source) = source else {
        return;
    };
    factors.push(ContextualFactorProvenance {
        factor: format!("{factor}:{value}").into_boxed_str(),
        source: source.source.as_str().into(),
        identity: source.identity,
    });
}

fn push_context_factor(factors: &mut Vec<Box<str>>, name: &str, value: Option<bool>) {
    if let Some(value) = value {
        factors.push(format!("{name}:{value}").into_boxed_str());
    }
}

fn contextual_risk_rule(
    severity: Severity,
    context_profile: Option<&ContextProfileValues>,
) -> &'static str {
    let Some(context_profile) = context_profile else {
        return "raw-severity";
    };
    let posture = contextual_posture(*context_profile);

    match severity {
        Severity::Unknown | Severity::None | Severity::Critical => "raw-severity",
        Severity::High => match posture {
            ContextualPosture::HardenedPrivate => "mitigated-private-downgrade",
            ContextualPosture::PublicEdge
            | ContextualPosture::PublicCritical
            | ContextualPosture::CriticalInternal => "critical-surface-escalation",
            ContextualPosture::Unspecified
            | ContextualPosture::InternalRestricted
            | ContextualPosture::ProductionService => "high-baseline",
        },
        Severity::Medium => match posture {
            ContextualPosture::HardenedPrivate => "mitigated-private-downgrade",
            ContextualPosture::PublicCritical => "public-critical-escalation",
            ContextualPosture::PublicEdge
            | ContextualPosture::ProductionService
            | ContextualPosture::CriticalInternal => "service-surface-escalation",
            ContextualPosture::Unspecified | ContextualPosture::InternalRestricted => {
                "medium-baseline"
            }
        },
        Severity::Low => match posture {
            ContextualPosture::PublicCritical => "public-critical-escalation",
            ContextualPosture::PublicEdge
            | ContextualPosture::ProductionService
            | ContextualPosture::CriticalInternal => "service-surface-escalation",
            ContextualPosture::Unspecified
            | ContextualPosture::InternalRestricted
            | ContextualPosture::HardenedPrivate => "low-baseline",
        },
    }
}

#[must_use]
pub fn contextualize_active_findings(
    inventory: &ComponentInventory,
    findings: Vec<ActiveFindingProjection>,
) -> Vec<ContextualActiveFindingProjection> {
    let mut context_cache: BTreeMap<Box<str>, Option<EffectiveContextProfile>> = BTreeMap::new();
    findings
        .into_iter()
        .map(|finding| {
            let component_key = finding.finding.component_key.clone();
            let context_profile = context_cache
                .entry(component_key.clone())
                .or_insert_with(|| {
                    inventory.managed_component_effective_context(component_key.as_ref())
                })
                .clone();
            ContextualActiveFindingProjection::from_active_finding(finding, context_profile)
        })
        .collect()
}

#[must_use]
pub fn contextualize_collection_active_findings(
    inventory: &ComponentInventory,
    collection_key: &str,
    findings: Vec<ActiveFindingProjection>,
) -> Vec<ContextualActiveFindingProjection> {
    let mut context_cache: BTreeMap<Box<str>, Option<EffectiveContextProfile>> = BTreeMap::new();
    findings
        .into_iter()
        .map(|finding| {
            let component_key = finding.finding.component_key.clone();
            let context_profile = context_cache
                .entry(component_key.clone())
                .or_insert_with(|| {
                    inventory.managed_component_effective_context_in_collection(
                        collection_key,
                        component_key.as_ref(),
                    )
                })
                .clone();
            ContextualActiveFindingProjection::from_active_finding(finding, context_profile)
        })
        .collect()
}

#[must_use]
pub fn contextual_risk_level(
    severity: Severity,
    context_profile: Option<&ContextProfileValues>,
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
    let posture = contextual_posture(*context_profile);

    if severity == Severity::Unknown {
        return ContextualRiskLevel::Unknown;
    }
    if severity == Severity::None {
        return ContextualRiskLevel::None;
    }
    if severity == Severity::Critical {
        return ContextualRiskLevel::Critical;
    }
    if severity == Severity::High {
        return match posture {
            ContextualPosture::HardenedPrivate => ContextualRiskLevel::Medium,
            ContextualPosture::PublicEdge
            | ContextualPosture::PublicCritical
            | ContextualPosture::CriticalInternal => ContextualRiskLevel::Critical,
            ContextualPosture::Unspecified
            | ContextualPosture::InternalRestricted
            | ContextualPosture::ProductionService => ContextualRiskLevel::High,
        };
    }
    if severity == Severity::Medium {
        return match posture {
            ContextualPosture::HardenedPrivate => ContextualRiskLevel::Low,
            ContextualPosture::PublicCritical => ContextualRiskLevel::Critical,
            ContextualPosture::PublicEdge
            | ContextualPosture::ProductionService
            | ContextualPosture::CriticalInternal => ContextualRiskLevel::High,
            ContextualPosture::Unspecified | ContextualPosture::InternalRestricted => {
                ContextualRiskLevel::Medium
            }
        };
    }

    match posture {
        ContextualPosture::PublicCritical => ContextualRiskLevel::High,
        ContextualPosture::PublicEdge
        | ContextualPosture::ProductionService
        | ContextualPosture::CriticalInternal => ContextualRiskLevel::Medium,
        ContextualPosture::Unspecified
        | ContextualPosture::InternalRestricted
        | ContextualPosture::HardenedPrivate => ContextualRiskLevel::Low,
    }
}

fn contextual_posture(context_profile: ContextProfileValues) -> ContextualPosture {
    let internet_exposed = context_profile.internet_exposed.unwrap_or(false);
    let production = context_profile.production.unwrap_or(false);
    let mission_critical = context_profile.mission_critical.unwrap_or(false);
    let vpn_restricted = context_profile.vpn_restricted.unwrap_or(false);
    let non_privileged_user = context_profile.non_privileged_user.unwrap_or(false);

    if internet_exposed && (production || mission_critical) {
        return ContextualPosture::PublicCritical;
    }
    if internet_exposed {
        return ContextualPosture::PublicEdge;
    }
    if production && mission_critical {
        return ContextualPosture::CriticalInternal;
    }
    if production || mission_critical {
        return ContextualPosture::ProductionService;
    }
    if vpn_restricted && non_privileged_user {
        return ContextualPosture::HardenedPrivate;
    }
    if vpn_restricted || non_privileged_user {
        return ContextualPosture::InternalRestricted;
    }
    ContextualPosture::Unspecified
}

#[cfg(test)]
mod tests {
    use super::{ContextualActiveFindingProjection, ContextualRiskLevel, contextual_risk_level};
    use crate::{
        ActiveFindingProjection, ArtifactKind, ArtifactRef, ContextFactorOrigin,
        ContextFactorSource, ContextProfileRef, ContextProfileValues,
        EffectiveContextFactorSources, EffectiveContextProfile, FindingGovernanceState,
        FindingRef, ManagedContextProfile, PackageCoordinate, Severity,
    };

    #[test]
    fn medium_finding_in_internet_production_context_becomes_critical() {
        let profile = ManagedContextProfile {
            profile_key: "context:internet-prod".into(),
            name: "Internet Production".into(),
            internet_exposed: Some(true),
            production: Some(true),
            mission_critical: Some(true),
            vpn_restricted: None,
            non_privileged_user: None,
        };

        assert_eq!(
            contextual_risk_level(Severity::Medium, Some(&profile.values())),
            ContextualRiskLevel::Critical
        );
    }

    #[test]
    fn medium_finding_in_critical_internal_context_becomes_high() {
        let profile = ManagedContextProfile {
            profile_key: "context:internal-critical".into(),
            name: "Internal Critical".into(),
            internet_exposed: Some(false),
            production: Some(true),
            mission_critical: Some(true),
            vpn_restricted: Some(true),
            non_privileged_user: Some(true),
        };

        assert_eq!(
            contextual_risk_level(Severity::Medium, Some(&profile.values())),
            ContextualRiskLevel::High
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
    fn high_finding_in_vpn_restricted_non_privileged_context_becomes_medium() {
        let profile = ManagedContextProfile {
            profile_key: "context:corp-api-private".into(),
            name: "Corporate Private API".into(),
            internet_exposed: None,
            production: None,
            mission_critical: None,
            vpn_restricted: Some(true),
            non_privileged_user: Some(true),
        };

        assert_eq!(
            contextual_risk_level(Severity::High, Some(&profile.values())),
            ContextualRiskLevel::Medium
        );
    }

    #[test]
    fn low_finding_on_public_edge_becomes_medium() {
        let profile = ManagedContextProfile {
            profile_key: "context:public-edge".into(),
            name: "Public Edge".into(),
            internet_exposed: Some(true),
            production: Some(false),
            mission_critical: Some(false),
            vpn_restricted: None,
            non_privileged_user: None,
        };

        assert_eq!(
            contextual_risk_level(Severity::Low, Some(&profile.values())),
            ContextualRiskLevel::Medium
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
            internet_exposed: Some(true),
            production: Some(true),
            mission_critical: Some(true),
            vpn_restricted: None,
            non_privileged_user: None,
        };

        let contextual = ContextualActiveFindingProjection::from_active_finding(
            projection,
            Some(EffectiveContextProfile {
                values: profile.values(),
                factor_sources: EffectiveContextFactorSources {
                    internet_exposed: Some(ContextFactorOrigin::new(
                        ContextFactorSource::Component,
                        "context:internet-prod",
                    )),
                    production: Some(ContextFactorOrigin::new(
                        ContextFactorSource::Component,
                        "context:internet-prod",
                    )),
                    mission_critical: Some(ContextFactorOrigin::new(
                        ContextFactorSource::Component,
                        "context:internet-prod",
                    )),
                    vpn_restricted: None,
                    non_privileged_user: None,
                },
                component_profile: Some(profile.reference()),
                collection_profile: None,
                tag_profiles: Vec::new(),
            }),
        );

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

    #[test]
    fn contextual_projection_drops_singular_identity_for_composite_provenance() {
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

        let contextual = ContextualActiveFindingProjection::from_active_finding(
            projection,
            Some(EffectiveContextProfile {
                values: ContextProfileValues {
                    internet_exposed: Some(true),
                    production: Some(true),
                    mission_critical: Some(true),
                    vpn_restricted: None,
                    non_privileged_user: None,
                },
                factor_sources: EffectiveContextFactorSources {
                    internet_exposed: Some(ContextFactorOrigin::new(
                        ContextFactorSource::Component,
                        "context:payments-edge",
                    )),
                    production: Some(ContextFactorOrigin::new(
                        ContextFactorSource::Collection,
                        "context:corp-api-baseline",
                    )),
                    mission_critical: Some(ContextFactorOrigin::new(
                        ContextFactorSource::Collection,
                        "context:corp-api-baseline",
                    )),
                    vpn_restricted: None,
                    non_privileged_user: None,
                },
                component_profile: Some(ContextProfileRef {
                    profile_key: "context:payments-edge".into(),
                    name: "Payments Edge".into(),
                }),
                collection_profile: Some(ContextProfileRef {
                    profile_key: "context:corp-api-baseline".into(),
                    name: "Corporate API Baseline".into(),
                }),
                tag_profiles: Vec::new(),
            }),
        );

        assert!(contextual.context_profile_key.is_none());
        assert!(contextual.context_profile_name.is_none());
        assert_eq!(
            contextual
                .component_context_profile
                .as_ref()
                .map(|profile| profile.profile_key.as_ref()),
            Some("context:payments-edge")
        );
        assert_eq!(
            contextual
                .collection_context_profile
                .as_ref()
                .map(|profile| profile.profile_key.as_ref()),
            Some("context:corp-api-baseline")
        );
        assert_eq!(
            contextual
                .contextual_factor_provenance
                .iter()
                .find(|factor| factor.factor.as_ref() == "production:true")
                .map(|factor| (factor.source.as_ref(), factor.identity.as_ref())),
            Some(("collection", "context:corp-api-baseline"))
        );
    }
}
