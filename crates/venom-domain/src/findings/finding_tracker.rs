use crate::{ArtifactRef, ProviderScanReport, ReportedFinding};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Observable effect of recording one provider scan report.
///
/// This is the smallest business-facing summary needed after comparing the new
/// snapshot with the previous known snapshot for the same component and
/// artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingChangeSet {
    /// Findings present now that were not present in the previous snapshot.
    pub discovered: usize,
    /// Findings present both before and now.
    pub repeated: usize,
    /// Findings that were present before but are absent now.
    pub withdrawn: usize,
    /// Total active findings after applying the new snapshot.
    pub active: usize,
}

impl FindingChangeSet {
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        self.discovered == 0 && self.withdrawn == 0
    }
}

/// In-memory snapshot comparator for provider scan reports.
///
/// This type exists to make the core lifecycle semantics explicit: providers
/// send full snapshots, and VENOM derives discovery, repetition, and
/// withdrawal by comparing canonical fingerprints over time.
#[derive(Debug, Clone, Default)]
pub struct FindingTracker {
    snapshots: HashMap<TrackedArtifactKey, Vec<FindingFingerprint>>,
}

impl FindingTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one provider snapshot and derive the resulting lifecycle changes.
    ///
    /// The comparison key is the pair `(component, artifact)` plus a canonical
    /// per-finding fingerprint built from vulnerability and package identity.
    #[must_use]
    pub fn record_scan_report(&mut self, report: &ProviderScanReport) -> FindingChangeSet {
        let artifact_key =
            TrackedArtifactKey::new(report.component_key.clone(), report.artifact.clone());
        let current = canonicalize_findings(&report.findings);

        let previous = self.snapshots.entry(artifact_key).or_default();
        let (discovered, repeated, withdrawn) = diff_sorted(previous, &current);

        *previous = current;

        FindingChangeSet {
            discovered,
            repeated,
            withdrawn,
            active: previous.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FindingFingerprint {
    vulnerability_id: Box<str>,
    package_name: Box<str>,
    package_version: Box<str>,
    package_purl: Option<Box<str>>,
}

impl From<&ReportedFinding> for FindingFingerprint {
    fn from(value: &ReportedFinding) -> Self {
        Self {
            vulnerability_id: value.vulnerability_id.clone(),
            package_name: value.package.name.clone(),
            package_version: value.package.version.clone(),
            package_purl: value.package.purl.clone(),
        }
    }
}

fn canonicalize_findings(findings: &[ReportedFinding]) -> Vec<FindingFingerprint> {
    let mut canonical = findings
        .iter()
        .map(FindingFingerprint::from)
        .collect::<Vec<_>>();
    canonical.sort_unstable();
    canonical.dedup();
    canonical
}

fn diff_sorted(
    previous: &[FindingFingerprint],
    current: &[FindingFingerprint],
) -> (usize, usize, usize) {
    let mut previous_index = 0;
    let mut current_index = 0;
    let mut discovered = 0;
    let mut repeated = 0;
    let mut withdrawn = 0;

    while previous_index < previous.len() && current_index < current.len() {
        match previous[previous_index].cmp(&current[current_index]) {
            std::cmp::Ordering::Less => {
                withdrawn += 1;
                previous_index += 1;
            }
            std::cmp::Ordering::Equal => {
                repeated += 1;
                previous_index += 1;
                current_index += 1;
            }
            std::cmp::Ordering::Greater => {
                discovered += 1;
                current_index += 1;
            }
        }
    }

    withdrawn += previous.len().saturating_sub(previous_index);
    discovered += current.len().saturating_sub(current_index);

    (discovered, repeated, withdrawn)
}

#[cfg(test)]
mod tests {
    use super::FindingTracker;
    use crate::{
        ArtifactKind, ArtifactRef, EvidenceFreshness, PackageCoordinate, ProviderScanReport,
        ReportedFinding, Severity,
    };
    use std::time::SystemTime;

    fn report(findings: Vec<ReportedFinding>) -> ProviderScanReport {
        ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            findings,
        )
    }

    fn openssl_finding() -> ReportedFinding {
        ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
            .with_severity(Severity::High)
    }

    #[test]
    fn first_snapshot_discovers_one_active_finding() {
        let mut tracker = FindingTracker::new();

        let change_set = tracker.record_scan_report(&report(vec![openssl_finding()]));

        assert_eq!(change_set.discovered, 1);
        assert_eq!(change_set.repeated, 0);
        assert_eq!(change_set.withdrawn, 0);
        assert_eq!(change_set.active, 1);
    }

    #[test]
    fn repeated_snapshot_does_not_discover_the_same_finding_twice() {
        let mut tracker = FindingTracker::new();
        let report = report(vec![openssl_finding()]);

        let first = tracker.record_scan_report(&report);
        let second = tracker.record_scan_report(&report);

        assert_eq!(first.discovered, 1);
        assert_eq!(second.discovered, 0);
        assert_eq!(second.repeated, 1);
        assert_eq!(second.withdrawn, 0);
        assert_eq!(second.active, 1);
        assert!(second.is_idle());
    }

    #[test]
    fn missing_finding_is_withdrawn_when_next_snapshot_is_empty() {
        let mut tracker = FindingTracker::new();

        let _ = tracker.record_scan_report(&report(vec![openssl_finding()]));
        let withdrawn = tracker.record_scan_report(&report(Vec::new()));

        assert_eq!(withdrawn.discovered, 0);
        assert_eq!(withdrawn.repeated, 0);
        assert_eq!(withdrawn.withdrawn, 1);
        assert_eq!(withdrawn.active, 0);
    }
}
