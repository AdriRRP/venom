use crate::{ComponentInventory, FindingChangeSet, FindingTracker, ProviderScanReport};

/// Canonical failure when ingesting a provider scan report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingIngestionError {
    /// The report references a component that is not under management.
    UnmanagedComponent,
    /// The report references an artifact that is not bound to the component.
    UnmanagedArtifact,
}

impl FindingIngestionError {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnmanagedComponent => "unmanaged-component",
            Self::UnmanagedArtifact => "unmanaged-artifact",
        }
    }
}

/// Minimal domain service that gates finding ingestion behind inventory.
#[derive(Debug, Clone, Default)]
pub struct FindingIngestion {
    inventory: ComponentInventory,
    tracker: FindingTracker,
}

impl FindingIngestion {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn inventory(&self) -> &ComponentInventory {
        &self.inventory
    }

    #[must_use]
    pub const fn inventory_mut(&mut self) -> &mut ComponentInventory {
        &mut self.inventory
    }

    /// Record one provider report only if the component is managed.
    ///
    /// # Errors
    ///
    /// Returns [`FindingIngestionError::UnmanagedComponent`] when the report
    /// references a component key that is not currently under management, or
    /// [`FindingIngestionError::UnmanagedArtifact`] when the component does not
    /// own the reported immutable artifact.
    pub fn record_scan_report(
        &mut self,
        report: &ProviderScanReport,
    ) -> Result<FindingChangeSet, FindingIngestionError> {
        if !self.inventory.is_managed(report.component_key.as_ref()) {
            return Err(FindingIngestionError::UnmanagedComponent);
        }
        if !self
            .inventory
            .component_owns_artifact(report.component_key.as_ref(), &report.artifact)
        {
            return Err(FindingIngestionError::UnmanagedArtifact);
        }

        Ok(self.tracker.record_scan_report(report))
    }
}

#[cfg(test)]
mod tests {
    use super::{FindingIngestion, FindingIngestionError};
    use crate::{
        ArtifactKind, ArtifactRef, ComponentRegistration, EvidenceFreshness, PackageCoordinate,
        ProviderScanReport, ReportedFinding,
    };
    use std::time::SystemTime;

    fn report() -> ProviderScanReport {
        ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )],
        )
    }

    #[test]
    fn unmanaged_components_are_rejected() {
        let mut ingestion = FindingIngestion::new();

        let result = ingestion.record_scan_report(&report());

        assert_eq!(result, Err(FindingIngestionError::UnmanagedComponent));
    }

    #[test]
    fn managed_components_can_record_reports() {
        let mut ingestion = FindingIngestion::new();
        let _ = ingestion
            .inventory_mut()
            .register(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ));
        let _ = ingestion.inventory_mut().bind_artifact(
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
        );

        let result = ingestion
            .record_scan_report(&report())
            .expect("managed component should accept a provider report");

        assert_eq!(result.discovered, 1);
        assert_eq!(result.active, 1);
    }

    #[test]
    fn managed_components_reject_unbound_artifacts() {
        let mut ingestion = FindingIngestion::new();
        let _ = ingestion
            .inventory_mut()
            .register(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ));

        let result = ingestion.record_scan_report(&report());

        assert_eq!(result, Err(FindingIngestionError::UnmanagedArtifact));
    }
}
