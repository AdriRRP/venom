use crate::{
    ArtifactRef, CollectionScopedArtifact, FindingDecision, FindingGovernanceState, FindingRef,
    PackageCoordinate, ProviderScanReport, ReportedFinding, RiskAcceptance, Severity, Suppression,
};
use std::collections::BTreeMap;

pub const DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT: usize = 50;
pub const MAX_ACTIVE_FINDINGS_PAGE_LIMIT: usize = 200;

/// Rebuildable operator-facing view of currently active findings.
///
/// This read model is intentionally narrow: it stores one compact operator
/// projection for each active canonical finding, not the full provider-facing
/// payload. The source of truth remains the durable history that can replay
/// these snapshots.
#[derive(Debug, Clone, Default)]
pub struct FindingReadModel {
    active: BTreeMap<TrackedArtifactKey, Vec<ActiveFindingRecord>>,
    decisions: BTreeMap<FindingRef, FindingDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFindingsQuery {
    pub component_key: Box<str>,
    pub artifact: ArtifactRef,
    pub min_severity: Option<Severity>,
    pub package_name: Option<Box<str>>,
    pub offset: usize,
    pub limit: usize,
}

impl ActiveFindingsQuery {
    #[must_use]
    pub fn new(component_key: impl Into<Box<str>>, artifact: ArtifactRef) -> Self {
        Self {
            component_key: component_key.into(),
            artifact,
            min_severity: None,
            package_name: None,
            offset: 0,
            limit: DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT,
        }
    }

    #[must_use]
    pub const fn with_min_severity(mut self, min_severity: Severity) -> Self {
        self.min_severity = Some(min_severity);
        self
    }

    #[must_use]
    pub fn with_package_name(mut self, package_name: impl Into<Box<str>>) -> Self {
        self.package_name = Some(package_name.into());
        self
    }

    #[must_use]
    pub const fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFindingsPage {
    pub total: usize,
    pub returned: usize,
    pub offset: usize,
    pub limit: usize,
    pub findings: Vec<ActiveFindingProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedActiveFindingsQuery {
    pub min_severity: Option<Severity>,
    pub package_name: Option<Box<str>>,
    pub offset: usize,
    pub limit: usize,
}

impl ScopedActiveFindingsQuery {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min_severity: None,
            package_name: None,
            offset: 0,
            limit: DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT,
        }
    }

    #[must_use]
    pub const fn with_min_severity(mut self, min_severity: Severity) -> Self {
        self.min_severity = Some(min_severity);
        self
    }

    #[must_use]
    pub fn with_package_name(mut self, package_name: impl Into<Box<str>>) -> Self {
        self.package_name = Some(package_name.into());
        self
    }

    #[must_use]
    pub const fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

impl Default for ScopedActiveFindingsQuery {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedActiveFindingsPage {
    pub total: usize,
    pub returned: usize,
    pub offset: usize,
    pub limit: usize,
    pub findings: Vec<ScopedActiveFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFindingProjection {
    pub finding: FindingRef,
    pub severity: Severity,
    pub governance_state: FindingGovernanceState,
    pub governance_reason: Option<Box<str>>,
    pub governance_until_unix_ms: Option<u64>,
}

pub type ScopedActiveFinding = ActiveFindingProjection;

impl FindingReadModel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one full provider snapshot to the active findings projection.
    pub fn record_scan_report(&mut self, report: &ProviderScanReport) {
        let key = TrackedArtifactKey::new(report.component_key.clone(), report.artifact.clone());
        self.active
            .insert(key, canonicalize_findings(&report.findings));
    }

    /// Restore one provider snapshot during replay from already canonical findings.
    pub(crate) fn replay_canonical_scan_report(
        &mut self,
        component_key: Box<str>,
        artifact: ArtifactRef,
        canonical_findings: &[ReportedFinding],
    ) {
        let key = TrackedArtifactKey::new(component_key, artifact);
        self.active
            .insert(key, canonicalize_findings(canonical_findings));
    }

    pub fn accept_risk(&mut self, finding: FindingRef, acceptance: RiskAcceptance) {
        self.decisions
            .insert(finding, FindingDecision::RiskAccepted(acceptance));
    }

    pub fn replay_risk_acceptance(&mut self, finding: FindingRef, acceptance: RiskAcceptance) {
        self.accept_risk(finding, acceptance);
    }

    pub fn suppress(&mut self, finding: FindingRef, suppression: Suppression) {
        self.decisions
            .insert(finding, FindingDecision::Suppressed(suppression));
    }

    pub fn replay_suppression(&mut self, finding: FindingRef, suppression: Suppression) {
        self.suppress(finding, suppression);
    }

    #[must_use]
    pub fn active_finding_count(&self, component_key: &str, artifact: &ArtifactRef) -> usize {
        self.active
            .get(&TrackedArtifactKey::new(
                component_key.into(),
                artifact.clone(),
            ))
            .map_or(0, Vec::len)
    }

    #[must_use]
    pub fn has_active_vulnerability(
        &self,
        component_key: &str,
        artifact: &ArtifactRef,
        vulnerability_id: &str,
    ) -> bool {
        self.active
            .get(&TrackedArtifactKey::new(
                component_key.into(),
                artifact.clone(),
            ))
            .is_some_and(|findings| {
                findings
                    .iter()
                    .any(|finding| finding.vulnerability_id.as_ref() == vulnerability_id)
            })
    }

    #[must_use]
    pub fn has_active_finding(&self, finding: &FindingRef) -> bool {
        self.active
            .get(&TrackedArtifactKey::new(
                finding.component_key.clone(),
                finding.artifact.clone(),
            ))
            .is_some_and(|findings| findings.iter().any(|candidate| candidate.matches(finding)))
    }

    #[must_use]
    pub fn query_active_findings(&self, query: &ActiveFindingsQuery) -> ActiveFindingsPage {
        let offset = query.offset;
        let limit = normalize_page_limit(query.limit);
        let mut filtered = self
            .active
            .get(&TrackedArtifactKey::new(
                query.component_key.clone(),
                query.artifact.clone(),
            ))
            .into_iter()
            .flatten()
            .filter(|finding| {
                query
                    .min_severity
                    .is_none_or(|min| severity_rank(finding.severity) >= severity_rank(min))
            })
            .filter(|finding| {
                query
                    .package_name
                    .as_deref()
                    .is_none_or(|package_name| finding.package_name.as_ref() == package_name)
            })
            .collect::<Vec<_>>();
        filtered.sort_unstable_by_key(|finding| {
            (
                std::cmp::Reverse(severity_rank(finding.severity)),
                finding_dedup_key(finding),
            )
        });

        let total = filtered.len();
        let page = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|finding| {
                self.project_active_finding(
                    query.component_key.clone(),
                    query.artifact.clone(),
                    finding,
                )
            })
            .collect::<Vec<_>>();

        ActiveFindingsPage {
            total,
            returned: page.len(),
            offset,
            limit,
            findings: page,
        }
    }

    #[must_use]
    pub fn query_scoped_active_findings(
        &self,
        scope: &[CollectionScopedArtifact],
        query: &ScopedActiveFindingsQuery,
    ) -> ScopedActiveFindingsPage {
        let offset = query.offset;
        let limit = normalize_page_limit(query.limit);
        let mut filtered = scope
            .iter()
            .flat_map(|scope_item| {
                self.active
                    .get(&TrackedArtifactKey::new(
                        scope_item.component_key.clone(),
                        scope_item.artifact.clone(),
                    ))
                    .into_iter()
                    .flatten()
                    .map(move |finding| (scope_item, finding))
            })
            .filter(|(_, finding)| {
                query
                    .min_severity
                    .is_none_or(|min| severity_rank(finding.severity) >= severity_rank(min))
            })
            .filter(|(_, finding)| {
                query
                    .package_name
                    .as_deref()
                    .is_none_or(|package_name| finding.package_name.as_ref() == package_name)
            })
            .collect::<Vec<_>>();
        filtered.sort_unstable_by_key(|(scope_item, finding)| {
            (
                std::cmp::Reverse(severity_rank(finding.severity)),
                scope_item.component_key.as_ref(),
                scope_item.artifact.kind,
                scope_item.artifact.identity.as_ref(),
                finding_dedup_key(finding),
            )
        });

        let total = filtered.len();
        let page = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(scope_item, finding)| {
                self.project_active_finding(
                    scope_item.component_key.clone(),
                    scope_item.artifact.clone(),
                    finding,
                )
            })
            .collect::<Vec<_>>();

        ScopedActiveFindingsPage {
            total,
            returned: page.len(),
            offset,
            limit,
            findings: page,
        }
    }

    fn project_active_finding(
        &self,
        component_key: Box<str>,
        artifact: ArtifactRef,
        finding: &ActiveFindingRecord,
    ) -> ActiveFindingProjection {
        let finding_ref = finding.finding_ref(component_key, artifact);
        let (governance_state, governance_reason, governance_until_unix_ms) = self
            .decisions
            .get(&finding_ref)
            .map_or((FindingGovernanceState::Open, None, None), |decision| {
                (
                    decision.state(),
                    decision.reason().map(Into::into),
                    decision.until_unix_ms(),
                )
            });

        ActiveFindingProjection {
            finding: finding_ref,
            severity: finding.severity,
            governance_state,
            governance_reason,
            governance_until_unix_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TrackedArtifactKey {
    component_key: Box<str>,
    artifact: ArtifactRef,
}

impl TrackedArtifactKey {
    const fn new(component_key: Box<str>, artifact: ArtifactRef) -> Self {
        Self {
            component_key,
            artifact,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveFindingRecord {
    vulnerability_id: Box<str>,
    package_name: Box<str>,
    package_version: Box<str>,
    package_purl: Option<Box<str>>,
    severity: Severity,
}

impl ActiveFindingRecord {
    fn finding_ref(&self, component_key: Box<str>, artifact: ArtifactRef) -> FindingRef {
        FindingRef::new(
            component_key,
            artifact,
            self.vulnerability_id.clone(),
            PackageCoordinate {
                name: self.package_name.clone(),
                version: self.package_version.clone(),
                purl: self.package_purl.clone(),
            },
        )
    }

    fn to_reported_finding(&self) -> ReportedFinding {
        ReportedFinding::new(
            self.vulnerability_id.clone(),
            PackageCoordinate {
                name: self.package_name.clone(),
                version: self.package_version.clone(),
                purl: self.package_purl.clone(),
            },
        )
        .with_severity(self.severity)
    }

    fn matches(&self, finding: &FindingRef) -> bool {
        self.vulnerability_id == finding.vulnerability_id
            && self.package_name == finding.package.name
            && self.package_version == finding.package.version
            && self.package_purl == finding.package.purl
    }
}

impl From<&ReportedFinding> for ActiveFindingRecord {
    fn from(finding: &ReportedFinding) -> Self {
        Self {
            vulnerability_id: finding.vulnerability_id.clone(),
            package_name: finding.package.name.clone(),
            package_version: finding.package.version.clone(),
            package_purl: finding.package.purl.clone(),
            severity: finding.severity,
        }
    }
}

fn canonicalize_findings(findings: &[ReportedFinding]) -> Vec<ActiveFindingRecord> {
    let mut canonical = findings
        .iter()
        .map(ActiveFindingRecord::from)
        .collect::<Vec<_>>();
    canonical.sort_unstable_by(finding_sort_key);
    canonical.dedup_by(|left, right| finding_dedup_key(left) == finding_dedup_key(right));
    canonical
}

pub(crate) fn canonicalize_reported_findings(findings: &[ReportedFinding]) -> Vec<ReportedFinding> {
    canonicalize_findings(findings)
        .into_iter()
        .map(|finding| finding.to_reported_finding())
        .collect()
}

fn finding_sort_key(left: &ActiveFindingRecord, right: &ActiveFindingRecord) -> std::cmp::Ordering {
    finding_dedup_key(left).cmp(&finding_dedup_key(right))
}

const fn severity_rank(value: Severity) -> u8 {
    match value {
        Severity::Unknown => 0,
        Severity::None => 1,
        Severity::Low => 2,
        Severity::Medium => 3,
        Severity::High => 4,
        Severity::Critical => 5,
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

fn finding_dedup_key(finding: &ActiveFindingRecord) -> FindingDedupKey<'_> {
    FindingDedupKey {
        vulnerability_id: finding.vulnerability_id.as_ref(),
        package_name: finding.package_name.as_ref(),
        package_version: finding.package_version.as_ref(),
        package_purl: finding.package_purl.as_deref(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FindingDedupKey<'a> {
    vulnerability_id: &'a str,
    package_name: &'a str,
    package_version: &'a str,
    package_purl: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::{
        ActiveFindingsQuery, DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT, FindingReadModel,
        MAX_ACTIVE_FINDINGS_PAGE_LIMIT, ScopedActiveFindingsQuery,
    };
    use crate::{
        ArtifactKind, ArtifactRef, CollectionScopedArtifact, EvidenceFreshness, FindingRef,
        PackageCoordinate, ProviderScanReport, ReportedFinding, RiskAcceptance, Severity,
        Suppression,
    };
    use std::time::SystemTime;

    fn artifact() -> ArtifactRef {
        ArtifactRef::new(
            ArtifactKind::ContainerImage,
            "registry.example/payments@sha256:111",
        )
    }

    fn report(findings: Vec<ReportedFinding>) -> ProviderScanReport {
        ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            findings,
        )
    }

    #[test]
    fn projection_tracks_active_findings_for_one_artifact() {
        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]));

        assert_eq!(
            read_model.active_finding_count("component:payments-api", &artifact()),
            1
        );
        assert!(read_model.has_active_vulnerability(
            "component:payments-api",
            &artifact(),
            "CVE-2026-0001"
        ));
    }

    #[test]
    fn empty_snapshot_withdraws_active_projection() {
        let mut read_model = FindingReadModel::new();
        let active = report(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]);
        let empty = report(Vec::new());

        read_model.record_scan_report(&active);
        read_model.record_scan_report(&empty);

        assert_eq!(
            read_model.active_finding_count("component:payments-api", &artifact()),
            0
        );
        assert!(!read_model.has_active_vulnerability(
            "component:payments-api",
            &artifact(),
            "CVE-2026-0001"
        ));
    }

    #[test]
    fn query_filters_by_min_severity_and_pages_stably() {
        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report(vec![
            ReportedFinding::new("CVE-2026-0002", PackageCoordinate::new("busybox", "1.36.0"))
                .with_severity(Severity::Low),
            ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
                .with_severity(Severity::Critical),
            ReportedFinding::new("CVE-2026-0003", PackageCoordinate::new("glibc", "2.40"))
                .with_severity(Severity::High),
        ]));

        let query = ActiveFindingsQuery::new("component:payments-api", artifact())
            .with_min_severity(Severity::High)
            .with_offset(0)
            .with_limit(1);
        let page = read_model.query_active_findings(&query);

        assert_eq!(page.total, 2);
        assert_eq!(page.returned, 1);
        assert_eq!(page.limit, 1);
        assert_eq!(
            page.findings[0].finding.vulnerability_id.as_ref(),
            "CVE-2026-0001"
        );
    }

    #[test]
    fn query_normalizes_zero_and_large_page_limits() {
        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]));

        let default_page = read_model.query_active_findings(
            &ActiveFindingsQuery::new("component:payments-api", artifact()).with_limit(0),
        );
        assert_eq!(default_page.limit, DEFAULT_ACTIVE_FINDINGS_PAGE_LIMIT);

        let capped_page = read_model.query_active_findings(
            &ActiveFindingsQuery::new("component:payments-api", artifact())
                .with_limit(MAX_ACTIVE_FINDINGS_PAGE_LIMIT + 100),
        );
        assert_eq!(capped_page.limit, MAX_ACTIVE_FINDINGS_PAGE_LIMIT);
    }

    #[test]
    fn scoped_query_aggregates_findings_across_multiple_collection_members() {
        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report(vec![
            ReportedFinding::new("CVE-2026-0002", PackageCoordinate::new("busybox", "1.36.0"))
                .with_severity(Severity::Low),
            ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
                .with_severity(Severity::Critical),
        ]));

        let billing_artifact = ArtifactRef::new(
            ArtifactKind::ContainerImage,
            "registry.example/billing@sha256:222",
        );
        let billing_report = ProviderScanReport::new(
            "fixture-provider",
            "component:billing-api",
            billing_artifact.clone(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![
                ReportedFinding::new("CVE-2026-0003", PackageCoordinate::new("nghttp2", "1.61"))
                    .with_severity(Severity::High),
            ],
        );
        read_model.record_scan_report(&billing_report);

        let scope = vec![
            CollectionScopedArtifact {
                component_key: "component:payments-api".into(),
                artifact: artifact(),
            },
            CollectionScopedArtifact {
                component_key: "component:billing-api".into(),
                artifact: billing_artifact,
            },
        ];

        let page = read_model.query_scoped_active_findings(
            &scope,
            &ScopedActiveFindingsQuery::new()
                .with_min_severity(Severity::High)
                .with_limit(10),
        );

        assert_eq!(page.total, 2);
        assert_eq!(page.returned, 2);
        assert_eq!(
            page.findings[0].finding.component_key.as_ref(),
            "component:payments-api"
        );
        assert_eq!(
            page.findings[0].finding.vulnerability_id.as_ref(),
            "CVE-2026-0001"
        );
        assert_eq!(
            page.findings[1].finding.component_key.as_ref(),
            "component:billing-api"
        );
        assert_eq!(
            page.findings[1].finding.vulnerability_id.as_ref(),
            "CVE-2026-0003"
        );
    }

    #[test]
    fn accepted_risk_is_projected_on_active_findings() {
        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report(vec![
            ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
                .with_severity(Severity::High),
        ]));

        read_model.accept_risk(
            FindingRef::new(
                "component:payments-api",
                artifact(),
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            ),
            RiskAcceptance::new("Compensating control in place").until_unix_ms(1_760_000_000_000),
        );

        let page = read_model.query_active_findings(&ActiveFindingsQuery::new(
            "component:payments-api",
            artifact(),
        ));

        assert_eq!(page.findings[0].governance_state.as_str(), "risk-accepted");
        assert_eq!(
            page.findings[0].governance_reason.as_deref(),
            Some("Compensating control in place")
        );
        assert_eq!(
            page.findings[0].governance_until_unix_ms,
            Some(1_760_000_000_000)
        );
    }

    #[test]
    fn suppression_is_projected_on_active_findings() {
        let mut read_model = FindingReadModel::new();
        read_model.record_scan_report(&report(vec![
            ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
                .with_severity(Severity::High),
        ]));

        read_model.suppress(
            FindingRef::new(
                "component:payments-api",
                artifact(),
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            ),
            Suppression::new("Known upstream false alarm"),
        );

        let page = read_model.query_active_findings(&ActiveFindingsQuery::new(
            "component:payments-api",
            artifact(),
        ));

        assert_eq!(page.findings[0].governance_state.as_str(), "suppressed");
        assert_eq!(
            page.findings[0].governance_reason.as_deref(),
            Some("Known upstream false alarm")
        );
        assert_eq!(page.findings[0].governance_until_unix_ms, None);
    }
}
