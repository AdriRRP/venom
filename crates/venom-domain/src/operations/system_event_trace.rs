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
    retained_event_slots: BTreeMap<Box<str>, u16>,
    retained_events: Vec<Arc<SystemEvent>>,
    recent_events: Vec<u16>,
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
            retained_event_slots: BTreeMap::new(),
            retained_events: Vec::new(),
            recent_events: Vec::new(),
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
        let slot = register_retained_event(
            &mut self.retained_event_slots,
            &mut self.retained_events,
            event,
        );
        self.total += 1;
        match self.retained_events[usize::from(slot)].category() {
            SystemEventCategory::Scheduler => {
                self.scheduler_total += 1;
            }
            SystemEventCategory::Command => {
                self.command_total += 1;
            }
            SystemEventCategory::Governance => {
                self.governance_total += 1;
            }
            SystemEventCategory::Publication => {
                self.publication_total += 1;
            }
        }
        push_bounded_slot(&mut self.recent_events, slot);
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
            ..Self::from_recent_id_windows(merge_recent_events(
                &left.recent_events,
                &left.retained_events,
                &right.recent_events,
                &right.retained_events,
            ))
        }
    }

    fn from_recent_id_windows(recent_events: Vec<Arc<SystemEvent>>) -> Self {
        let mut index = Self::new();
        load_recent_window(
            &mut index.retained_event_slots,
            &mut index.retained_events,
            &mut index.recent_events,
            recent_events,
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
            retained_event_slots: BTreeMap::new(),
            retained_events: Vec::new(),
            recent_events: Vec::new(),
        };
        load_recent_window(
            &mut index.retained_event_slots,
            &mut index.retained_events,
            &mut index.recent_events,
            windows.recent_events,
        );
        index.retain_only_windowed_events();
        index
    }

    #[must_use]
    pub fn query(&self, query: &SystemEventsQuery) -> SystemEventsPage {
        let limit = query.normalized_limit();
        let total = match query.category {
            None => self.total,
            Some(SystemEventCategory::Scheduler) => self.scheduler_total,
            Some(SystemEventCategory::Command) => self.command_total,
            Some(SystemEventCategory::Governance) => self.governance_total,
            Some(SystemEventCategory::Publication) => self.publication_total,
        };

        let events = self
            .recent_events
            .iter()
            .filter_map(|slot| self.retained_events.get(usize::from(*slot)).map(Arc::clone))
            .filter(|event| {
                query
                    .category
                    .is_none_or(|category| event.category() == category)
            })
            .take(limit)
            .collect::<Vec<_>>();
        SystemEventsPage {
            total,
            returned: events.len(),
            limit,
            events,
        }
    }
    fn retain_only_windowed_events(&mut self) {
        let mut retained_slots = BTreeSet::<u16>::new();
        for slot in self.recent_events.iter() {
            retained_slots.insert(*slot);
        }
        let mut remap = BTreeMap::<u16, u16>::new();
        let mut next_events = Vec::with_capacity(retained_slots.len());
        for retained_slot in retained_slots {
            let next_slot = u16::try_from(next_events.len())
                .expect("bounded retained system event windows should fit in u16 slots");
            remap.insert(retained_slot, next_slot);
            next_events.push(Arc::clone(
                self.retained_events
                    .get(usize::from(retained_slot))
                    .expect("retained system event slot should exist"),
            ));
        }
        remap_recent_slots(&mut self.recent_events, &remap);
        self.retained_event_slots = next_events
            .iter()
            .enumerate()
            .map(|(index, event)| {
                (
                    event.event_id.clone(),
                    u16::try_from(index)
                        .expect("bounded retained system event windows should fit in u16 slots"),
                )
            })
            .collect();
        self.retained_events = next_events;
    }
}

fn load_recent_window(
    retained_event_slots: &mut BTreeMap<Box<str>, u16>,
    retained_events: &mut Vec<Arc<SystemEvent>>,
    target: &mut Vec<u16>,
    events: Vec<Arc<SystemEvent>>,
) {
    for event in events {
        target.push(register_retained_event(
            retained_event_slots,
            retained_events,
            event,
        ));
    }
}

fn register_retained_event(
    retained_event_slots: &mut BTreeMap<Box<str>, u16>,
    retained_events: &mut Vec<Arc<SystemEvent>>,
    event: Arc<SystemEvent>,
) -> u16 {
    if let Some(existing) = retained_event_slots.get(event.event_id.as_ref()) {
        retained_events[usize::from(*existing)] = event;
        return *existing;
    }

    let slot =
        u16::try_from(retained_events.len()).expect("bounded retained event window should fit");
    retained_event_slots.insert(event.event_id.clone(), slot);
    retained_events.push(event);
    slot
}

fn push_bounded_slot(events: &mut Vec<u16>, slot: u16) {
    if events.len() == MAX_SYSTEM_EVENTS_LIMIT {
        events.pop();
    }
    events.insert(0, slot);
}

fn merge_recent_events(
    left_slots: &[u16],
    left_retained: &[Arc<SystemEvent>],
    right_slots: &[u16],
    right_retained: &[Arc<SystemEvent>],
) -> Vec<Arc<SystemEvent>> {
    let mut merged =
        Vec::with_capacity((left_slots.len() + right_slots.len()).min(MAX_SYSTEM_EVENTS_LIMIT));
    let mut left_index = 0;
    let mut right_index = 0;

    while merged.len() < MAX_SYSTEM_EVENTS_LIMIT
        && (left_index < left_slots.len() || right_index < right_slots.len())
    {
        let take_left = match (
            left_slots
                .get(left_index)
                .and_then(|slot| left_retained.get(usize::from(*slot))),
            right_slots
                .get(right_index)
                .and_then(|slot| right_retained.get(usize::from(*slot))),
        ) {
            (Some(left_event), Some(right_event)) => {
                compare_recent_event_order(left_event, right_event).is_lt()
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if take_left {
            merged.push(Arc::clone(
                left_retained
                    .get(usize::from(left_slots[left_index]))
                    .expect("left retained system event slot should exist"),
            ));
            left_index += 1;
        } else {
            merged.push(Arc::clone(
                right_retained
                    .get(usize::from(right_slots[right_index]))
                    .expect("right retained system event slot should exist"),
            ));
            right_index += 1;
        }
    }

    merged
}

fn remap_recent_slots(events: &mut [u16], remap: &BTreeMap<u16, u16>) {
    for slot in events.iter_mut() {
        *slot = *remap
            .get(slot)
            .expect("windowed retained system event slot should remap");
    }
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
