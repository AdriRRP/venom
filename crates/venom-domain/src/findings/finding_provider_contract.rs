use crate::{
    EvidenceFreshness, FindingProviderError, ProviderScanReport, ReportedFinding, ScanRequest,
};
use std::collections::BTreeSet;

/// Semantic contract that every finding provider report must satisfy.
///
/// These checks are intentionally provider-agnostic. They verify that an
/// adapter returns a canonical snapshot shape that VENOM can trust, regardless
/// of how the adapter obtained the data.
///
/// # Errors
///
/// Returns a [`FindingProviderContractViolation`] when the report does not echo
/// the request correctly or when any normalized finding breaks the canonical
/// provider-boundary rules.
pub fn validate_provider_scan_report(
    provider_key: &'static str,
    request: &ScanRequest,
    report: &ProviderScanReport,
) -> Result<(), FindingProviderContractViolation> {
    if report.provider_key.as_ref() != provider_key {
        return Err(FindingProviderContractViolation::ProviderKeyMismatch);
    }
    if report.component_key != request.component_key {
        return Err(FindingProviderContractViolation::ComponentKeyMismatch);
    }
    if report.artifact != request.artifact {
        return Err(FindingProviderContractViolation::ArtifactMismatch);
    }
    if report.freshness != request.freshness {
        return Err(FindingProviderContractViolation::FreshnessMismatch);
    }
    if request.freshness == EvidenceFreshness::Deterministic
        && report.knowledge_revision.as_deref().is_none()
    {
        return Err(FindingProviderContractViolation::MissingKnowledgeRevision);
    }

    let mut seen = BTreeSet::new();
    for finding in &report.findings {
        validate_finding(finding)?;
        let fingerprint = (
            finding.vulnerability_id.as_ref(),
            finding.package.name.as_ref(),
            finding.package.version.as_ref(),
            finding.package.purl.as_deref(),
        );

        if !seen.insert(fingerprint) {
            return Err(FindingProviderContractViolation::DuplicateFinding);
        }
    }

    Ok(())
}

/// Normalized contract violation at the provider boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingProviderContractViolation {
    ProviderKeyMismatch,
    ComponentKeyMismatch,
    ArtifactMismatch,
    FreshnessMismatch,
    MissingKnowledgeRevision,
    EmptyVulnerabilityId,
    EmptyPackageName,
    EmptyPackageVersion,
    EmptyProviderFindingKey,
    EmptyAlias,
    DuplicateFinding,
}

impl FindingProviderContractViolation {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::ProviderKeyMismatch => "report provider key does not match the adapter key",
            Self::ComponentKeyMismatch => {
                "report component key does not match the requested component"
            }
            Self::ArtifactMismatch => "report artifact does not match the requested artifact",
            Self::FreshnessMismatch => "report freshness does not match the requested freshness",
            Self::MissingKnowledgeRevision => {
                "deterministic reports must carry a knowledge revision"
            }
            Self::EmptyVulnerabilityId => "finding vulnerability id must not be empty",
            Self::EmptyPackageName => "finding package name must not be empty",
            Self::EmptyPackageVersion => "finding package version must not be empty",
            Self::EmptyProviderFindingKey => {
                "provider finding key must not be present as an empty string"
            }
            Self::EmptyAlias => "finding aliases must not contain empty strings",
            Self::DuplicateFinding => {
                "report must not contain duplicate finding fingerprints for one artifact"
            }
        }
    }
}

fn validate_finding(finding: &ReportedFinding) -> Result<(), FindingProviderContractViolation> {
    if finding.vulnerability_id.trim().is_empty() {
        return Err(FindingProviderContractViolation::EmptyVulnerabilityId);
    }
    if finding.package.name.trim().is_empty() {
        return Err(FindingProviderContractViolation::EmptyPackageName);
    }
    if finding.package.version.trim().is_empty() {
        return Err(FindingProviderContractViolation::EmptyPackageVersion);
    }
    if finding
        .provider_finding_key
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(FindingProviderContractViolation::EmptyProviderFindingKey);
    }
    if finding.aliases.iter().any(|alias| alias.trim().is_empty()) {
        return Err(FindingProviderContractViolation::EmptyAlias);
    }

    Ok(())
}

/// Convert a contract violation into a canonical provider error for runner code.
#[must_use]
pub fn as_provider_error(violation: FindingProviderContractViolation) -> FindingProviderError {
    FindingProviderError::new(
        crate::FindingProviderErrorKind::CorruptResponse,
        false,
        violation.message(),
    )
}

#[cfg(test)]
mod tests {
    use super::{FindingProviderContractViolation, validate_provider_scan_report};
    use crate::{
        ArtifactKind, ArtifactRef, EvidenceFreshness, PackageCoordinate, ProviderScanReport,
        ReportedFinding, ScanRequest, Severity,
    };
    use std::time::SystemTime;

    fn request(freshness: EvidenceFreshness) -> ScanRequest {
        ScanRequest::new(
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            freshness,
        )
    }

    fn valid_report(freshness: EvidenceFreshness) -> ProviderScanReport {
        ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            SystemTime::UNIX_EPOCH,
            freshness,
            vec![
                ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"))
                    .with_provider_finding_key("provider:1")
                    .with_severity(Severity::High)
                    .with_alias("GHSA-1234"),
            ],
        )
        .with_knowledge_revision("fixture-db:2026-05-14")
    }

    #[test]
    fn deterministic_reports_require_knowledge_revision() {
        let req = request(EvidenceFreshness::Deterministic);
        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            Vec::new(),
        );

        let result = validate_provider_scan_report("fixture-provider", &req, &report);

        assert_eq!(
            result,
            Err(FindingProviderContractViolation::MissingKnowledgeRevision)
        );
    }

    #[test]
    fn duplicate_findings_are_rejected() {
        let req = request(EvidenceFreshness::Deterministic);
        let finding =
            ReportedFinding::new("CVE-2026-0001", PackageCoordinate::new("openssl", "3.0.0"));
        let report = ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            ArtifactRef::new(
                ArtifactKind::ContainerImage,
                "registry.example/payments@sha256:111",
            ),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            vec![finding.clone(), finding],
        )
        .with_knowledge_revision("fixture-db:2026-05-14");

        let result = validate_provider_scan_report("fixture-provider", &req, &report);

        assert_eq!(
            result,
            Err(FindingProviderContractViolation::DuplicateFinding)
        );
    }

    #[test]
    fn valid_report_passes_the_contract() {
        let req = request(EvidenceFreshness::Live);
        let report = valid_report(EvidenceFreshness::Live);

        let result = validate_provider_scan_report("fixture-provider", &req, &report);

        assert_eq!(result, Ok(()));
    }
}
