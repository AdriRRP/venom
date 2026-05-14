use crate::{
    ArtifactRef, BindArtifactChange, BindArtifactResult, ComponentRegistration, EvidenceFreshness,
    FindingChangeSet, FindingIngestion, FindingIngestionError, FindingReadModel, PackageCoordinate,
    ProviderScanReport, RegisterComponentChange, RegisterComponentResult, ReportedFinding,
    Severity,
};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Minimal durable state boundary for the current domain slice.
///
/// The source of truth is a local append-only JSON-lines history. In-memory
/// domain state and read models are reconstructed from that history at open
/// time and are only swapped in after a durable append succeeds.
#[derive(Debug, Clone)]
pub struct DurableState {
    history_path: PathBuf,
    ingestion: FindingIngestion,
    read_model: FindingReadModel,
}

impl DurableState {
    /// Open or create one local durable history and rebuild domain state from it.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the history cannot be read, parsed,
    /// or replayed into a valid domain state.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, DurableStateError> {
        let history_path = path.into();
        if let Some(parent) = history_path.parent() {
            std::fs::create_dir_all(parent).map_err(DurableStateError::Io)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&history_path)
            .map_err(DurableStateError::Io)?;

        let mut state = Self {
            history_path,
            ingestion: FindingIngestion::default(),
            read_model: FindingReadModel::default(),
        };
        state.rebuild_from_history()?;
        Ok(state)
    }

    #[must_use]
    pub const fn ingestion(&self) -> &FindingIngestion {
        &self.ingestion
    }

    #[must_use]
    pub const fn read_model(&self) -> &FindingReadModel {
        &self.read_model
    }

    /// Durably register a managed component.
    ///
    /// The append-only history records only business-relevant state changes.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn register_component(
        &mut self,
        registration: ComponentRegistration,
    ) -> Result<RegisterComponentResult, DurableStateError> {
        let mut candidate = self.ingestion.clone();
        let result = candidate.inventory_mut().register(registration.clone());
        if result.change == RegisterComponentChange::Registered {
            self.append_event(&DurableEvent::ComponentRegistered {
                registration: StoredComponentRegistration::from(registration),
            })?;
            self.ingestion = candidate;
        }
        Ok(result)
    }

    /// Durably bind one immutable artifact to a managed component.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn bind_artifact(
        &mut self,
        component_key: &str,
        artifact: ArtifactRef,
    ) -> Result<BindArtifactResult, DurableStateError> {
        let mut candidate = self.ingestion.clone();
        let result = candidate
            .inventory_mut()
            .bind_artifact(component_key, artifact.clone());
        if result.change == BindArtifactChange::Bound {
            self.append_event(&DurableEvent::ArtifactBound {
                component_key: component_key.into(),
                artifact,
            })?;
            self.ingestion = candidate;
        }
        Ok(result)
    }

    /// Durably record one accepted provider snapshot and update the projection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::Ingestion`] when the report violates
    /// managed ownership, or another [`DurableStateError`] when the durable
    /// append or history format fails.
    pub fn record_scan_report(
        &mut self,
        report: &ProviderScanReport,
    ) -> Result<FindingChangeSet, DurableStateError> {
        let mut candidate_ingestion = self.ingestion.clone();
        let mut candidate_read_model = self.read_model.clone();
        let change_set = candidate_ingestion
            .record_scan_report(report)
            .map_err(DurableStateError::Ingestion)?;
        candidate_read_model.record_scan_report(report);
        self.append_event(&DurableEvent::ProviderScanRecorded {
            report: StoredProviderScanReport::from_report(report)?,
        })?;
        self.ingestion = candidate_ingestion;
        self.read_model = candidate_read_model;
        Ok(change_set)
    }

    fn rebuild_from_history(&mut self) -> Result<(), DurableStateError> {
        let file = File::open(&self.history_path).map_err(DurableStateError::Io)?;
        let reader = BufReader::new(file);
        self.ingestion = FindingIngestion::default();
        self.read_model = FindingReadModel::default();

        for (line_index, line) in reader.lines().enumerate() {
            let line = line.map_err(DurableStateError::Io)?;
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str::<DurableEvent>(&line).map_err(|error| {
                DurableStateError::CorruptHistory {
                    line: line_index + 1,
                    reason: error.to_string().into_boxed_str(),
                }
            })?;
            self.apply_event(event, line_index + 1)?;
        }

        Ok(())
    }

    fn apply_event(&mut self, event: DurableEvent, line: usize) -> Result<(), DurableStateError> {
        match event {
            DurableEvent::ComponentRegistered { registration } => {
                let result = self
                    .ingestion
                    .inventory_mut()
                    .register(registration.into_domain());
                match result.change {
                    RegisterComponentChange::Registered | RegisterComponentChange::Unchanged => {
                        Ok(())
                    }
                    RegisterComponentChange::Rejected => Err(DurableStateError::CorruptHistory {
                        line,
                        reason: "conflicting component registration".into(),
                    }),
                }
            }
            DurableEvent::ArtifactBound {
                component_key,
                artifact,
            } => {
                let result = self
                    .ingestion
                    .inventory_mut()
                    .bind_artifact(component_key.as_ref(), artifact);
                match result.change {
                    BindArtifactChange::Bound | BindArtifactChange::Unchanged => Ok(()),
                    BindArtifactChange::Rejected => Err(DurableStateError::CorruptHistory {
                        line,
                        reason: "invalid artifact binding".into(),
                    }),
                }
            }
            DurableEvent::ProviderScanRecorded { report } => {
                let report = report.into_domain()?;
                self.ingestion
                    .record_scan_report(&report)
                    .map_err(DurableStateError::Ingestion)
                    .map_err(|error| match error {
                        DurableStateError::Ingestion(reason) => DurableStateError::CorruptHistory {
                            line,
                            reason: format!(
                                "provider report cannot be replayed: {}",
                                reason.as_str()
                            )
                            .into_boxed_str(),
                        },
                        other => other,
                    })?;
                self.read_model.record_scan_report(&report);
                Ok(())
            }
        }
    }

    fn append_event(&self, event: &DurableEvent) -> Result<(), DurableStateError> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.history_path)
            .map_err(DurableStateError::Io)?;
        serde_json::to_writer(&mut file, event).map_err(DurableStateError::Serialize)?;
        file.write_all(b"\n").map_err(DurableStateError::Io)?;
        file.flush().map_err(DurableStateError::Io)?;
        file.sync_all().map_err(DurableStateError::Io)?;
        Ok(())
    }
}

/// Canonical failure returned by the local durable state boundary.
#[derive(Debug)]
pub enum DurableStateError {
    Io(io::Error),
    Serialize(serde_json::Error),
    CorruptHistory { line: usize, reason: Box<str> },
    Ingestion(FindingIngestionError),
    Time(String),
}

impl DurableStateError {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "io-error",
            Self::Serialize(_) => "serialization-error",
            Self::CorruptHistory { .. } => "corrupt-history",
            Self::Ingestion(FindingIngestionError::UnmanagedComponent) => "unmanaged-component",
            Self::Ingestion(FindingIngestionError::UnmanagedArtifact) => "unmanaged-artifact",
            Self::Time(_) => "invalid-time",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum DurableEvent {
    ComponentRegistered {
        registration: StoredComponentRegistration,
    },
    ArtifactBound {
        component_key: Box<str>,
        artifact: ArtifactRef,
    },
    ProviderScanRecorded {
        report: StoredProviderScanReport,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredComponentRegistration {
    component_key: Box<str>,
    name: Box<str>,
}

impl From<ComponentRegistration> for StoredComponentRegistration {
    fn from(value: ComponentRegistration) -> Self {
        Self {
            component_key: value.component_key,
            name: value.name,
        }
    }
}

impl StoredComponentRegistration {
    fn into_domain(self) -> ComponentRegistration {
        ComponentRegistration::new(self.component_key, self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredProviderScanReport {
    provider_key: Box<str>,
    component_key: Box<str>,
    artifact: ArtifactRef,
    observed_at: Box<str>,
    freshness: EvidenceFreshness,
    knowledge_revision: Option<Box<str>>,
    findings: Vec<StoredReportedFinding>,
}

impl StoredProviderScanReport {
    fn from_report(report: &ProviderScanReport) -> Result<Self, DurableStateError> {
        let observed_at = OffsetDateTime::from(report.observed_at)
            .format(&Rfc3339)
            .map_err(|error| DurableStateError::Time(error.to_string()))?;
        Ok(Self {
            provider_key: report.provider_key.clone(),
            component_key: report.component_key.clone(),
            artifact: report.artifact.clone(),
            observed_at: observed_at.into_boxed_str(),
            freshness: report.freshness,
            knowledge_revision: report.knowledge_revision.clone(),
            findings: report
                .findings
                .iter()
                .cloned()
                .map(StoredReportedFinding::from)
                .collect(),
        })
    }

    fn into_domain(self) -> Result<ProviderScanReport, DurableStateError> {
        let observed_at = OffsetDateTime::parse(&self.observed_at, &Rfc3339)
            .map_err(|error| DurableStateError::Time(error.to_string()))?
            .into();
        let mut report = ProviderScanReport::new(
            self.provider_key,
            self.component_key,
            self.artifact,
            observed_at,
            self.freshness,
            self.findings
                .into_iter()
                .map(StoredReportedFinding::into_domain)
                .collect(),
        );
        report.knowledge_revision = self.knowledge_revision;
        Ok(report)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredReportedFinding {
    vulnerability_id: Box<str>,
    provider_finding_key: Option<Box<str>>,
    package: StoredPackageCoordinate,
    fix_version: Option<Box<str>>,
    severity: Severity,
    aliases: Vec<Box<str>>,
}

impl From<ReportedFinding> for StoredReportedFinding {
    fn from(value: ReportedFinding) -> Self {
        Self {
            vulnerability_id: value.vulnerability_id,
            provider_finding_key: value.provider_finding_key,
            package: StoredPackageCoordinate::from(value.package),
            fix_version: value.fix_version,
            severity: value.severity,
            aliases: value.aliases,
        }
    }
}

impl StoredReportedFinding {
    fn into_domain(self) -> ReportedFinding {
        ReportedFinding {
            vulnerability_id: self.vulnerability_id,
            provider_finding_key: self.provider_finding_key,
            package: self.package.into_domain(),
            fix_version: self.fix_version,
            severity: self.severity,
            aliases: self.aliases,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPackageCoordinate {
    name: Box<str>,
    version: Box<str>,
    purl: Option<Box<str>>,
}

impl From<PackageCoordinate> for StoredPackageCoordinate {
    fn from(value: PackageCoordinate) -> Self {
        Self {
            name: value.name,
            version: value.version,
            purl: value.purl,
        }
    }
}

impl StoredPackageCoordinate {
    fn into_domain(self) -> PackageCoordinate {
        PackageCoordinate {
            name: self.name,
            version: self.version,
            purl: self.purl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DurableState;
    use crate::{
        ArtifactKind, ArtifactRef, ComponentRegistration, EvidenceFreshness, PackageCoordinate,
        ProviderScanReport, ReportedFinding,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("venom-{name}-{nanos}-{counter}.jsonl"))
    }

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
        .with_knowledge_revision("fixture-db:2026-05-14")
    }

    #[test]
    fn replay_rebuilds_inventory_and_active_findings() {
        let path = temp_path("durable-state-replay");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("scan report should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert!(
            rebuilt
                .ingestion()
                .inventory()
                .is_managed("component:payments-api")
        );
        assert!(
            rebuilt
                .ingestion()
                .inventory()
                .component_owns_artifact("component:payments-api", &artifact())
        );
        assert_eq!(
            rebuilt
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            1
        );
    }

    #[test]
    fn replay_keeps_withdrawn_findings_inactive() {
        let path = temp_path("durable-state-withdrawal");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("first report should persist");
        let _ = state
            .record_scan_report(&report(Vec::new()))
            .expect("withdrawal snapshot should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert_eq!(
            rebuilt
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            0
        );
    }
}
