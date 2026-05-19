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

/// Explicit suppression decision owned by VENOM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suppression {
    pub reason: Box<str>,
}

impl Suppression {
    #[must_use]
    pub fn new(reason: impl Into<Box<str>>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Durable governance decision for one finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingDecision {
    RiskAccepted(RiskAcceptance),
    Suppressed(Suppression),
}

impl FindingDecision {
    #[must_use]
    pub const fn state(&self) -> FindingGovernanceState {
        match self {
            Self::RiskAccepted(_) => FindingGovernanceState::RiskAccepted,
            Self::Suppressed(_) => FindingGovernanceState::Suppressed,
        }
    }

    #[must_use]
    pub const fn risk_acceptance(&self) -> Option<&RiskAcceptance> {
        match self {
            Self::RiskAccepted(acceptance) => Some(acceptance),
            Self::Suppressed(_) => None,
        }
    }

    #[must_use]
    pub const fn suppression(&self) -> Option<&Suppression> {
        match self {
            Self::RiskAccepted(_) => None,
            Self::Suppressed(suppression) => Some(suppression),
        }
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::RiskAccepted(acceptance) => Some(acceptance.reason.as_ref()),
            Self::Suppressed(suppression) => Some(suppression.reason.as_ref()),
        }
    }

    #[must_use]
    pub const fn until_unix_ms(&self) -> Option<u64> {
        match self {
            Self::RiskAccepted(acceptance) => acceptance.until_unix_ms,
            Self::Suppressed(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingGovernanceState {
    Open,
    RiskAccepted,
    Suppressed,
}

impl FindingGovernanceState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::RiskAccepted => "risk-accepted",
            Self::Suppressed => "suppressed",
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

    pub fn suppress(
        &mut self,
        finding: FindingRef,
        suppression: Suppression,
    ) -> SuppressFindingResult {
        let next = FindingDecision::Suppressed(suppression.clone());
        let change = if self.decision(&finding) == Some(&next) {
            SuppressFindingChange::Unchanged
        } else {
            self.decisions.insert(finding, next);
            SuppressFindingChange::Suppressed
        };

        SuppressFindingResult {
            change,
            suppression,
        }
    }

    pub fn replay_suppression(&mut self, finding: FindingRef, suppression: Suppression) {
        self.decisions
            .insert(finding, FindingDecision::Suppressed(suppression));
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkAcceptRiskResult {
    pub targeted: usize,
    pub accepted: usize,
    pub unchanged: usize,
    pub acceptance: RiskAcceptance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressFindingChange {
    Suppressed,
    Unchanged,
}

impl SuppressFindingChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Suppressed => "suppressed",
            Self::Unchanged => "unchanged",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuppressFindingResult {
    pub change: SuppressFindingChange,
    pub suppression: Suppression,
}

#[cfg(test)]
mod tests {
    use super::{
        AcceptRiskChange, FindingGovernance, FindingRef, RiskAcceptance, SuppressFindingChange,
        Suppression,
    };
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

    #[test]
    fn suppressing_one_finding_persists_one_decision() {
        let mut governance = FindingGovernance::new();

        let result = governance.suppress(finding(), Suppression::new("Known upstream false alarm"));

        assert_eq!(result.change, SuppressFindingChange::Suppressed);
        assert!(governance.decision(&finding()).is_some());
    }
}
