use serde::{Deserialize, Serialize};

/// Durable system-level integration publication runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationRuntimeConfig {
    Fixture,
    Http {
        endpoint_url: Box<str>,
        timeout_ms: u32,
    },
}

impl IntegrationRuntimeConfig {
    #[must_use]
    pub const fn publisher_key(&self) -> &'static str {
        match self {
            Self::Fixture => "fixture-publisher",
            Self::Http { .. } => "http-publisher",
        }
    }

    #[must_use]
    pub fn endpoint_url(&self) -> Option<&str> {
        match self {
            Self::Fixture => None,
            Self::Http { endpoint_url, .. } => Some(endpoint_url.as_ref()),
        }
    }

    #[must_use]
    pub const fn timeout_ms(&self) -> Option<u32> {
        match self {
            Self::Fixture => None,
            Self::Http { timeout_ms, .. } => Some(*timeout_ms),
        }
    }
}

/// Observable outcome of configuring the integration publication runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigureIntegrationRuntimeChange {
    Configured,
    Unchanged,
}

impl ConfigureIntegrationRuntimeChange {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Unchanged => "unchanged",
        }
    }
}

/// Result of one integration runtime configuration command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigureIntegrationRuntimeResult {
    pub change: ConfigureIntegrationRuntimeChange,
    pub config: IntegrationRuntimeConfig,
}
