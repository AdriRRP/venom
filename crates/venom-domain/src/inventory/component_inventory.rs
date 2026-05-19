use crate::{ArtifactRef, EvidenceFreshness};
use std::collections::{BTreeMap, BTreeSet};

/// Canonical registration request for one managed component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRegistration {
    /// Stable component identity inside VENOM.
    pub component_key: Box<str>,
    /// Human-readable component name.
    pub name: Box<str>,
}

impl ComponentRegistration {
    #[must_use]
    pub fn new(component_key: impl Into<Box<str>>, name: impl Into<Box<str>>) -> Self {
        Self {
            component_key: component_key.into(),
            name: name.into(),
        }
    }
}

/// Observable outcome of a component registration attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterComponentChange {
    /// The component key was new and is now managed.
    Registered,
    /// The same component was registered again with the same canonical data.
    Unchanged,
    /// The component key already exists with conflicting canonical data.
    Rejected,
}

impl RegisterComponentChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of one registration command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterComponentResult {
    /// Observable state change caused by the registration attempt.
    pub change: RegisterComponentChange,
    /// Total number of managed components after the operation.
    pub managed_components: usize,
}

/// Observable outcome of binding one immutable artifact to a managed component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindArtifactChange {
    /// The artifact is now bound to the component.
    Bound,
    /// The exact same ownership binding already existed.
    Unchanged,
    /// The binding was rejected because the component is missing or the
    /// artifact already belongs to another component.
    Rejected,
}

impl BindArtifactChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of binding one artifact to a managed component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindArtifactResult {
    /// Observable state change caused by the binding attempt.
    pub change: BindArtifactChange,
    /// Total number of artifacts currently bound to the component.
    pub bound_artifacts: usize,
}

/// Observable outcome of configuring one component provider runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigureProviderChange {
    /// The component now has a provider runtime configuration.
    Configured,
    /// The exact same provider runtime configuration already existed.
    Unchanged,
    /// The configuration was rejected because the component is missing.
    Rejected,
}

impl ConfigureProviderChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of configuring one component provider runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigureProviderResult {
    /// Observable state change caused by the configuration attempt.
    pub change: ConfigureProviderChange,
    /// Configured provider key after the operation when the component exists.
    pub provider_key: Option<Box<str>>,
}

/// Canonical registration request for one reusable execution-context profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextProfileRegistration {
    /// Stable profile identity inside VENOM.
    pub profile_key: Box<str>,
    /// Human-readable profile name.
    pub name: Box<str>,
    /// Whether the workload is exposed to untrusted network paths.
    pub internet_exposed: bool,
    /// Whether the workload is part of production runtime.
    pub production: bool,
    /// Whether the workload is mission-critical for the operator.
    pub mission_critical: bool,
}

impl ContextProfileRegistration {
    #[must_use]
    pub fn new(
        profile_key: impl Into<Box<str>>,
        name: impl Into<Box<str>>,
        internet_exposed: bool,
        production: bool,
        mission_critical: bool,
    ) -> Self {
        Self {
            profile_key: profile_key.into(),
            name: name.into(),
            internet_exposed,
            production,
            mission_critical,
        }
    }
}

/// Observable outcome of one context profile registration attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterContextProfileChange {
    /// The profile key was new and is now managed.
    Registered,
    /// The same profile was registered again with the same canonical data.
    Unchanged,
    /// The profile key already exists with conflicting canonical data.
    Rejected,
}

impl RegisterContextProfileChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of one context profile registration command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterContextProfileResult {
    /// Observable state change caused by the profile registration attempt.
    pub change: RegisterContextProfileChange,
    /// Total number of managed context profiles after the operation.
    pub managed_context_profiles: usize,
}

/// Operator-facing snapshot of one managed execution-context profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedContextProfile {
    /// Stable profile identity inside VENOM.
    pub profile_key: Box<str>,
    /// Human-readable profile name.
    pub name: Box<str>,
    /// Whether the workload is exposed to untrusted network paths.
    pub internet_exposed: bool,
    /// Whether the workload is part of production runtime.
    pub production: bool,
    /// Whether the workload is mission-critical for the operator.
    pub mission_critical: bool,
}

/// Observable outcome of assigning one context profile to one managed component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignContextProfileChange {
    /// The component now uses the context profile.
    Assigned,
    /// The exact same assignment already existed.
    Unchanged,
    /// The assignment was rejected because the component or profile is missing.
    Rejected,
}

impl AssignContextProfileChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Assigned => "assigned",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of assigning one context profile to one managed component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignContextProfileResult {
    /// Observable state change caused by the assignment attempt.
    pub change: AssignContextProfileChange,
    /// Assigned profile key after the operation when the component exists.
    pub profile_key: Option<Box<str>>,
}

/// Canonical registration request for one managed collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionRegistration {
    /// Stable collection identity inside VENOM.
    pub collection_key: Box<str>,
    /// Human-readable collection name.
    pub name: Box<str>,
}

impl CollectionRegistration {
    #[must_use]
    pub fn new(collection_key: impl Into<Box<str>>, name: impl Into<Box<str>>) -> Self {
        Self {
            collection_key: collection_key.into(),
            name: name.into(),
        }
    }
}

/// Observable outcome of one collection creation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterCollectionChange {
    /// The collection key was new and is now managed.
    Created,
    /// The same collection was created again with the same canonical data.
    Unchanged,
    /// The collection key already exists with conflicting canonical data.
    Rejected,
}

impl RegisterCollectionChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of one collection creation command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterCollectionResult {
    /// Observable state change caused by the collection creation attempt.
    pub change: RegisterCollectionChange,
    /// Total number of managed collections after the operation.
    pub managed_collections: usize,
}

/// Observable outcome of adding one managed component to one collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddCollectionComponentChange {
    /// The component is now part of the collection.
    Added,
    /// The exact same membership already existed.
    Unchanged,
    /// The collection or component is unknown.
    Rejected,
}

impl AddCollectionComponentChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of adding one managed component to one collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddCollectionComponentResult {
    /// Observable state change caused by the membership attempt.
    pub change: AddCollectionComponentChange,
    /// Total number of members currently in the collection after the operation.
    pub members: usize,
}

/// Observable outcome of removing one managed component from one collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveCollectionComponentChange {
    /// The component is no longer part of the collection.
    Removed,
    /// The component was already absent from the collection.
    Unchanged,
    /// The collection is unknown.
    Rejected,
}

impl RemoveCollectionComponentChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Removed => "removed",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of removing one managed component from one collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveCollectionComponentResult {
    /// Observable state change caused by the membership removal attempt.
    pub change: RemoveCollectionComponentChange,
    /// Total number of members currently in the collection after the operation.
    pub members: usize,
}

/// Canonical kind of one declared collection source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionSourceKind {
    /// The collection derives membership from one declared component list.
    ComponentList,
}

impl CollectionSourceKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ComponentList => "component-list",
        }
    }
}

/// Explicit materialization semantics for one declared collection source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionSourceMode {
    /// Replace current collection membership with the source membership exactly.
    Replace,
    /// Add source membership into the collection without removing existing members.
    Reconcile,
}

impl CollectionSourceMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Reconcile => "reconcile",
        }
    }
}

/// One declared component-list source for a managed collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentListCollectionSource {
    /// Explicit source materialization mode.
    pub mode: CollectionSourceMode,
    /// Canonical managed component keys declared by the source.
    pub component_keys: Vec<Box<str>>,
}

impl ComponentListCollectionSource {
    #[must_use]
    pub fn new(mode: CollectionSourceMode, component_keys: Vec<Box<str>>) -> Self {
        let canonical_component_keys = component_keys
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            mode,
            component_keys: canonical_component_keys,
        }
    }
}

/// Declared source configuration for one managed collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionSource {
    /// One explicit component-list source stored inside VENOM.
    ComponentList(ComponentListCollectionSource),
}

impl CollectionSource {
    #[must_use]
    pub const fn kind(&self) -> CollectionSourceKind {
        match self {
            Self::ComponentList(_) => CollectionSourceKind::ComponentList,
        }
    }

    #[must_use]
    pub const fn mode(&self) -> CollectionSourceMode {
        match self {
            Self::ComponentList(source) => source.mode,
        }
    }

    #[must_use]
    pub fn component_keys(&self) -> &[Box<str>] {
        match self {
            Self::ComponentList(source) => &source.component_keys,
        }
    }

    #[must_use]
    pub fn summary(&self) -> CollectionSourceSummary {
        CollectionSourceSummary {
            kind: self.kind(),
            mode: self.mode(),
            component_count: self.component_keys().len(),
        }
    }
}

/// Lean operator-facing summary of one declared collection source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionSourceSummary {
    /// Declared source kind.
    pub kind: CollectionSourceKind,
    /// Explicit source materialization mode.
    pub mode: CollectionSourceMode,
    /// Number of canonical component keys declared by the source.
    pub component_count: usize,
}

/// Observable outcome of configuring one declared collection source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigureCollectionSourceChange {
    /// The collection now has one source or the source changed.
    Configured,
    /// The exact same source already existed.
    Unchanged,
    /// The source was rejected because the collection or one declared component is unknown.
    Rejected,
}

impl ConfigureCollectionSourceChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of configuring one declared collection source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigureCollectionSourceResult {
    /// Observable state change caused by the source configuration attempt.
    pub change: ConfigureCollectionSourceChange,
    /// Source visible after the operation when the collection exists.
    pub source: Option<CollectionSource>,
}

/// Observable outcome of materializing one declared collection source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializeCollectionSourceChange {
    /// The collection membership changed according to the source semantics.
    Materialized,
    /// Materialization completed without changing membership.
    Unchanged,
    /// Materialization was rejected because the collection or source is missing, or one declared component is unknown.
    Rejected,
}

impl MaterializeCollectionSourceChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Materialized => "materialized",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of materializing one declared collection source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializeCollectionSourceResult {
    /// Observable state change caused by the materialization attempt.
    pub change: MaterializeCollectionSourceChange,
    /// Total number of members in the collection after the operation.
    pub members: usize,
    /// Number of members added by the materialization.
    pub added: usize,
    /// Number of members removed by the materialization.
    pub removed: usize,
    /// Concrete component keys added during materialization.
    pub added_component_keys: Vec<Box<str>>,
    /// Concrete component keys removed during materialization.
    pub removed_component_keys: Vec<Box<str>>,
}

/// Durable periodic scan schedule attached to one managed collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionScanSchedule {
    /// Explicit periodic cadence in minutes.
    pub cadence_minutes: u32,
    /// Freshness mode that every materialized collection scan request must use.
    pub freshness: EvidenceFreshness,
    /// Unix epoch time in milliseconds when the next scheduler pass may materialize one batch.
    pub next_due_at_unix_ms: u64,
    /// Unix epoch time in milliseconds when one scheduler pass last materialized this collection.
    pub last_materialized_at_unix_ms: Option<u64>,
    /// Number of scan commands enqueued by the last scheduler pass.
    pub last_enqueued_commands: Option<u32>,
}

/// Observable outcome of configuring one collection scan schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigureCollectionScanScheduleChange {
    /// The collection now has one schedule or its schedule changed.
    Configured,
    /// The exact same schedule already existed.
    Unchanged,
    /// The configuration was rejected because the collection is missing or invalid.
    Rejected,
}

impl ConfigureCollectionScanScheduleChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Unchanged => "unchanged",
            Self::Rejected => "rejected",
        }
    }
}

/// Result of configuring one collection scan schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigureCollectionScanScheduleResult {
    /// Observable state change caused by the schedule configuration attempt.
    pub change: ConfigureCollectionScanScheduleChange,
    /// Schedule visible after the operation when the collection exists.
    pub schedule: Option<CollectionScanSchedule>,
}

/// Operator-facing snapshot of one managed collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedCollection {
    /// Stable collection identity inside VENOM.
    pub collection_key: Box<str>,
    /// Human-readable collection name.
    pub name: Box<str>,
    /// Canonical managed component keys in the collection.
    pub component_keys: Vec<Box<str>>,
    /// Optional declared source for the collection membership.
    pub source: Option<CollectionSource>,
    /// Optional periodic collection scan schedule.
    pub scan_schedule: Option<CollectionScanSchedule>,
}

/// One immutable artifact that belongs to one managed collection through one
/// managed component membership.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CollectionScopedArtifact {
    /// Stable component identity that owns the artifact.
    pub component_key: Box<str>,
    /// Immutable artifact identity under that managed component.
    pub artifact: ArtifactRef,
}

/// Operator-facing summary of one managed collection in the release operations view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedCollectionOperationsSummary {
    /// Stable collection identity inside VENOM.
    pub collection_key: Box<str>,
    /// Human-readable collection name.
    pub name: Box<str>,
    /// Total number of managed members currently in the collection.
    pub members: usize,
    /// Optional lean summary of the declared source.
    pub source: Option<CollectionSourceSummary>,
    /// Optional periodic collection scan schedule.
    pub scan_schedule: Option<CollectionScanSchedule>,
    /// Whether the collection is due for one scheduler pass now.
    pub due_now: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentRecord {
    registration: ComponentRegistration,
    artifacts: BTreeSet<ArtifactRef>,
    provider_key: Option<Box<str>>,
    context_profile_key: Option<Box<str>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectionRecord {
    registration: CollectionRegistration,
    component_keys: BTreeSet<Box<str>>,
    source: Option<CollectionSource>,
    scan_schedule: Option<CollectionScanSchedule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContextProfileRecord {
    registration: ContextProfileRegistration,
}

/// Minimal in-memory inventory of managed components and their immutable artifacts.
///
/// This inventory is intentionally small and deterministic. It gives the
/// product a trustworthy notion of "what is under management" and "which
/// artifact belongs to which component" before we wire a durable store in
/// later waves.
#[derive(Debug, Clone, Default)]
pub struct ComponentInventory {
    components: BTreeMap<Box<str>, ComponentRecord>,
    collections: BTreeMap<Box<str>, CollectionRecord>,
    context_profiles: BTreeMap<Box<str>, ContextProfileRecord>,
}

impl ComponentInventory {
    /// Register one component under management.
    #[must_use]
    pub fn register(&mut self, registration: ComponentRegistration) -> RegisterComponentResult {
        let change = match self.components.get(registration.component_key.as_ref()) {
            Some(existing) if existing.registration == registration => {
                RegisterComponentChange::Unchanged
            }
            Some(_) => RegisterComponentChange::Rejected,
            None => {
                let key = registration.component_key.clone();
                self.components.insert(
                    key,
                    ComponentRecord {
                        registration,
                        artifacts: BTreeSet::new(),
                        provider_key: None,
                        context_profile_key: None,
                    },
                );
                RegisterComponentChange::Registered
            }
        };

        RegisterComponentResult {
            change,
            managed_components: self.components.len(),
        }
    }

    /// Bind one immutable artifact to an already managed component.
    #[must_use]
    pub fn bind_artifact(
        &mut self,
        component_key: &str,
        artifact: ArtifactRef,
    ) -> BindArtifactResult {
        if self.components.iter().any(|(other_key, record)| {
            other_key.as_ref() != component_key && record.artifacts.contains(&artifact)
        }) {
            return BindArtifactResult {
                change: BindArtifactChange::Rejected,
                bound_artifacts: self.bound_artifacts(component_key),
            };
        }

        let Some(record) = self.components.get_mut(component_key) else {
            return BindArtifactResult {
                change: BindArtifactChange::Rejected,
                bound_artifacts: 0,
            };
        };

        let change = if record.artifacts.insert(artifact) {
            BindArtifactChange::Bound
        } else {
            BindArtifactChange::Unchanged
        };

        BindArtifactResult {
            change,
            bound_artifacts: record.artifacts.len(),
        }
    }

    /// Configure one finding provider runtime for a managed component.
    #[must_use]
    pub fn configure_provider(
        &mut self,
        component_key: &str,
        provider_key: impl Into<Box<str>>,
    ) -> ConfigureProviderResult {
        let provider_key = provider_key.into();
        let Some(record) = self.components.get_mut(component_key) else {
            return ConfigureProviderResult {
                change: ConfigureProviderChange::Rejected,
                provider_key: None,
            };
        };

        let change = if record.provider_key.as_deref() == Some(provider_key.as_ref()) {
            ConfigureProviderChange::Unchanged
        } else {
            record.provider_key = Some(provider_key);
            ConfigureProviderChange::Configured
        };

        ConfigureProviderResult {
            change,
            provider_key: record.provider_key.clone(),
        }
    }

    /// Register one reusable execution-context profile.
    #[must_use]
    pub fn register_context_profile(
        &mut self,
        registration: ContextProfileRegistration,
    ) -> RegisterContextProfileResult {
        let change = match self.context_profiles.get(registration.profile_key.as_ref()) {
            Some(existing) if existing.registration == registration => {
                RegisterContextProfileChange::Unchanged
            }
            Some(_) => RegisterContextProfileChange::Rejected,
            None => {
                let key = registration.profile_key.clone();
                self.context_profiles
                    .insert(key, ContextProfileRecord { registration });
                RegisterContextProfileChange::Registered
            }
        };

        RegisterContextProfileResult {
            change,
            managed_context_profiles: self.context_profiles.len(),
        }
    }

    /// Assign one managed context profile to one managed component.
    #[must_use]
    pub fn assign_context_profile(
        &mut self,
        component_key: &str,
        profile_key: &str,
    ) -> AssignContextProfileResult {
        if !self.context_profiles.contains_key(profile_key) {
            return AssignContextProfileResult {
                change: AssignContextProfileChange::Rejected,
                profile_key: None,
            };
        }

        let Some(record) = self.components.get_mut(component_key) else {
            return AssignContextProfileResult {
                change: AssignContextProfileChange::Rejected,
                profile_key: None,
            };
        };

        let change = if record.context_profile_key.as_deref() == Some(profile_key) {
            AssignContextProfileChange::Unchanged
        } else {
            record.context_profile_key = Some(profile_key.into());
            AssignContextProfileChange::Assigned
        };

        AssignContextProfileResult {
            change,
            profile_key: record.context_profile_key.clone(),
        }
    }

    /// Register one closed collection under management.
    #[must_use]
    pub fn register_collection(
        &mut self,
        registration: CollectionRegistration,
    ) -> RegisterCollectionResult {
        let change = match self.collections.get(registration.collection_key.as_ref()) {
            Some(existing) if existing.registration == registration => {
                RegisterCollectionChange::Unchanged
            }
            Some(_) => RegisterCollectionChange::Rejected,
            None => {
                let key = registration.collection_key.clone();
                self.collections.insert(
                    key,
                    CollectionRecord {
                        registration,
                        component_keys: BTreeSet::new(),
                        source: None,
                        scan_schedule: None,
                    },
                );
                RegisterCollectionChange::Created
            }
        };

        RegisterCollectionResult {
            change,
            managed_collections: self.collections.len(),
        }
    }

    /// Add one managed component to one collection.
    #[must_use]
    pub fn add_component_to_collection(
        &mut self,
        collection_key: &str,
        component_key: &str,
    ) -> AddCollectionComponentResult {
        if !self.is_managed(component_key) {
            return AddCollectionComponentResult {
                change: AddCollectionComponentChange::Rejected,
                members: self.collection_member_count(collection_key),
            };
        }

        let Some(record) = self.collections.get_mut(collection_key) else {
            return AddCollectionComponentResult {
                change: AddCollectionComponentChange::Rejected,
                members: 0,
            };
        };

        let change = if record.component_keys.insert(component_key.into()) {
            AddCollectionComponentChange::Added
        } else {
            AddCollectionComponentChange::Unchanged
        };

        AddCollectionComponentResult {
            change,
            members: record.component_keys.len(),
        }
    }

    /// Remove one managed component from one collection.
    #[must_use]
    pub fn remove_component_from_collection(
        &mut self,
        collection_key: &str,
        component_key: &str,
    ) -> RemoveCollectionComponentResult {
        let Some(record) = self.collections.get_mut(collection_key) else {
            return RemoveCollectionComponentResult {
                change: RemoveCollectionComponentChange::Rejected,
                members: 0,
            };
        };

        let change = if record.component_keys.remove(component_key) {
            RemoveCollectionComponentChange::Removed
        } else {
            RemoveCollectionComponentChange::Unchanged
        };

        RemoveCollectionComponentResult {
            change,
            members: record.component_keys.len(),
        }
    }

    /// Configure one declared source for one managed collection.
    #[must_use]
    pub fn configure_collection_source(
        &mut self,
        collection_key: &str,
        source: CollectionSource,
    ) -> ConfigureCollectionSourceResult {
        if source
            .component_keys()
            .iter()
            .any(|component_key| !self.is_managed(component_key.as_ref()))
        {
            return ConfigureCollectionSourceResult {
                change: ConfigureCollectionSourceChange::Rejected,
                source: self
                    .collections
                    .get(collection_key)
                    .and_then(|record| record.source.clone()),
            };
        }

        let Some(record) = self.collections.get_mut(collection_key) else {
            return ConfigureCollectionSourceResult {
                change: ConfigureCollectionSourceChange::Rejected,
                source: None,
            };
        };

        let change = if record.source.as_ref() == Some(&source) {
            ConfigureCollectionSourceChange::Unchanged
        } else {
            record.source = Some(source);
            ConfigureCollectionSourceChange::Configured
        };

        ConfigureCollectionSourceResult {
            change,
            source: record.source.clone(),
        }
    }

    /// Materialize one configured collection source into collection membership.
    #[must_use]
    pub fn materialize_collection_source(
        &mut self,
        collection_key: &str,
    ) -> MaterializeCollectionSourceResult {
        let Some(record) = self.collections.get(collection_key).cloned() else {
            return MaterializeCollectionSourceResult {
                change: MaterializeCollectionSourceChange::Rejected,
                members: 0,
                added: 0,
                removed: 0,
                added_component_keys: Vec::new(),
                removed_component_keys: Vec::new(),
            };
        };

        let Some(source) = record.source else {
            return MaterializeCollectionSourceResult {
                change: MaterializeCollectionSourceChange::Rejected,
                members: record.component_keys.len(),
                added: 0,
                removed: 0,
                added_component_keys: Vec::new(),
                removed_component_keys: Vec::new(),
            };
        };

        if source
            .component_keys()
            .iter()
            .any(|component_key| !self.is_managed(component_key.as_ref()))
        {
            return MaterializeCollectionSourceResult {
                change: MaterializeCollectionSourceChange::Rejected,
                members: record.component_keys.len(),
                added: 0,
                removed: 0,
                added_component_keys: Vec::new(),
                removed_component_keys: Vec::new(),
            };
        }

        let target_component_keys = source
            .component_keys()
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut next_component_keys = record.component_keys.clone();

        let added_component_keys = target_component_keys
            .difference(&record.component_keys)
            .cloned()
            .collect::<Vec<_>>();
        let removed_component_keys = match source.mode() {
            CollectionSourceMode::Replace => record
                .component_keys
                .difference(&target_component_keys)
                .cloned()
                .collect::<Vec<_>>(),
            CollectionSourceMode::Reconcile => Vec::new(),
        };

        match source.mode() {
            CollectionSourceMode::Replace => {
                next_component_keys = target_component_keys;
            }
            CollectionSourceMode::Reconcile => {
                next_component_keys.extend(added_component_keys.iter().cloned());
            }
        }

        let change = if added_component_keys.is_empty() && removed_component_keys.is_empty() {
            MaterializeCollectionSourceChange::Unchanged
        } else {
            self.collections
                .get_mut(collection_key)
                .expect("collection existence checked above")
                .component_keys = next_component_keys;
            MaterializeCollectionSourceChange::Materialized
        };

        let members = self.collection_member_count(collection_key);
        MaterializeCollectionSourceResult {
            change,
            members,
            added: added_component_keys.len(),
            removed: removed_component_keys.len(),
            added_component_keys,
            removed_component_keys,
        }
    }

    /// Configure one periodic collection scan schedule.
    #[must_use]
    pub fn configure_collection_scan_schedule(
        &mut self,
        collection_key: &str,
        cadence_minutes: u32,
        freshness: EvidenceFreshness,
        next_due_at_unix_ms: u64,
    ) -> ConfigureCollectionScanScheduleResult {
        if cadence_minutes == 0 {
            return ConfigureCollectionScanScheduleResult {
                change: ConfigureCollectionScanScheduleChange::Rejected,
                schedule: None,
            };
        }

        let Some(record) = self.collections.get_mut(collection_key) else {
            return ConfigureCollectionScanScheduleResult {
                change: ConfigureCollectionScanScheduleChange::Rejected,
                schedule: None,
            };
        };

        let prior_run = record.scan_schedule.map_or((None, None), |schedule| {
            (
                schedule.last_materialized_at_unix_ms,
                schedule.last_enqueued_commands,
            )
        });
        let schedule = CollectionScanSchedule {
            cadence_minutes,
            freshness,
            next_due_at_unix_ms,
            last_materialized_at_unix_ms: prior_run.0,
            last_enqueued_commands: prior_run.1,
        };
        let change = if record.scan_schedule == Some(schedule) {
            ConfigureCollectionScanScheduleChange::Unchanged
        } else {
            record.scan_schedule = Some(schedule);
            ConfigureCollectionScanScheduleChange::Configured
        };

        ConfigureCollectionScanScheduleResult {
            change,
            schedule: record.scan_schedule,
        }
    }

    #[must_use]
    pub fn record_collection_scan_materialization(
        &mut self,
        collection_key: &str,
        next_due_at_unix_ms: u64,
        materialized_at_unix_ms: u64,
        enqueued_commands: u32,
    ) -> ConfigureCollectionScanScheduleResult {
        let Some(record) = self.collections.get_mut(collection_key) else {
            return ConfigureCollectionScanScheduleResult {
                change: ConfigureCollectionScanScheduleChange::Rejected,
                schedule: None,
            };
        };

        let Some(existing_schedule) = record.scan_schedule else {
            return ConfigureCollectionScanScheduleResult {
                change: ConfigureCollectionScanScheduleChange::Rejected,
                schedule: None,
            };
        };

        let schedule = CollectionScanSchedule {
            cadence_minutes: existing_schedule.cadence_minutes,
            freshness: existing_schedule.freshness,
            next_due_at_unix_ms,
            last_materialized_at_unix_ms: Some(materialized_at_unix_ms),
            last_enqueued_commands: Some(enqueued_commands),
        };

        let change = if record.scan_schedule == Some(schedule) {
            ConfigureCollectionScanScheduleChange::Unchanged
        } else {
            record.scan_schedule = Some(schedule);
            ConfigureCollectionScanScheduleChange::Configured
        };

        ConfigureCollectionScanScheduleResult {
            change,
            schedule: record.scan_schedule,
        }
    }

    #[must_use]
    pub fn is_managed(&self, component_key: &str) -> bool {
        self.components.contains_key(component_key)
    }

    #[must_use]
    pub fn component_owns_artifact(&self, component_key: &str, artifact: &ArtifactRef) -> bool {
        self.components
            .get(component_key)
            .is_some_and(|record| record.artifacts.contains(artifact))
    }

    #[must_use]
    pub fn managed_components(&self) -> usize {
        self.components.len()
    }

    #[must_use]
    pub fn bound_artifacts(&self, component_key: &str) -> usize {
        self.components
            .get(component_key)
            .map_or(0, |record| record.artifacts.len())
    }

    #[must_use]
    pub fn configured_provider(&self, component_key: &str) -> Option<&str> {
        self.components
            .get(component_key)
            .and_then(|record| record.provider_key.as_deref())
    }

    #[must_use]
    pub fn assigned_context_profile(&self, component_key: &str) -> Option<&str> {
        self.components
            .get(component_key)
            .and_then(|record| record.context_profile_key.as_deref())
    }

    #[must_use]
    pub fn managed_component_context_profile(
        &self,
        component_key: &str,
    ) -> Option<ManagedContextProfile> {
        self.assigned_context_profile(component_key)
            .and_then(|profile_key| self.context_profile(profile_key))
    }

    #[must_use]
    pub fn managed_context_profiles(&self) -> usize {
        self.context_profiles.len()
    }

    #[must_use]
    pub fn context_profiles(&self) -> Vec<ManagedContextProfile> {
        self.context_profiles
            .values()
            .map(|record| ManagedContextProfile {
                profile_key: record.registration.profile_key.clone(),
                name: record.registration.name.clone(),
                internet_exposed: record.registration.internet_exposed,
                production: record.registration.production,
                mission_critical: record.registration.mission_critical,
            })
            .collect()
    }

    #[must_use]
    pub fn context_profile(&self, profile_key: &str) -> Option<ManagedContextProfile> {
        self.context_profiles
            .get(profile_key)
            .map(|record| ManagedContextProfile {
                profile_key: record.registration.profile_key.clone(),
                name: record.registration.name.clone(),
                internet_exposed: record.registration.internet_exposed,
                production: record.registration.production,
                mission_critical: record.registration.mission_critical,
            })
    }

    #[must_use]
    pub fn bound_artifact_refs(&self, component_key: &str) -> Option<Vec<ArtifactRef>> {
        self.components.get(component_key).map(|record| {
            record
                .artifacts
                .iter()
                .cloned()
                .collect::<Vec<ArtifactRef>>()
        })
    }

    #[must_use]
    pub fn is_collection_managed(&self, collection_key: &str) -> bool {
        self.collections.contains_key(collection_key)
    }

    #[must_use]
    pub fn managed_collections(&self) -> usize {
        self.collections.len()
    }

    #[must_use]
    pub fn collection_member_count(&self, collection_key: &str) -> usize {
        self.collections
            .get(collection_key)
            .map_or(0, |record| record.component_keys.len())
    }

    #[must_use]
    pub fn collection_members(&self, collection_key: &str) -> Option<Vec<Box<str>>> {
        self.collections.get(collection_key).map(|record| {
            record
                .component_keys
                .iter()
                .cloned()
                .collect::<Vec<Box<str>>>()
        })
    }

    #[must_use]
    pub fn collection_scoped_artifacts(
        &self,
        collection_key: &str,
    ) -> Option<Vec<CollectionScopedArtifact>> {
        self.collections.get(collection_key).map(|record| {
            record
                .component_keys
                .iter()
                .flat_map(|component_key| {
                    self.components
                        .get(component_key.as_ref())
                        .into_iter()
                        .flat_map(move |component| {
                            component.artifacts.iter().cloned().map(move |artifact| {
                                CollectionScopedArtifact {
                                    component_key: component_key.clone(),
                                    artifact,
                                }
                            })
                        })
                })
                .collect::<Vec<_>>()
        })
    }

    #[must_use]
    pub fn collection_scan_schedule(&self, collection_key: &str) -> Option<CollectionScanSchedule> {
        self.collections
            .get(collection_key)
            .and_then(|record| record.scan_schedule)
    }

    #[must_use]
    pub fn collection_source(&self, collection_key: &str) -> Option<CollectionSource> {
        self.collections
            .get(collection_key)
            .and_then(|record| record.source.clone())
    }

    #[must_use]
    pub fn due_collection_keys(&self, now_unix_ms: u64, limit: usize) -> Vec<Box<str>> {
        if limit == 0 {
            return Vec::new();
        }

        let mut due = self
            .collections
            .iter()
            .filter_map(|(collection_key, record)| {
                record.scan_schedule.and_then(|schedule| {
                    (schedule.next_due_at_unix_ms <= now_unix_ms)
                        .then_some((schedule.next_due_at_unix_ms, collection_key.clone()))
                })
            })
            .collect::<Vec<_>>();
        due.sort_by(|(left_due, left_key), (right_due, right_key)| {
            left_due
                .cmp(right_due)
                .then_with(|| left_key.as_ref().cmp(right_key.as_ref()))
        });
        due.into_iter()
            .take(limit)
            .map(|(_, collection_key)| collection_key)
            .collect()
    }

    #[must_use]
    pub fn collections(&self) -> Vec<ManagedCollection> {
        self.collections
            .values()
            .map(|record| ManagedCollection {
                collection_key: record.registration.collection_key.clone(),
                name: record.registration.name.clone(),
                component_keys: record.component_keys.iter().cloned().collect(),
                source: record.source.clone(),
                scan_schedule: record.scan_schedule,
            })
            .collect()
    }

    #[must_use]
    pub fn collection_operations_summaries(
        &self,
        now_unix_ms: u64,
    ) -> Vec<ManagedCollectionOperationsSummary> {
        let mut summaries = self
            .collections
            .values()
            .map(|record| {
                let scan_schedule = record.scan_schedule;
                let due_now = scan_schedule
                    .is_some_and(|schedule| schedule.next_due_at_unix_ms <= now_unix_ms);

                ManagedCollectionOperationsSummary {
                    collection_key: record.registration.collection_key.clone(),
                    name: record.registration.name.clone(),
                    members: record.component_keys.len(),
                    source: record.source.as_ref().map(CollectionSource::summary),
                    scan_schedule,
                    due_now,
                }
            })
            .collect::<Vec<_>>();

        summaries.sort_by(|left, right| {
            let left_due = left
                .scan_schedule
                .map(|schedule| schedule.next_due_at_unix_ms);
            let right_due = right
                .scan_schedule
                .map(|schedule| schedule.next_due_at_unix_ms);
            match (left_due, right_due) {
                (Some(left_due), Some(right_due)) => left_due.cmp(&right_due).then_with(|| {
                    left.collection_key
                        .as_ref()
                        .cmp(right.collection_key.as_ref())
                }),
                (Some(_), None) => core::cmp::Ordering::Less,
                (None, Some(_)) => core::cmp::Ordering::Greater,
                (None, None) => left
                    .collection_key
                    .as_ref()
                    .cmp(right.collection_key.as_ref()),
            }
        });

        summaries
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AddCollectionComponentChange, BindArtifactChange, CollectionRegistration,
        CollectionSource, CollectionSourceMode, ComponentInventory, ComponentListCollectionSource,
        ComponentRegistration, ConfigureCollectionScanScheduleChange,
        ConfigureCollectionSourceChange, ConfigureProviderChange, ContextProfileRegistration,
        MaterializeCollectionSourceChange, RegisterCollectionChange, RegisterComponentChange,
        RegisterContextProfileChange, RemoveCollectionComponentChange,
    };
    use crate::{ArtifactKind, ArtifactRef, EvidenceFreshness};

    fn artifact(identity: &str) -> ArtifactRef {
        ArtifactRef::new(ArtifactKind::ContainerImage, identity)
    }

    #[test]
    fn new_component_is_registered() {
        let mut inventory = ComponentInventory::default();

        let result = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));

        assert_eq!(result.change, RegisterComponentChange::Registered);
        assert_eq!(result.managed_components, 1);
        assert!(inventory.is_managed("component:payments-api"));
    }

    #[test]
    fn same_registration_is_idempotent() {
        let mut inventory = ComponentInventory::default();
        let registration = ComponentRegistration::new("component:payments-api", "Payments API");

        let _ = inventory.register(registration.clone());
        let result = inventory.register(registration);

        assert_eq!(result.change, RegisterComponentChange::Unchanged);
        assert_eq!(result.managed_components, 1);
    }

    #[test]
    fn conflicting_registration_is_rejected() {
        let mut inventory = ComponentInventory::default();

        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let result = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Billing API",
        ));

        assert_eq!(result.change, RegisterComponentChange::Rejected);
        assert_eq!(result.managed_components, 1);
    }

    #[test]
    fn managed_component_can_bind_an_artifact() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));

        let result = inventory.bind_artifact(
            "component:payments-api",
            artifact("registry.example/payments@sha256:111"),
        );

        assert_eq!(result.change, BindArtifactChange::Bound);
        assert_eq!(result.bound_artifacts, 1);
        assert!(inventory.component_owns_artifact(
            "component:payments-api",
            &artifact("registry.example/payments@sha256:111"),
        ));
    }

    #[test]
    fn repeated_artifact_binding_is_idempotent() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let artifact = artifact("registry.example/payments@sha256:111");

        let _ = inventory.bind_artifact("component:payments-api", artifact.clone());
        let result = inventory.bind_artifact("component:payments-api", artifact);

        assert_eq!(result.change, BindArtifactChange::Unchanged);
        assert_eq!(result.bound_artifacts, 1);
    }

    #[test]
    fn artifact_cannot_be_bound_to_two_components() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.register(ComponentRegistration::new(
            "component:billing-api",
            "Billing API",
        ));
        let artifact = artifact("registry.example/shared@sha256:111");

        let _ = inventory.bind_artifact("component:payments-api", artifact.clone());
        let result = inventory.bind_artifact("component:billing-api", artifact);

        assert_eq!(result.change, BindArtifactChange::Rejected);
        assert_eq!(result.bound_artifacts, 0);
    }

    #[test]
    fn managed_component_can_configure_one_provider() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));

        let result = inventory.configure_provider("component:payments-api", "fixture-provider");

        assert_eq!(result.change, ConfigureProviderChange::Configured);
        assert_eq!(result.provider_key.as_deref(), Some("fixture-provider"));
        assert_eq!(
            inventory.configured_provider("component:payments-api"),
            Some("fixture-provider")
        );
    }

    #[test]
    fn repeated_provider_configuration_is_idempotent() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.configure_provider("component:payments-api", "fixture-provider");

        let result = inventory.configure_provider("component:payments-api", "fixture-provider");

        assert_eq!(result.change, ConfigureProviderChange::Unchanged);
        assert_eq!(result.provider_key.as_deref(), Some("fixture-provider"));
    }

    #[test]
    fn unknown_component_cannot_configure_one_provider() {
        let mut inventory = ComponentInventory::default();

        let result = inventory.configure_provider("component:payments-api", "fixture-provider");

        assert_eq!(result.change, ConfigureProviderChange::Rejected);
        assert!(result.provider_key.is_none());
    }

    #[test]
    fn new_context_profile_is_registered() {
        let mut inventory = ComponentInventory::default();

        let result = inventory.register_context_profile(ContextProfileRegistration::new(
            "context:internet-prod",
            "Internet Production",
            true,
            true,
            true,
        ));

        assert_eq!(result.change, RegisterContextProfileChange::Registered);
        assert_eq!(result.managed_context_profiles, 1);
        assert_eq!(
            inventory.assigned_context_profile("component:payments-api"),
            None
        );
        assert_eq!(
            inventory
                .context_profile("context:internet-prod")
                .expect("profile should exist")
                .name
                .as_ref(),
            "Internet Production"
        );
    }

    #[test]
    fn same_context_profile_registration_is_idempotent() {
        let mut inventory = ComponentInventory::default();
        let registration = ContextProfileRegistration::new(
            "context:internet-prod",
            "Internet Production",
            true,
            true,
            true,
        );

        let _ = inventory.register_context_profile(registration.clone());
        let result = inventory.register_context_profile(registration);

        assert_eq!(result.change, RegisterContextProfileChange::Unchanged);
        assert_eq!(result.managed_context_profiles, 1);
    }

    #[test]
    fn managed_component_can_assign_one_context_profile() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.register_context_profile(ContextProfileRegistration::new(
            "context:internet-prod",
            "Internet Production",
            true,
            true,
            true,
        ));

        let result =
            inventory.assign_context_profile("component:payments-api", "context:internet-prod");

        assert_eq!(result.change.as_str(), "assigned");
        assert_eq!(result.profile_key.as_deref(), Some("context:internet-prod"));
        assert_eq!(
            inventory.assigned_context_profile("component:payments-api"),
            Some("context:internet-prod")
        );
    }

    #[test]
    fn new_collection_is_created() {
        let mut inventory = ComponentInventory::default();

        let result =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "May"));

        assert_eq!(result.change, RegisterCollectionChange::Created);
        assert_eq!(result.managed_collections, 1);
        assert!(inventory.is_collection_managed("release:2026.05"));
    }

    #[test]
    fn same_collection_creation_is_idempotent() {
        let mut inventory = ComponentInventory::default();
        let registration = CollectionRegistration::new("release:2026.05", "May");

        let _ = inventory.register_collection(registration.clone());
        let result = inventory.register_collection(registration);

        assert_eq!(result.change, RegisterCollectionChange::Unchanged);
        assert_eq!(result.managed_collections, 1);
    }

    #[test]
    fn conflicting_collection_creation_is_rejected() {
        let mut inventory = ComponentInventory::default();

        let _ =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "May"));
        let result =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "June"));

        assert_eq!(result.change, RegisterCollectionChange::Rejected);
        assert_eq!(result.managed_collections, 1);
    }

    #[test]
    fn managed_component_can_join_one_collection() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "May"));

        let result =
            inventory.add_component_to_collection("release:2026.05", "component:payments-api");

        assert_eq!(result.change, AddCollectionComponentChange::Added);
        assert_eq!(result.members, 1);
        assert_eq!(
            inventory.collection_members("release:2026.05"),
            Some(vec![Box::<str>::from("component:payments-api")])
        );
    }

    #[test]
    fn managed_collection_can_configure_one_periodic_scan_schedule() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));

        let result = inventory.configure_collection_scan_schedule(
            "release:2026.05",
            60,
            EvidenceFreshness::Deterministic,
            1_000,
        );

        assert_eq!(
            result.change,
            ConfigureCollectionScanScheduleChange::Configured
        );
        assert_eq!(
            inventory.collection_scan_schedule("release:2026.05"),
            Some(super::CollectionScanSchedule {
                cadence_minutes: 60,
                freshness: EvidenceFreshness::Deterministic,
                next_due_at_unix_ms: 1_000,
                last_materialized_at_unix_ms: None,
                last_enqueued_commands: None,
            })
        );
    }

    #[test]
    fn due_collections_are_ordered_by_due_time_then_key() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.06",
            "June Release",
        ));
        let _ = inventory.configure_collection_scan_schedule(
            "release:2026.05",
            60,
            EvidenceFreshness::Deterministic,
            2_000,
        );
        let _ = inventory.configure_collection_scan_schedule(
            "release:2026.06",
            60,
            EvidenceFreshness::Deterministic,
            1_000,
        );

        let due = inventory.due_collection_keys(2_000, 8);

        assert_eq!(
            due,
            vec![
                Box::<str>::from("release:2026.06"),
                Box::<str>::from("release:2026.05"),
            ]
        );
    }

    #[test]
    fn collection_operations_summaries_are_ordered_by_schedule_then_key() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.register(ComponentRegistration::new(
            "component:billing-api",
            "Billing API",
        ));
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.06",
            "June Release",
        ));
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.07",
            "July Release",
        ));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:payments-api");
        let _ = inventory.add_component_to_collection("release:2026.06", "component:billing-api");
        let _ = inventory.configure_collection_scan_schedule(
            "release:2026.06",
            120,
            EvidenceFreshness::Deterministic,
            2_000,
        );
        let _ = inventory.configure_collection_scan_schedule(
            "release:2026.05",
            60,
            EvidenceFreshness::Deterministic,
            1_000,
        );

        let summaries = inventory.collection_operations_summaries(1_500);

        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0].collection_key.as_ref(), "release:2026.05");
        assert_eq!(summaries[0].members, 1);
        assert!(summaries[0].due_now);
        assert_eq!(summaries[1].collection_key.as_ref(), "release:2026.06");
        assert!(!summaries[1].due_now);
        assert_eq!(summaries[2].collection_key.as_ref(), "release:2026.07");
        assert!(summaries[2].scan_schedule.is_none());
    }

    #[test]
    fn materialized_collection_schedule_keeps_last_run_metadata() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register_collection(CollectionRegistration::new(
            "release:2026.05",
            "May Release",
        ));
        let _ = inventory.configure_collection_scan_schedule(
            "release:2026.05",
            60,
            EvidenceFreshness::Deterministic,
            1_000,
        );

        let result = inventory.record_collection_scan_materialization(
            "release:2026.05",
            3_601_500,
            1_500,
            1,
        );

        assert_eq!(
            result.change,
            ConfigureCollectionScanScheduleChange::Configured
        );
        assert_eq!(
            inventory.collection_scan_schedule("release:2026.05"),
            Some(super::CollectionScanSchedule {
                cadence_minutes: 60,
                freshness: EvidenceFreshness::Deterministic,
                next_due_at_unix_ms: 3_601_500,
                last_materialized_at_unix_ms: Some(1_500),
                last_enqueued_commands: Some(1),
            })
        );
    }

    #[test]
    fn unmanaged_component_cannot_join_one_collection() {
        let mut inventory = ComponentInventory::default();
        let _ =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "May"));

        let result =
            inventory.add_component_to_collection("release:2026.05", "component:payments-api");

        assert_eq!(result.change, AddCollectionComponentChange::Rejected);
        assert_eq!(result.members, 0);
    }

    #[test]
    fn collection_member_can_be_removed() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "May"));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:payments-api");

        let result =
            inventory.remove_component_from_collection("release:2026.05", "component:payments-api");

        assert_eq!(result.change, RemoveCollectionComponentChange::Removed);
        assert_eq!(result.members, 0);
    }

    #[test]
    fn collection_source_can_replace_membership() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.register(ComponentRegistration::new(
            "component:billing-api",
            "Billing API",
        ));
        let _ =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "May"));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:billing-api");

        let source_result = inventory.configure_collection_source(
            "release:2026.05",
            CollectionSource::ComponentList(ComponentListCollectionSource::new(
                CollectionSourceMode::Replace,
                vec![Box::<str>::from("component:payments-api")],
            )),
        );
        let result = inventory.materialize_collection_source("release:2026.05");

        assert_eq!(source_result.change, ConfigureCollectionSourceChange::Configured);
        assert_eq!(result.change, MaterializeCollectionSourceChange::Materialized);
        assert_eq!(result.added, 1);
        assert_eq!(result.removed, 1);
        assert_eq!(
            inventory.collection_members("release:2026.05"),
            Some(vec![Box::<str>::from("component:payments-api")])
        );
    }

    #[test]
    fn collection_source_can_reconcile_without_removing_membership() {
        let mut inventory = ComponentInventory::default();
        let _ = inventory.register(ComponentRegistration::new(
            "component:payments-api",
            "Payments API",
        ));
        let _ = inventory.register(ComponentRegistration::new(
            "component:billing-api",
            "Billing API",
        ));
        let _ =
            inventory.register_collection(CollectionRegistration::new("release:2026.05", "May"));
        let _ = inventory.add_component_to_collection("release:2026.05", "component:billing-api");
        let _ = inventory.configure_collection_source(
            "release:2026.05",
            CollectionSource::ComponentList(ComponentListCollectionSource::new(
                CollectionSourceMode::Reconcile,
                vec![Box::<str>::from("component:payments-api")],
            )),
        );

        let result = inventory.materialize_collection_source("release:2026.05");

        assert_eq!(result.change, MaterializeCollectionSourceChange::Materialized);
        assert_eq!(result.added, 1);
        assert_eq!(result.removed, 0);
        assert_eq!(
            inventory.collection_members("release:2026.05"),
            Some(vec![
                Box::<str>::from("component:billing-api"),
                Box::<str>::from("component:payments-api"),
            ])
        );
    }
}
