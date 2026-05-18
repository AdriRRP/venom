use crate::{ArtifactRef, PackageCoordinate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Canonical identity of one finding inside one managed component and artifact.
///
/// This value object is the stable anchor for durable governance decisions.
/// It is intentionally provider-agnostic and derived from the same canonical
/// fields used to track active findings over time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FindingRef {
    pub component_key: Box<str>,
    pub artifact: ArtifactRef,
    pub vulnerability_id: Box<str>,
    pub package: PackageCoordinate,
}

impl FindingRef {
    #[must_use]
    pub fn new(
        component_key: impl Into<Box<str>>,
        artifact: ArtifactRef,
        vulnerability_id: impl Into<Box<str>>,
        package: PackageCoordinate,
    ) -> Self {
        Self {
            component_key: component_key.into(),
            artifact,
            vulnerability_id: vulnerability_id.into(),
            package,
        }
    }
}

/// Explicit risk-acceptance decision owned by VENOM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskAcceptance {
    pub reason: Box<str>,
    pub until_unix_ms: Option<u64>,
}

impl RiskAcceptance {
    #[must_use]
    pub fn new(reason: impl Into<Box<str>>) -> Self {
        Self {
            reason: reason.into(),
            until_unix_ms: None,
        }
    }

    #[must_use]
    pub const fn until_unix_ms(mut self, until_unix_ms: u64) -> Self {
        self.until_unix_ms = Some(until_unix_ms);
        self
    }
}

/// Durable governance decision for one finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingDecision {
    RiskAccepted(RiskAcceptance),
}

impl FindingDecision {
    #[must_use]
    pub const fn state(&self) -> FindingGovernanceState {
        match self {
            Self::RiskAccepted(_) => FindingGovernanceState::RiskAccepted,
        }
    }

    #[must_use]
    pub const fn risk_acceptance(&self) -> Option<&RiskAcceptance> {
        match self {
            Self::RiskAccepted(acceptance) => Some(acceptance),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingGovernanceState {
    Open,
    RiskAccepted,
}

impl FindingGovernanceState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::RiskAccepted => "risk-accepted",
        }
    }
}

/// Write-side owner of durable governance decisions for findings.
#[derive(Debug, Clone, Default)]
pub struct FindingGovernance {
    decisions: BTreeMap<FindingRef, FindingDecision>,
}

impl FindingGovernance {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn decision(&self, finding: &FindingRef) -> Option<&FindingDecision> {
        self.decisions.get(finding)
    }

    pub fn accept_risk(
        &mut self,
        finding: FindingRef,
        acceptance: RiskAcceptance,
    ) -> AcceptRiskResult {
        let next = FindingDecision::RiskAccepted(acceptance.clone());
        let change = if self.decision(&finding) == Some(&next) {
            AcceptRiskChange::Unchanged
        } else {
            self.decisions.insert(finding, next);
            AcceptRiskChange::Accepted
        };

        AcceptRiskResult { change, acceptance }
    }

    pub fn replay_risk_acceptance(&mut self, finding: FindingRef, acceptance: RiskAcceptance) {
        self.decisions
            .insert(finding, FindingDecision::RiskAccepted(acceptance));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptRiskChange {
    Accepted,
    Unchanged,
}

impl AcceptRiskChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Unchanged => "unchanged",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptRiskResult {
    pub change: AcceptRiskChange,
    pub acceptance: RiskAcceptance,
}

#[cfg(test)]
mod tests {
    use super::{AcceptRiskChange, FindingGovernance, FindingRef, RiskAcceptance};
    use crate::{ArtifactKind, ArtifactRef, PackageCoordinate};

    fn finding() -> FindingRef {
        FindingRef::new(
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )
    }

    #[test]
    fn accepting_risk_persists_one_decision() {
        let mut governance = FindingGovernance::new();

        let result = governance.accept_risk(
            finding(),
            RiskAcceptance::new("Compensating control in place").until_unix_ms(1_760_000_000_000),
        );

        assert_eq!(result.change, AcceptRiskChange::Accepted);
        assert!(governance.decision(&finding()).is_some());
    }

    #[test]
    fn identical_risk_acceptance_is_idempotent() {
        let mut governance = FindingGovernance::new();
        let acceptance = RiskAcceptance::new("Compensating control in place");

        let first = governance.accept_risk(finding(), acceptance.clone());
        let second = governance.accept_risk(finding(), acceptance);

        assert_eq!(first.change, AcceptRiskChange::Accepted);
        assert_eq!(second.change, AcceptRiskChange::Unchanged);
    }
}
