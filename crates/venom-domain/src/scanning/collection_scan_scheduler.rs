use crate::{ComponentInventory, ScanPlanner, ScanRequest};

/// Canonical due collection scan batch produced by one scheduler pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueCollectionScan {
    /// Stable collection identity that was due.
    pub collection_key: Box<str>,
    /// Due time that triggered this scheduler materialization.
    pub due_at_unix_ms: u64,
    /// Canonical scan requests materialized for this collection.
    pub requests: Vec<ScanRequest>,
    /// Next due time after this scheduler pass, in unix milliseconds.
    pub next_due_at_unix_ms: u64,
}

/// Minimal domain service that materializes due collection schedules into scan requests.
#[derive(Debug)]
pub struct CollectionScanScheduler<'a> {
    inventory: &'a mut ComponentInventory,
}

impl<'a> CollectionScanScheduler<'a> {
    #[must_use]
    pub const fn new(inventory: &'a mut ComponentInventory) -> Self {
        Self { inventory }
    }

    #[must_use]
    ///
    /// # Panics
    ///
    /// Panics if one collection materializes more than `u32::MAX` scan requests in one pass.
    pub fn collect_due(
        &mut self,
        now_unix_ms: u64,
        max_collections: usize,
    ) -> Vec<DueCollectionScan> {
        if max_collections == 0 {
            return Vec::new();
        }

        let due_collection_keys = self
            .inventory
            .due_collection_keys(now_unix_ms, max_collections);
        let mut due_scans = Vec::with_capacity(due_collection_keys.len());

        for collection_key in due_collection_keys {
            let Some(schedule) = self
                .inventory
                .collection_scan_schedule(collection_key.as_ref())
            else {
                continue;
            };
            let due_at_unix_ms = schedule.next_due_at_unix_ms;
            let Ok(batch) = ScanPlanner::new(self.inventory)
                .plan_collection(collection_key.as_ref(), schedule.freshness)
            else {
                continue;
            };
            let next_due_at_unix_ms =
                now_unix_ms.saturating_add(cadence_minutes_to_millis(schedule.cadence_minutes));
            due_scans.push(DueCollectionScan {
                collection_key,
                due_at_unix_ms,
                requests: batch.requests,
                next_due_at_unix_ms,
            });
        }

        due_scans
    }
}

const fn cadence_minutes_to_millis(cadence_minutes: u32) -> u64 {
    (cadence_minutes as u64) * 60 * 1_000
}

#[cfg(test)]
mod tests {
    use super::CollectionScanScheduler;
    use crate::{
        ArtifactKind, ArtifactRef, CollectionRegistration, ComponentInventory,
        ComponentRegistration, EvidenceFreshness,
    };

    fn artifact(identity: &str) -> ArtifactRef {
        ArtifactRef::new(ArtifactKind::ContainerImage, identity)
    }

    #[test]
    fn due_collection_scan_plans_materialization_without_mutating_inventory() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.bind_artifact(
            "component:payments-api",
            artifact("registry.example/payments@sha256:111"),
        );
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:payments-api");
        let _ = inventory.configure_collection_scan_schedule(
            "release:2026.05",
            60,
            EvidenceFreshness::Deterministic,
            1_000,
        );

        let due = CollectionScanScheduler::new(&mut inventory).collect_due(1_500, 8);

        assert_eq!(due.len(), 1);
        assert_eq!(due[0].collection_key.as_ref(), "release:2026.05");
        assert_eq!(due[0].requests.len(), 1);
        assert_eq!(
            due[0].requests[0].artifact.identity.as_ref(),
            "registry.example/payments@sha256:111"
        );
        assert_eq!(due[0].next_due_at_unix_ms, 3_601_500);
        assert_eq!(
            inventory
                .collection_scan_schedule("release:2026.05")
                .expect("schedule should remain configured")
                .next_due_at_unix_ms,
            1_000
        );
        assert_eq!(
            inventory
                .collection_scan_schedule("release:2026.05")
                .expect("schedule should remain configured")
                .last_materialized_at_unix_ms,
            None
        );
        assert_eq!(
            inventory
                .collection_scan_schedule("release:2026.05")
                .expect("schedule should remain configured")
                .last_enqueued_commands,
            None
        );
    }
}
