use crate::{ArtifactRef, ComponentInventory, EvidenceFreshness, ScanRequest};

/// Canonical failure when planning a scan request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPlanningError {
    /// The requested component is not under management.
    UnmanagedComponent,
    /// The requested artifact is not bound to the managed component.
    UnmanagedArtifact,
}

impl ScanPlanningError {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnmanagedComponent => "unmanaged-component",
            Self::UnmanagedArtifact => "unmanaged-artifact",
        }
    }
}

/// Minimal domain service that turns managed ownership into canonical scan requests.
#[derive(Debug, Clone, Copy)]
pub struct ScanPlanner<'a> {
    inventory: &'a ComponentInventory,
}

impl<'a> ScanPlanner<'a> {
    #[must_use]
    pub const fn new(inventory: &'a ComponentInventory) -> Self {
        Self { inventory }
    }

    /// Create one canonical scan request for a managed component and owned artifact.
    ///
    /// # Errors
    ///
    /// Returns [`ScanPlanningError::UnmanagedComponent`] when the component is
    /// not under management, or [`ScanPlanningError::UnmanagedArtifact`] when
    /// the artifact is not bound to that component.
    pub fn plan(
        &self,
        component_key: &str,
        artifact: ArtifactRef,
        freshness: EvidenceFreshness,
    ) -> Result<ScanRequest, ScanPlanningError> {
        if !self.inventory.is_managed(component_key) {
            return Err(ScanPlanningError::UnmanagedComponent);
        }
        if !self
            .inventory
            .component_owns_artifact(component_key, &artifact)
        {
            return Err(ScanPlanningError::UnmanagedArtifact);
        }

        Ok(ScanRequest::new(
            component_key.to_owned(),
            artifact,
            freshness,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{ScanPlanner, ScanPlanningError};
    use crate::{
        ArtifactKind, ArtifactRef, ComponentInventory, ComponentRegistration, EvidenceFreshness,
    };

    fn artifact(identity: &str) -> ArtifactRef {
        ArtifactRef::new(ArtifactKind::ContainerImage, identity)
    }

    #[test]
    fn unmanaged_component_cannot_plan_a_scan() {
        let inventory = ComponentInventory::default();
        let planner = ScanPlanner::new(&inventory);

        let result = planner.plan(
            "component:payments-api",
            artifact("registry.example/payments@sha256:111"),
            EvidenceFreshness::Deterministic,
        );

        assert_eq!(result, Err(ScanPlanningError::UnmanagedComponent));
    }

    #[test]
    fn managed_component_needs_owned_artifact_to_plan_a_scan() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let planner = ScanPlanner::new(&inventory);

        let result = planner.plan(
            "component:payments-api",
            artifact("registry.example/payments@sha256:111"),
            EvidenceFreshness::Deterministic,
        );

        assert_eq!(result, Err(ScanPlanningError::UnmanagedArtifact));
    }

    #[test]
    fn managed_component_can_plan_a_live_scan_for_owned_artifact() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.bind_artifact(
            "component:payments-api",
            artifact("registry.example/payments@sha256:111"),
        );
        let planner = ScanPlanner::new(&inventory);

        let request = planner
            .plan(
                "component:payments-api",
                artifact("registry.example/payments@sha256:111"),
                EvidenceFreshness::Live,
            )
            .expect("owned artifact should produce a canonical scan request");

        assert_eq!(request.component_key.as_ref(), "component:payments-api");
        assert_eq!(
            request.artifact.identity.as_ref(),
            "registry.example/payments@sha256:111"
        );
        assert_eq!(request.freshness, EvidenceFreshness::Live);
    }
}
