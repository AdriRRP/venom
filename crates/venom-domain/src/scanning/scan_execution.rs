use crate::findings::finding_provider_contract::{
    as_provider_error, validate_provider_scan_report,
};
use crate::{
    FindingChangeSet, FindingIngestion, FindingIngestionError, FindingProvider,
    FindingProviderError, ScanRequest,
};

/// Observable outcome of executing one canonical scan request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanExecutionResult {
    /// Provider that executed the request.
    pub provider_key: Box<str>,
    /// Number of findings present in the provider snapshot.
    pub findings_reported: usize,
    /// Business-visible change after applying the provider report.
    pub change_set: FindingChangeSet,
}

/// Canonical failure when executing one scan request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanExecutionError {
    /// The provider failed or returned an untrustworthy report.
    Provider(FindingProviderError),
    /// The provider report could not be applied to managed ownership.
    Ingestion(FindingIngestionError),
}

impl ScanExecutionError {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Provider(_) => "provider-error",
            Self::Ingestion(FindingIngestionError::UnmanagedComponent) => "unmanaged-component",
            Self::Ingestion(FindingIngestionError::UnmanagedArtifact) => "unmanaged-artifact",
        }
    }
}

/// Execute one canonical scan request through a provider and apply its report.
///
/// # Errors
///
/// Returns [`ScanExecutionError::Provider`] when the provider fails or returns
/// a report that does not satisfy the canonical provider contract, or
/// [`ScanExecutionError::Ingestion`] when the resulting report cannot be
/// applied to managed ownership.
pub async fn execute_scan(
    ingestion: &mut FindingIngestion,
    provider: &(impl FindingProvider + Sync),
    request: &ScanRequest,
) -> Result<ScanExecutionResult, ScanExecutionError> {
    let report = provider
        .scan(request)
        .await
        .map_err(ScanExecutionError::Provider)?;

    validate_provider_scan_report(provider.provider_key(), request, &report)
        .map_err(as_provider_error)
        .map_err(ScanExecutionError::Provider)?;

    let findings_reported = report.findings.len();
    let change_set = ingestion
        .record_scan_report(&report)
        .map_err(ScanExecutionError::Ingestion)?;

    Ok(ScanExecutionResult {
        provider_key: report.provider_key,
        findings_reported,
        change_set,
    })
}

#[cfg(test)]
mod tests {
    use super::{ScanExecutionError, execute_scan};
    use crate::{
        ArtifactKind, ArtifactRef, ComponentRegistration, EvidenceFreshness, FindingIngestion,
        FindingProvider, FindingProviderError, FindingProviderErrorKind, PackageCoordinate,
        ProviderScanReport, ReportedFinding, ScanPlanner,
    };
    use std::time::SystemTime;

    #[derive(Debug, Clone)]
    enum FakeProviderMode {
        Success(Vec<ReportedFinding>),
        Failure(FindingProviderError),
    }

    #[derive(Debug, Clone)]
    struct FakeProvider {
        mode: FakeProviderMode,
    }

    impl FakeProvider {
        fn success(findings: Vec<ReportedFinding>) -> Self {
            Self {
                mode: FakeProviderMode::Success(findings),
            }
        }

        fn failure(error: FindingProviderError) -> Self {
            Self {
                mode: FakeProviderMode::Failure(error),
            }
        }
    }

    impl FindingProvider for FakeProvider {
        fn provider_key(&self) -> &'static str {
            "fake-provider"
        }

        async fn scan<'a>(
            &'a self,
            request: &'a crate::ScanRequest,
        ) -> Result<ProviderScanReport, FindingProviderError> {
            match &self.mode {
                FakeProviderMode::Success(findings) => Ok(ProviderScanReport::new(
                    "fake-provider",
                    request.component_key.clone(),
                    request.artifact.clone(),
                    SystemTime::UNIX_EPOCH,
                    request.freshness,
                    findings.clone(),
                )
                .with_knowledge_revision("fake-db:2026-05-14")),
                FakeProviderMode::Failure(error) => Err(error.clone()),
            }
        }
    }

    fn openssl_finding() -> ReportedFinding {
        ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
    }

    fn managed_ingestion() -> (FindingIngestion, crate::ScanRequest) {
        let mut ingestion = FindingIngestion::new();
        let artifact = ArtifactRef::new(
            ArtifactKind::ContainerImage,
            "registry.example/payments@sha256:111",
        );
        let _ = ingestion
            .inventory_mut()
            .register(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ));
        let _ = ingestion
            .inventory_mut()
            .bind_artifact("component:payments-api", artifact.clone());
        let planner = ScanPlanner::new(ingestion.inventory());
        let request = planner
            .plan(
                "component:payments-api",
                artifact,
                EvidenceFreshness::Deterministic,
            )
            .expect("managed ownership should plan a scan");

        (ingestion, request)
    }

    #[tokio::test]
    async fn successful_execution_applies_provider_findings() {
        let (mut ingestion, request) = managed_ingestion();
        let provider = FakeProvider::success(vec![openssl_finding()]);

        let result = execute_scan(&mut ingestion, &provider, &request)
            .await
            .expect("provider execution should succeed");

        assert_eq!(result.provider_key.as_ref(), "fake-provider");
        assert_eq!(result.findings_reported, 1);
        assert_eq!(result.change_set.discovered, 1);
        assert_eq!(result.change_set.active, 1);
    }

    #[tokio::test]
    async fn provider_failure_is_reported() {
        let (mut ingestion, request) = managed_ingestion();
        let provider = FakeProvider::failure(FindingProviderError::new(
            FindingProviderErrorKind::Unavailable,
            true,
            "scanner unavailable",
        ));

        let result = execute_scan(&mut ingestion, &provider, &request).await;

        assert!(matches!(result, Err(ScanExecutionError::Provider(_))));
    }

    #[tokio::test]
    async fn execution_still_respects_ingestion_guards() {
        let mut ingestion = FindingIngestion::new();
        let request = crate::ScanRequest::new(
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            EvidenceFreshness::Deterministic,
        );
        let provider = FakeProvider::success(vec![openssl_finding()]);

        let result = execute_scan(&mut ingestion, &provider, &request).await;

        assert_eq!(
            result,
            Err(ScanExecutionError::Ingestion(
                crate::FindingIngestionError::UnmanagedComponent
            ))
        );
    }
}
