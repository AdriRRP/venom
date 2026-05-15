use crate::ArtifactRef;
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentRecord {
    registration: ComponentRegistration,
    artifacts: BTreeSet<ArtifactRef>,
    provider_key: Option<Box<str>>,
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
}

#[cfg(test)]
mod tests {
    use super::{
        BindArtifactChange, ComponentInventory, ComponentRegistration, ConfigureProviderChange,
        RegisterComponentChange,
    };
    use crate::{ArtifactKind, ArtifactRef};

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
}
