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

/// Canonical failure when planning one collection scan batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionScanPlanningError {
    /// The requested collection is not under management.
    UnmanagedCollection,
}

impl CollectionScanPlanningError {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnmanagedCollection => "unmanaged-collection",
        }
    }
}

/// Canonical batch of scan requests derived from one closed collection scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionScanBatch {
    /// Stable collection identity that produced the batch.
    pub collection_key: Box<str>,
    /// Canonical scan requests expanded from the collection members and owned artifacts.
    pub requests: Vec<ScanRequest>,
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

    /// Create one canonical scan batch for a closed collection.
    ///
    /// # Errors
    ///
    /// Returns [`CollectionScanPlanningError::UnmanagedCollection`] when the
    /// collection is unknown.
    pub fn plan_collection(
        &self,
        collection_key: &str,
        freshness: EvidenceFreshness,
    ) -> Result<CollectionScanBatch, CollectionScanPlanningError> {
        let Some(component_keys) = self.inventory.collection_members(collection_key) else {
            return Err(CollectionScanPlanningError::UnmanagedCollection);
        };

        let mut requests = Vec::new();
        for component_key in component_keys {
            let Some(artifacts) = self.inventory.bound_artifact_refs(component_key.as_ref()) else {
                continue;
            };
            for artifact in artifacts {
                requests.push(ScanRequest::new(
                    component_key.to_string(),
                    artifact,
                    freshness,
                ));
            }
        }

        Ok(CollectionScanBatch {
            collection_key: collection_key.into(),
            requests,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CollectionScanPlanningError, ScanPlanner, ScanPlanningError};
    use crate::{
        ArtifactKind, ArtifactRef, CollectionRegistration, ComponentInventory,
        ComponentRegistration, EvidenceFreshness,
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

    #[test]
    fn unmanaged_collection_cannot_plan_a_scan_batch() {
        let inventory = ComponentInventory::default();
        let planner = ScanPlanner::new(&inventory);

        let result = planner.plan_collection("release:2026.05", EvidenceFreshness::Deterministic);

        assert_eq!(
            result,
            Err(CollectionScanPlanningError::UnmanagedCollection)
        );
    }

    #[test]
    fn managed_collection_expands_to_all_owned_artifacts() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.register(ComponentRegistration::new(
            "component:billing-api",
            "Billing API",
        ));
        let _ = inventory.bind_artifact(
            "component:payments-api",
            artifact("registry.example/payments@sha256:111"),
        );
        let _ = inventory.bind_artifact(
            "component:billing-api",
            artifact("registry.example/billing@sha256:222"),
        );
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:billing-api");
        let _ = inventory.add_component_to_collection("release:2026.05", "component:payments-api");
        let planner = ScanPlanner::new(&inventory);

        let batch = planner
            .plan_collection("release:2026.05", EvidenceFreshness::Deterministic)
            .expect("managed collection should expand to a canonical scan batch");

        assert_eq!(batch.collection_key.as_ref(), "release:2026.05");
        assert_eq!(batch.requests.len(), 2);
        assert_eq!(
            batch.requests[0].component_key.as_ref(),
            "component:billing-api"
        );
        assert_eq!(
            batch.requests[0].artifact.identity.as_ref(),
            "registry.example/billing@sha256:222"
        );
        assert_eq!(
            batch.requests[1].component_key.as_ref(),
            "component:payments-api"
        );
        assert_eq!(
            batch.requests[1].artifact.identity.as_ref(),
            "registry.example/payments@sha256:111"
        );
    }
}
