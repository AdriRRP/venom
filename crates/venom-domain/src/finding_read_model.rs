use crate::{ArtifactRef, ProviderScanReport, ReportedFinding};
use std::collections::BTreeMap;

/// Rebuildable operator-facing view of currently active findings.
///
/// This read model is intentionally narrow: it stores only the active
/// canonical findings for each managed `(component, artifact)` pair. The
/// source of truth remains the durable history that can replay these snapshots.
#[derive(Debug, Clone, Default)]
pub struct FindingReadModel {
    active: BTreeMap<TrackedArtifactKey, Vec<ReportedFinding>>,
}

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
    pub fn active_findings(
        &self,
        component_key: &str,
        artifact: &ArtifactRef,
    ) -> Vec<ReportedFinding> {
        self.active
            .get(&TrackedArtifactKey::new(
                component_key.into(),
                artifact.clone(),
            ))
            .cloned()
            .unwrap_or_default()
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

fn canonicalize_findings(findings: &[ReportedFinding]) -> Vec<ReportedFinding> {
    let mut canonical = findings.to_vec();
    canonical.sort_unstable_by(finding_sort_key);
    canonical.dedup_by(|left, right| finding_dedup_key(left) == finding_dedup_key(right));
    canonical
}

fn finding_sort_key(left: &ReportedFinding, right: &ReportedFinding) -> std::cmp::Ordering {
    finding_dedup_key(left).cmp(&finding_dedup_key(right))
}

fn finding_dedup_key(finding: &ReportedFinding) -> FindingDedupKey<'_> {
    FindingDedupKey {
        vulnerability_id: finding.vulnerability_id.as_ref(),
        package_name: finding.package.name.as_ref(),
        package_version: finding.package.version.as_ref(),
        package_purl: finding.package.purl.as_deref(),
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
    use super::FindingReadModel;
    use crate::{
        ArtifactKind, ArtifactRef, EvidenceFreshness, PackageCoordinate, ProviderScanReport,
        ReportedFinding,
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
}
