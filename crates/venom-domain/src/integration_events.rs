use crate::{ArtifactRef, EvidenceFreshness, FindingChangeSet};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Durable unpublished integration event owned by VENOM.
///
/// The event identity is stable for downstream idempotency. The event payload
/// stays canonical and compact so the core is not shaped by broker envelopes or
/// provider payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingIntegrationEvent {
    pub event_id: Box<str>,
    pub event: IntegrationEvent,
}

impl PendingIntegrationEvent {
    #[must_use]
    pub fn finding_changes_observed(
        component_key: impl Into<Box<str>>,
        artifact: ArtifactRef,
        provider_key: impl Into<Box<str>>,
        freshness: EvidenceFreshness,
        observed_at: SystemTime,
        change_set: FindingChangeSet,
    ) -> Self {
        Self {
            event_id: next_integration_event_id("finding-changes"),
            event: IntegrationEvent::FindingChangesObserved {
                component_key: component_key.into(),
                artifact,
                provider_key: provider_key.into(),
                freshness,
                observed_at_micros: system_time_to_micros(observed_at),
                change_set,
            },
        }
    }

    #[must_use]
    pub fn scan_command_completed(
        command_id: &str,
        component_key: impl Into<Box<str>>,
        artifact: ArtifactRef,
        provider_key: impl Into<Box<str>>,
        freshness: EvidenceFreshness,
        findings_reported: usize,
        change_set: FindingChangeSet,
    ) -> Self {
        Self {
            event_id: format!("integration-event-scan-command-completed-{command_id}")
                .into_boxed_str(),
            event: IntegrationEvent::ScanCommandCompleted {
                command_id: command_id.into(),
                component_key: component_key.into(),
                artifact,
                provider_key: provider_key.into(),
                freshness,
                findings_reported,
                change_set,
            },
        }
    }
}

/// Canonical external event payload published by VENOM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum IntegrationEvent {
    FindingChangesObserved {
        component_key: Box<str>,
        artifact: ArtifactRef,
        provider_key: Box<str>,
        freshness: EvidenceFreshness,
        observed_at_micros: u64,
        change_set: FindingChangeSet,
    },
    ScanCommandCompleted {
        command_id: Box<str>,
        component_key: Box<str>,
        artifact: ArtifactRef,
        provider_key: Box<str>,
        freshness: EvidenceFreshness,
        findings_reported: usize,
        change_set: FindingChangeSet,
    },
}

impl IntegrationEvent {
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::FindingChangesObserved { .. } => "finding-changes-observed",
            Self::ScanCommandCompleted { .. } => "scan-command-completed",
        }
    }
}

/// Publisher port for durable VENOM integration events.
#[allow(async_fn_in_trait)]
pub trait IntegrationEventPublisher {
    fn publisher_key(&self) -> &'static str;

    async fn publish<'a>(
        &'a self,
        event: &'a PendingIntegrationEvent,
    ) -> Result<(), IntegrationEventPublishError>;
}

/// Explicit external publication error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationEventPublishError {
    pub retryable: bool,
    pub message: Box<str>,
}

impl IntegrationEventPublishError {
    #[must_use]
    pub fn new(retryable: bool, message: impl Into<Box<str>>) -> Self {
        Self {
            retryable,
            message: message.into(),
        }
    }
}

/// Explicit bounded publication result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishIntegrationEventsResult {
    pub attempted: usize,
    pub published: usize,
    pub pending_remaining: usize,
    pub last_failure: Option<IntegrationEventPublicationFailure>,
}

/// Last failure observed while publishing one bounded batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationEventPublicationFailure {
    pub event_id: Box<str>,
    pub retryable: bool,
    pub message: Box<str>,
}

#[must_use]
/// Build one fresh durable integration-event identity.
///
/// # Panics
///
/// Panics if the system clock is before the Unix epoch.
pub fn next_integration_event_id(prefix: &str) -> Box<str> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    format!("integration-event-{prefix}-{nanos}").into_boxed_str()
}

#[must_use]
/// Convert one system time into microseconds since the Unix epoch.
///
/// # Panics
///
/// Panics if the system time is before the Unix epoch or the resulting
/// microsecond value does not fit in `u64`.
pub fn system_time_to_micros(value: SystemTime) -> u64 {
    value
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_micros()
        .try_into()
        .expect("system time micros should fit in u64")
}
