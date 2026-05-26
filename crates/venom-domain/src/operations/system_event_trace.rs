use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
    pub events: Vec<Arc<SystemEvent>>,
}

/// One bounded, truthful query index over operator-facing system events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemEventQueryIndex {
    total: usize,
    scheduler_total: usize,
    command_total: usize,
    governance_total: usize,
    publication_total: usize,
    retained_events: BTreeMap<Box<str>, Arc<SystemEvent>>,
    recent_events: Vec<Box<str>>,
    recent_scheduler_events: Vec<Box<str>>,
    recent_command_events: Vec<Box<str>>,
    recent_governance_events: Vec<Box<str>>,
    recent_publication_events: Vec<Box<str>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemEventWindowTotals {
    pub total: usize,
    pub scheduler_total: usize,
    pub command_total: usize,
    pub governance_total: usize,
    pub publication_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SystemEventRecentWindows {
    pub recent_events: Vec<Arc<SystemEvent>>,
    pub recent_scheduler_events: Vec<Arc<SystemEvent>>,
    pub recent_command_events: Vec<Arc<SystemEvent>>,
    pub recent_governance_events: Vec<Arc<SystemEvent>>,
    pub recent_publication_events: Vec<Arc<SystemEvent>>,
}

impl Default for SystemEventQueryIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemEventQueryIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total: 0,
            scheduler_total: 0,
            command_total: 0,
            governance_total: 0,
            publication_total: 0,
            retained_events: BTreeMap::new(),
            recent_events: Vec::new(),
            recent_scheduler_events: Vec::new(),
            recent_command_events: Vec::new(),
            recent_governance_events: Vec::new(),
            recent_publication_events: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_newest_first<'a>(events: impl IntoIterator<Item = &'a SystemEvent>) -> Self {
        let mut index = Self::new();
        for event in events {
            index.push_newest_shared(Arc::new(event.clone()));
        }
        index
    }

    pub fn push_newest(&mut self, event: SystemEvent) {
        self.push_newest_shared(Arc::new(event));
    }

    fn push_newest_shared(&mut self, event: Arc<SystemEvent>) {
        let event_id = event.event_id.clone();
        self.retained_events.insert(event_id.clone(), event);
        self.total += 1;
        match self
            .retained_events
            .get(event_id.as_ref())
            .expect("retained system event should exist")
            .category()
        {
            SystemEventCategory::Scheduler => {
                self.scheduler_total += 1;
                push_bounded_id(&mut self.recent_scheduler_events, event_id.clone());
            }
            SystemEventCategory::Command => {
                self.command_total += 1;
                push_bounded_id(&mut self.recent_command_events, event_id.clone());
            }
            SystemEventCategory::Governance => {
                self.governance_total += 1;
                push_bounded_id(&mut self.recent_governance_events, event_id.clone());
            }
            SystemEventCategory::Publication => {
                self.publication_total += 1;
                push_bounded_id(&mut self.recent_publication_events, event_id.clone());
            }
        }
        push_bounded_id(&mut self.recent_events, event_id);
        self.retain_only_windowed_events();
    }

    #[must_use]
    pub fn merged(left: &Self, right: &Self) -> Self {
        Self {
            total: left.total + right.total,
            scheduler_total: left.scheduler_total + right.scheduler_total,
            command_total: left.command_total + right.command_total,
            governance_total: left.governance_total + right.governance_total,
            publication_total: left.publication_total + right.publication_total,
            ..Self::from_recent_id_windows(
                &left.retained_events,
                &right.retained_events,
                merge_recent_event_ids(
                    &left.recent_events,
                    &left.retained_events,
                    &right.recent_events,
                    &right.retained_events,
                ),
                merge_recent_event_ids(
                    &left.recent_scheduler_events,
                    &left.retained_events,
                    &right.recent_scheduler_events,
                    &right.retained_events,
                ),
                merge_recent_event_ids(
                    &left.recent_command_events,
                    &left.retained_events,
                    &right.recent_command_events,
                    &right.retained_events,
                ),
                merge_recent_event_ids(
                    &left.recent_governance_events,
                    &left.retained_events,
                    &right.recent_governance_events,
                    &right.retained_events,
                ),
                merge_recent_event_ids(
                    &left.recent_publication_events,
                    &left.retained_events,
                    &right.recent_publication_events,
                    &right.retained_events,
                ),
            )
        }
    }

    fn from_recent_id_windows(
        left_retained: &BTreeMap<Box<str>, Arc<SystemEvent>>,
        right_retained: &BTreeMap<Box<str>, Arc<SystemEvent>>,
        recent_events: Vec<Box<str>>,
        recent_scheduler_events: Vec<Box<str>>,
        recent_command_events: Vec<Box<str>>,
        recent_governance_events: Vec<Box<str>>,
        recent_publication_events: Vec<Box<str>>,
    ) -> Self {
        let mut index = Self::new();
        load_recent_id_window(
            &mut index.retained_events,
            &mut index.recent_events,
            recent_events,
            left_retained,
            right_retained,
        );
        load_recent_id_window(
            &mut index.retained_events,
            &mut index.recent_scheduler_events,
            recent_scheduler_events,
            left_retained,
            right_retained,
        );
        load_recent_id_window(
            &mut index.retained_events,
            &mut index.recent_command_events,
            recent_command_events,
            left_retained,
            right_retained,
        );
        load_recent_id_window(
            &mut index.retained_events,
            &mut index.recent_governance_events,
            recent_governance_events,
            left_retained,
            right_retained,
        );
        load_recent_id_window(
            &mut index.retained_events,
            &mut index.recent_publication_events,
            recent_publication_events,
            left_retained,
            right_retained,
        );
        index
    }

    #[must_use]
    pub fn from_recent_windows(
        totals: SystemEventWindowTotals,
        windows: SystemEventRecentWindows,
    ) -> Self {
        let mut index = Self {
            total: totals.total,
            scheduler_total: totals.scheduler_total,
            command_total: totals.command_total,
            governance_total: totals.governance_total,
            publication_total: totals.publication_total,
            retained_events: BTreeMap::new(),
            recent_events: Vec::new(),
            recent_scheduler_events: Vec::new(),
            recent_command_events: Vec::new(),
            recent_governance_events: Vec::new(),
            recent_publication_events: Vec::new(),
        };
        load_recent_window(
            &mut index.retained_events,
            &mut index.recent_events,
            windows.recent_events,
        );
        load_recent_window(
            &mut index.retained_events,
            &mut index.recent_scheduler_events,
            windows.recent_scheduler_events,
        );
        load_recent_window(
            &mut index.retained_events,
            &mut index.recent_command_events,
            windows.recent_command_events,
        );
        load_recent_window(
            &mut index.retained_events,
            &mut index.recent_governance_events,
            windows.recent_governance_events,
        );
        load_recent_window(
            &mut index.retained_events,
            &mut index.recent_publication_events,
            windows.recent_publication_events,
        );
        index.retain_only_windowed_events();
        index
    }

    #[must_use]
    pub fn query(&self, query: &SystemEventsQuery) -> SystemEventsPage {
        let limit = query.normalized_limit();
        let (total, source) = match query.category {
            None => (self.total, &self.recent_events),
            Some(SystemEventCategory::Scheduler) => {
                (self.scheduler_total, &self.recent_scheduler_events)
            }
            Some(SystemEventCategory::Command) => (self.command_total, &self.recent_command_events),
            Some(SystemEventCategory::Governance) => {
                (self.governance_total, &self.recent_governance_events)
            }
            Some(SystemEventCategory::Publication) => {
                (self.publication_total, &self.recent_publication_events)
            }
        };

        let events = source
            .iter()
            .take(limit)
            .filter_map(|event_id| self.retained_events.get(event_id.as_ref()).map(Arc::clone))
            .collect::<Vec<_>>();
        SystemEventsPage {
            total,
            returned: events.len(),
            limit,
            events,
        }
    }
    fn retain_only_windowed_events(&mut self) {
        let mut retained_ids = BTreeSet::<Box<str>>::new();
        for event_id in self
            .recent_events
            .iter()
            .chain(self.recent_scheduler_events.iter())
            .chain(self.recent_command_events.iter())
            .chain(self.recent_governance_events.iter())
            .chain(self.recent_publication_events.iter())
        {
            retained_ids.insert(event_id.clone());
        }
        self.retained_events
            .retain(|event_id, _| retained_ids.contains(event_id));
    }
}

fn load_recent_window(
    retained_events: &mut BTreeMap<Box<str>, Arc<SystemEvent>>,
    target: &mut Vec<Box<str>>,
    events: Vec<Arc<SystemEvent>>,
) {
    for event in events {
        let event_id = event.event_id.clone();
        retained_events.entry(event_id.clone()).or_insert(event);
        target.push(event_id);
    }
}

fn load_recent_id_window(
    retained_events: &mut BTreeMap<Box<str>, Arc<SystemEvent>>,
    target: &mut Vec<Box<str>>,
    event_ids: Vec<Box<str>>,
    left_retained: &BTreeMap<Box<str>, Arc<SystemEvent>>,
    right_retained: &BTreeMap<Box<str>, Arc<SystemEvent>>,
) {
    for event_id in event_ids {
        let event = left_retained
            .get(event_id.as_ref())
            .or_else(|| right_retained.get(event_id.as_ref()))
            .expect("merged recent system event id should resolve in one retained window");
        retained_events
            .entry(event_id.clone())
            .or_insert_with(|| Arc::clone(event));
        target.push(event_id);
    }
}

fn push_bounded_id(events: &mut Vec<Box<str>>, event_id: Box<str>) {
    if events.len() == MAX_SYSTEM_EVENTS_LIMIT {
        events.pop();
    }
    events.insert(0, event_id);
}

fn merge_recent_event_ids(
    left_ids: &[Box<str>],
    left_retained: &BTreeMap<Box<str>, Arc<SystemEvent>>,
    right_ids: &[Box<str>],
    right_retained: &BTreeMap<Box<str>, Arc<SystemEvent>>,
) -> Vec<Box<str>> {
    let mut merged =
        Vec::with_capacity((left_ids.len() + right_ids.len()).min(MAX_SYSTEM_EVENTS_LIMIT));
    let mut left_index = 0;
    let mut right_index = 0;

    while merged.len() < MAX_SYSTEM_EVENTS_LIMIT
        && (left_index < left_ids.len() || right_index < right_ids.len())
    {
        let take_left = match (
            left_ids
                .get(left_index)
                .and_then(|event_id| left_retained.get(event_id.as_ref())),
            right_ids
                .get(right_index)
                .and_then(|event_id| right_retained.get(event_id.as_ref())),
        ) {
            (Some(left_event), Some(right_event)) => {
                compare_recent_event_order(left_event, right_event).is_lt()
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if take_left {
            merged.push(left_ids[left_index].clone());
            left_index += 1;
        } else {
            merged.push(right_ids[right_index].clone());
            right_index += 1;
        }
    }

    merged
}

fn compare_recent_event_order(left: &SystemEvent, right: &SystemEvent) -> std::cmp::Ordering {
    right
        .occurred_at_unix_ms
        .cmp(&left.occurred_at_unix_ms)
        .then_with(|| right.event_id.cmp(&left.event_id))
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
            returned_events.push(Arc::new(event.clone()));
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
        SystemEventKind, SystemEventQueryIndex, SystemEventsQuery, query_system_events,
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

    fn timed_event(event_id: &str, occurred_at_unix_ms: u64, kind: SystemEventKind) -> SystemEvent {
        SystemEvent {
            event_id: event_id.into(),
            occurred_at_unix_ms,
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

    #[test]
    fn merged_index_keeps_recent_order_without_query_rebuilds() {
        let left = SystemEventQueryIndex::from_newest_first(
            [
                timed_event("event-001", 1, SystemEventKind::FindingRiskAccepted),
                timed_event("event-003", 3, SystemEventKind::ScanCommandCompleted),
            ]
            .iter(),
        );
        let right = SystemEventQueryIndex::from_newest_first(
            [
                timed_event("event-002", 2, SystemEventKind::FindingSuppressed),
                timed_event("event-004", 4, SystemEventKind::IntegrationEventPublished),
            ]
            .iter(),
        );

        let merged = SystemEventQueryIndex::merged(&left, &right);
        let page = merged.query(&SystemEventsQuery::new().with_limit(4));

        let ordered_ids = page
            .events
            .iter()
            .map(|event| event.event_id.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            ordered_ids,
            vec!["event-004", "event-003", "event-002", "event-001"]
        );
        assert_eq!(page.total, 4);
    }
}
