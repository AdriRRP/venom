/// Default number of recent operator-facing system events returned in one query.
pub const DEFAULT_SYSTEM_EVENTS_LIMIT: usize = 50;

/// Maximum number of recent operator-facing system events returned in one query.
pub const MAX_SYSTEM_EVENTS_LIMIT: usize = 200;

/// One operator-facing category for recent durable system traceability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemEventCategory {
    Scheduler,
    Command,
    Governance,
    Publication,
}

impl SystemEventCategory {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduler => "scheduler",
            Self::Command => "command",
            Self::Governance => "governance",
            Self::Publication => "publication",
        }
    }
}

/// One canonical operator-facing recent event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemEventKind {
    CollectionScanMaterialized,
    ScanCommandEnqueued,
    ScanCommandCompleted,
    ScanCommandFailed,
    FindingRiskAccepted,
    FindingsRiskAccepted,
    FindingSuppressed,
    FindingsSuppressed,
    FindingReopened,
    FindingsReopened,
    IntegrationEventPublished,
    IntegrationEventPublicationFailed,
}

impl SystemEventKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CollectionScanMaterialized => "collection-scan-materialized",
            Self::ScanCommandEnqueued => "scan-command-enqueued",
            Self::ScanCommandCompleted => "scan-command-completed",
            Self::ScanCommandFailed => "scan-command-failed",
            Self::FindingRiskAccepted => "finding-risk-accepted",
            Self::FindingsRiskAccepted => "findings-risk-accepted",
            Self::FindingSuppressed => "finding-suppressed",
            Self::FindingsSuppressed => "findings-suppressed",
            Self::FindingReopened => "finding-reopened",
            Self::FindingsReopened => "findings-reopened",
            Self::IntegrationEventPublished => "integration-event-published",
            Self::IntegrationEventPublicationFailed => "integration-event-publication-failed",
        }
    }

    #[must_use]
    pub const fn category(self) -> SystemEventCategory {
        match self {
            Self::CollectionScanMaterialized => SystemEventCategory::Scheduler,
            Self::ScanCommandEnqueued | Self::ScanCommandCompleted | Self::ScanCommandFailed => {
                SystemEventCategory::Command
            }
            Self::FindingRiskAccepted
            | Self::FindingsRiskAccepted
            | Self::FindingSuppressed
            | Self::FindingsSuppressed
            | Self::FindingReopened
            | Self::FindingsReopened => SystemEventCategory::Governance,
            Self::IntegrationEventPublished | Self::IntegrationEventPublicationFailed => {
                SystemEventCategory::Publication
            }
        }
    }
}

/// One recent operator-facing system event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemEvent {
    pub event_id: Box<str>,
    pub occurred_at_unix_ms: u64,
    pub kind: SystemEventKind,
    pub collection_key: Option<Box<str>>,
    pub component_key: Option<Box<str>>,
    pub command_id: Option<Box<str>>,
    pub integration_event_id: Option<Box<str>>,
    pub finding_count: Option<u32>,
    pub retryable: Option<bool>,
    pub detail: Option<Box<str>>,
}

impl SystemEvent {
    #[must_use]
    pub const fn category(&self) -> SystemEventCategory {
        self.kind.category()
    }
}

/// One bounded query over recent operator-facing system events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemEventsQuery {
    pub category: Option<SystemEventCategory>,
    pub limit: usize,
}

impl Default for SystemEventsQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemEventsQuery {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            category: None,
            limit: DEFAULT_SYSTEM_EVENTS_LIMIT,
        }
    }

    #[must_use]
    pub const fn with_category(mut self, category: SystemEventCategory) -> Self {
        self.category = Some(category);
        self
    }

    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    #[must_use]
    pub fn normalized_limit(&self) -> usize {
        self.limit.clamp(1, MAX_SYSTEM_EVENTS_LIMIT)
    }
}

/// One bounded recent timeline for operators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemEventsPage {
    pub total: usize,
    pub returned: usize,
    pub limit: usize,
    pub events: Vec<SystemEvent>,
}

#[must_use]
pub fn query_system_events<'a>(
    events: impl IntoIterator<Item = &'a SystemEvent>,
    query: &SystemEventsQuery,
) -> SystemEventsPage {
    let limit = query.normalized_limit();
    let mut total = 0;
    let mut returned_events = Vec::with_capacity(limit);

    for event in events.into_iter().filter(|event| {
        query
            .category
            .is_none_or(|category| event.category() == category)
    }) {
        total += 1;
        if returned_events.len() < limit {
            returned_events.push(event.clone());
        }
    }

    SystemEventsPage {
        total,
        returned: returned_events.len(),
        limit,
        events: returned_events,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_SYSTEM_EVENTS_LIMIT, MAX_SYSTEM_EVENTS_LIMIT, SystemEvent, SystemEventCategory,
        SystemEventKind, SystemEventsQuery, query_system_events,
    };

    fn event(event_id: &str, kind: SystemEventKind) -> SystemEvent {
        SystemEvent {
            event_id: event_id.into(),
            occurred_at_unix_ms: 1,
            kind,
            collection_key: None,
            component_key: None,
            command_id: None,
            integration_event_id: None,
            finding_count: None,
            retryable: None,
            detail: None,
        }
    }

    #[test]
    fn system_events_query_reports_total_matches_not_only_returned_events() {
        let events = [
            event("event-001", SystemEventKind::ScanCommandCompleted),
            event("event-002", SystemEventKind::ScanCommandCompleted),
            event("event-003", SystemEventKind::ScanCommandCompleted),
        ];

        let page = query_system_events(events.iter(), &SystemEventsQuery::new().with_limit(2));

        assert_eq!(page.total, 3);
        assert_eq!(page.returned, 2);
        assert_eq!(page.limit, 2);
        assert_eq!(page.events.len(), 2);
    }

    #[test]
    fn system_events_query_counts_filtered_matches_before_truncation() {
        let events = [
            event("event-001", SystemEventKind::FindingRiskAccepted),
            event("event-002", SystemEventKind::FindingSuppressed),
            event("event-003", SystemEventKind::ScanCommandCompleted),
        ];

        let page = query_system_events(
            events.iter(),
            &SystemEventsQuery::new()
                .with_category(SystemEventCategory::Governance)
                .with_limit(1),
        );

        assert_eq!(page.total, 2);
        assert_eq!(page.returned, 1);
        assert_eq!(page.events[0].category(), SystemEventCategory::Governance);
    }

    #[test]
    fn system_events_query_normalizes_large_limits() {
        let page = query_system_events(
            [event("event-001", SystemEventKind::ScanCommandCompleted)].iter(),
            &SystemEventsQuery::new().with_limit(MAX_SYSTEM_EVENTS_LIMIT + 500),
        );

        assert_eq!(page.limit, MAX_SYSTEM_EVENTS_LIMIT);
        assert_eq!(page.total, 1);
        assert_eq!(page.returned, 1);
    }

    #[test]
    fn system_events_query_uses_default_limit() {
        let query = SystemEventsQuery::new();
        assert_eq!(query.normalized_limit(), DEFAULT_SYSTEM_EVENTS_LIMIT);
    }
}
