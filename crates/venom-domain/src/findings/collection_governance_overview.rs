use crate::findings::{
    CollectionHealthSummary, ContextualRiskLevel, DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT,
    FindingGovernanceState, FindingReadModel, MAX_ACTIVE_FINDINGS_PAGE_LIMIT, ScopedActiveFinding,
    ScopedActiveFindingsPage, ScopedActiveFindingsQuery, contextual_risk_level,
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

#[derive(Default)]
struct CollectionGovernanceAccumulator {
    health: CollectionHealthSummary,
    bulk_governance: BulkGovernanceCohortSummary,
    page: BoundedScopedFindingsPage,
}

#[derive(Debug, Default)]
struct BoundedScopedFindingsPage {
    total: usize,
    cap: usize,
    findings: Vec<ScopedActiveFinding>,
}

#[must_use]
pub fn query_collection_governance_overview(
    inventory: &ComponentInventory,
    read_model: &FindingReadModel,
    collection_key: &str,
    query: &ScopedActiveFindingsQuery,
) -> Option<CollectionGovernanceOverview> {
    let scope = inventory.collection_scoped_artifacts(collection_key)?;
    let mut accumulator = CollectionGovernanceAccumulator::new(query);
    let governance_state = query
        .governance_state
        .unwrap_or(FindingGovernanceState::Open);
    let mut context_profiles = BTreeMap::new();
    read_model.visit_scoped_active_findings(&scope, |finding| {
        let context_profile = context_profiles
            .entry(finding.finding.component_key.clone())
            .or_insert_with(|| {
                inventory.managed_component_effective_context_in_collection(
                    collection_key,
                    finding.finding.component_key.as_ref(),
                )
            });
        let risk = contextual_risk_level(
            finding.severity,
            context_profile.as_ref().map(|context| &context.values),
        );
        accumulator.observe(finding, query, governance_state, risk);
    });

    Some(CollectionGovernanceOverview {
        health: accumulator.health,
        bulk_governance: accumulator.bulk_governance,
        page: accumulator.page.into_page(query),
    })
}

impl CollectionGovernanceAccumulator {
    fn new(query: &ScopedActiveFindingsQuery) -> Self {
        Self {
            page: BoundedScopedFindingsPage::new(query.offset, normalize_page_limit(query.limit)),
            ..Self::default()
        }
    }

    fn observe(
        &mut self,
        finding: ScopedActiveFinding,
        query: &ScopedActiveFindingsQuery,
        governance_state: FindingGovernanceState,
        risk: ContextualRiskLevel,
    ) {
        match finding.governance_state {
            FindingGovernanceState::Open => self.health.open += 1,
            FindingGovernanceState::RiskAccepted => self.health.risk_accepted += 1,
            FindingGovernanceState::Suppressed => self.health.suppressed += 1,
        }
        self.health.total += 1;
        match risk {
            ContextualRiskLevel::Critical => self.health.critical_risk += 1,
            ContextualRiskLevel::High => self.health.high_risk += 1,
            ContextualRiskLevel::Unknown
            | ContextualRiskLevel::None
            | ContextualRiskLevel::Low
            | ContextualRiskLevel::Medium => {}
        }

        if matches_bulk_governance_filter(&finding, governance_state, query) {
            self.bulk_governance.targeted += 1;
            match risk {
                ContextualRiskLevel::Critical => self.bulk_governance.critical_risk += 1,
                ContextualRiskLevel::High => self.bulk_governance.high_risk += 1,
                ContextualRiskLevel::Unknown
                | ContextualRiskLevel::None
                | ContextualRiskLevel::Low
                | ContextualRiskLevel::Medium => {}
            }
        }

        if matches_page_filter(&finding, query) {
            self.page.observe(finding);
        }
    }
}

impl BoundedScopedFindingsPage {
    const fn new(offset: usize, limit: usize) -> Self {
        Self {
            total: 0,
            cap: offset.saturating_add(limit),
            findings: Vec::new(),
        }
    }

    fn observe(&mut self, finding: ScopedActiveFinding) {
        self.total += 1;
        if self.cap == 0 {
            return;
        }

        let position = self
            .findings
            .binary_search_by(|candidate| compare_scoped_findings(candidate, &finding))
            .unwrap_or_else(|index| index);
        if position >= self.cap {
            return;
        }

        self.findings.insert(position, finding);
        if self.findings.len() > self.cap {
            self.findings.pop();
        }
    }

    fn into_page(self, query: &ScopedActiveFindingsQuery) -> ScopedActiveFindingsPage {
        let limit = normalize_page_limit(query.limit);
        let offset = query.offset;
        let findings = self
            .findings
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        ScopedActiveFindingsPage {
            total: self.total,
            returned: findings.len(),
            offset,
            limit,
            findings,
        }
    }
}

fn compare_scoped_findings(
    left: &ScopedActiveFinding,
    right: &ScopedActiveFinding,
) -> std::cmp::Ordering {
    std::cmp::Reverse(severity_rank(left.severity))
        .cmp(&std::cmp::Reverse(severity_rank(right.severity)))
        .then_with(|| {
            left.finding
                .component_key
                .as_ref()
                .cmp(right.finding.component_key.as_ref())
        })
        .then_with(|| left.finding.artifact.kind.cmp(&right.finding.artifact.kind))
        .then_with(|| {
            left.finding
                .artifact
                .identity
                .as_ref()
                .cmp(right.finding.artifact.identity.as_ref())
        })
        .then_with(|| {
            left.finding
                .vulnerability_id
                .as_ref()
                .cmp(right.finding.vulnerability_id.as_ref())
        })
        .then_with(|| {
            left.finding
                .package
                .name
                .as_ref()
                .cmp(right.finding.package.name.as_ref())
        })
        .then_with(|| {
            left.finding
                .package
                .version
                .as_ref()
                .cmp(right.finding.package.version.as_ref())
        })
        .then_with(|| {
            left.finding
                .package
                .purl
                .as_deref()
                .cmp(&right.finding.package.purl.as_deref())
        })
}

fn matches_bulk_governance_filter(
    finding: &ScopedActiveFinding,
    governance_state: FindingGovernanceState,
    query: &ScopedActiveFindingsQuery,
) -> bool {
    finding.governance_state == governance_state
        && query
            .min_severity
            .is_none_or(|min| severity_rank(finding.severity) >= severity_rank(min))
        && query
            .package_name
            .as_deref()
            .is_none_or(|package_name| finding.finding.package.name.as_ref() == package_name)
}

fn matches_page_filter(finding: &ScopedActiveFinding, query: &ScopedActiveFindingsQuery) -> bool {
    query
        .min_severity
        .is_none_or(|min| severity_rank(finding.severity) >= severity_rank(min))
        && query
            .governance_state
            .is_none_or(|governance_state| finding.governance_state == governance_state)
        && query
            .package_name
            .as_deref()
            .is_none_or(|package_name| finding.finding.package.name.as_ref() == package_name)
}

const fn severity_rank(value: crate::Severity) -> u8 {
    match value {
        crate::Severity::Unknown => 0,
        crate::Severity::None => 1,
        crate::Severity::Low => 2,
        crate::Severity::Medium => 3,
        crate::Severity::High => 4,
        crate::Severity::Critical => 5,
    }
}

const fn normalize_page_limit(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT
    } else if limit > MAX_ACTIVE_FINDINGS_PAGE_LIMIT {
        MAX_ACTIVE_FINDINGS_PAGE_LIMIT
    } else {
        limit
    }
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
                critical_risk: 0,
                high_risk: 1,
            }
        );
        assert_eq!(overview.page.total, 1);
        assert_eq!(
            overview.page.findings[0].governance_state.as_str(),
            "suppressed"
        );
    }

    #[test]
    fn collection_governance_overview_keeps_only_requested_page_window() {
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

        let findings = (0..6)
            .map(|index| {
                ReportedFinding::new(
                    format!("CVE-2026-000{index}"),
                    PackageCoordinate::new(format!("pkg-{index}"), "1.0.0"),
                )
                .with_severity(match index {
                    1 | 4 => Severity::Critical,
                    2 => Severity::High,
                    3 => Severity::Medium,
                    5 => Severity::Low,
                    _ => Severity::Unknown,
                })
            })
            .collect::<Vec<_>>();
        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact,
            std::time::SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            findings,
        );

        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report);

        let overview = query_collection_governance_overview(
            &inventory,
            &read_model,
            "release:2026.05",
            &ScopedActiveFindingsQuery::new()
                .with_offset(2)
                .with_limit(2),
        )
        .expect("collection overview should exist");

        assert_eq!(overview.page.total, 6);
        assert_eq!(overview.page.returned, 2);
        assert_eq!(overview.page.findings.len(), 2);
        assert_eq!(overview.page.findings[0].severity, Severity::High);
        assert_eq!(overview.page.findings[1].severity, Severity::High);
    }
}
