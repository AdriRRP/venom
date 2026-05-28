use std::collections::HashSet;
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
    pub fn new() -> Self {
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
    retained_event_ids: HashSet<Box<str>>,
    retained_events: Vec<Arc<SystemEvent>>,
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
    pub fn new() -> Self {
        Self {
            total: 0,
            scheduler_total: 0,
            command_total: 0,
            governance_total: 0,
            publication_total: 0,
            retained_event_ids: HashSet::new(),
            retained_events: Vec::new(),
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
        if self.retained_event_ids.contains(event.event_id.as_ref()) {
            return;
        }
        self.total += 1;
        match event.category() {
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
        self.retained_events.insert(0, event);
        self.trim_retained_events();
    }

    #[must_use]
    pub fn merged(left: &Self, right: &Self) -> Self {
        Self::from_retained_events(
            merge_window_totals(&left.window_totals(), &right.window_totals()),
            merge_recent_arc_events(&left.retained_events, &right.retained_events),
        )
    }

    #[must_use]
    pub fn delta_since(&self, base: &Self) -> Option<Self> {
        let totals = self.window_totals();
        let base_totals = base.window_totals();
        if totals.total < base_totals.total
            || totals.scheduler_total < base_totals.scheduler_total
            || totals.command_total < base_totals.command_total
            || totals.governance_total < base_totals.governance_total
            || totals.publication_total < base_totals.publication_total
        {
            return None;
        }

        let windows = self.recent_windows();
        let base_windows = base.recent_windows();
        Some(Self::from_recent_windows(
            SystemEventWindowTotals {
                total: totals.total - base_totals.total,
                scheduler_total: totals.scheduler_total - base_totals.scheduler_total,
                command_total: totals.command_total - base_totals.command_total,
                governance_total: totals.governance_total - base_totals.governance_total,
                publication_total: totals.publication_total - base_totals.publication_total,
            },
            SystemEventRecentWindows {
                recent_events: newer_prefix_since(
                    &windows.recent_events,
                    &base_windows.recent_events,
                )?,
                recent_scheduler_events: newer_prefix_since(
                    &windows.recent_scheduler_events,
                    &base_windows.recent_scheduler_events,
                )?,
                recent_command_events: newer_prefix_since(
                    &windows.recent_command_events,
                    &base_windows.recent_command_events,
                )?,
                recent_governance_events: newer_prefix_since(
                    &windows.recent_governance_events,
                    &base_windows.recent_governance_events,
                )?,
                recent_publication_events: newer_prefix_since(
                    &windows.recent_publication_events,
                    &base_windows.recent_publication_events,
                )?,
            },
        ))
    }

    #[must_use]
    pub const fn window_totals(&self) -> SystemEventWindowTotals {
        SystemEventWindowTotals {
            total: self.total,
            scheduler_total: self.scheduler_total,
            command_total: self.command_total,
            governance_total: self.governance_total,
            publication_total: self.publication_total,
        }
    }

    #[must_use]
    pub fn recent_windows(&self) -> SystemEventRecentWindows {
        SystemEventRecentWindows {
            recent_events: self
                .retained_events
                .iter()
                .take(MAX_SYSTEM_EVENTS_LIMIT)
                .cloned()
                .collect(),
            recent_scheduler_events: self
                .retained_events
                .iter()
                .filter(|event| event.category() == SystemEventCategory::Scheduler)
                .take(MAX_SYSTEM_EVENTS_LIMIT)
                .cloned()
                .collect(),
            recent_command_events: self
                .retained_events
                .iter()
                .filter(|event| event.category() == SystemEventCategory::Command)
                .take(MAX_SYSTEM_EVENTS_LIMIT)
                .cloned()
                .collect(),
            recent_governance_events: self
                .retained_events
                .iter()
                .filter(|event| event.category() == SystemEventCategory::Governance)
                .take(MAX_SYSTEM_EVENTS_LIMIT)
                .cloned()
                .collect(),
            recent_publication_events: self
                .retained_events
                .iter()
                .filter(|event| event.category() == SystemEventCategory::Publication)
                .take(MAX_SYSTEM_EVENTS_LIMIT)
                .cloned()
                .collect(),
        }
    }

    #[must_use]
    pub fn from_recent_windows(
        totals: SystemEventWindowTotals,
        windows: SystemEventRecentWindows,
    ) -> Self {
        let retained_events = merge_recent_windows_into_retained(windows);
        Self::from_retained_events(totals, retained_events)
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

        let events = query.category.map_or_else(
            || {
                self.retained_events
                    .iter()
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
            },
            |category| {
                self.retained_events
                    .iter()
                    .filter(|event| event.category() == category)
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
            },
        );
        SystemEventsPage {
            total,
            returned: events.len(),
            limit,
            events,
        }
    }

    fn from_retained_events(
        totals: SystemEventWindowTotals,
        retained_events: Vec<Arc<SystemEvent>>,
    ) -> Self {
        let mut index = Self::new();
        index.total = totals.total;
        index.scheduler_total = totals.scheduler_total;
        index.command_total = totals.command_total;
        index.governance_total = totals.governance_total;
        index.publication_total = totals.publication_total;
        index.retained_events = retained_events;
        index.trim_retained_events();
        index
    }

    fn trim_retained_events(&mut self) {
        let mut global = 0usize;
        let mut scheduler = 0usize;
        let mut command = 0usize;
        let mut governance = 0usize;
        let mut publication = 0usize;
        let mut retained = Vec::new();
        for event in &self.retained_events {
            let category_count = match event.category() {
                SystemEventCategory::Scheduler => &mut scheduler,
                SystemEventCategory::Command => &mut command,
                SystemEventCategory::Governance => &mut governance,
                SystemEventCategory::Publication => &mut publication,
            };
            let keep =
                global < MAX_SYSTEM_EVENTS_LIMIT || *category_count < MAX_SYSTEM_EVENTS_LIMIT;
            if keep {
                retained.push(Arc::clone(event));
                global += 1;
                *category_count += 1;
            }
        }
        self.retained_events = retained;
        self.retained_event_ids = self
            .retained_events
            .iter()
            .map(|event| event.event_id.clone())
            .collect();
    }
}

fn newer_prefix_since(
    current: &[Arc<SystemEvent>],
    base: &[Arc<SystemEvent>],
) -> Option<Vec<Arc<SystemEvent>>> {
    if base.len() > current.len() {
        return None;
    }
    let suffix_start = current.len() - base.len();
    let suffix = &current[suffix_start..];
    if !suffix.iter().zip(base.iter()).all(|(left, right)| {
        left.event_id == right.event_id && left.occurred_at_unix_ms == right.occurred_at_unix_ms
    }) {
        return None;
    }
    Some(current[..suffix_start].to_vec())
}

const fn merge_window_totals(
    left: &SystemEventWindowTotals,
    right: &SystemEventWindowTotals,
) -> SystemEventWindowTotals {
    SystemEventWindowTotals {
        total: left.total + right.total,
        scheduler_total: left.scheduler_total + right.scheduler_total,
        command_total: left.command_total + right.command_total,
        governance_total: left.governance_total + right.governance_total,
        publication_total: left.publication_total + right.publication_total,
    }
}

fn merge_recent_arc_events(
    left: &[Arc<SystemEvent>],
    right: &[Arc<SystemEvent>],
) -> Vec<Arc<SystemEvent>> {
    let mut merged = Vec::with_capacity((left.len() + right.len()).min(MAX_SYSTEM_EVENTS_LIMIT));
    let mut left_index = 0;
    let mut right_index = 0;

    while merged.len() < MAX_SYSTEM_EVENTS_LIMIT
        && (left_index < left.len() || right_index < right.len())
    {
        let take_left = match (left.get(left_index), right.get(right_index)) {
            (Some(left_event), Some(right_event)) => {
                compare_recent_event_order(left_event, right_event).is_lt()
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if take_left {
            merged.push(Arc::clone(
                left.get(left_index)
                    .expect("left recent event should exist for merge"),
            ));
            left_index += 1;
        } else {
            merged.push(Arc::clone(
                right
                    .get(right_index)
                    .expect("right recent event should exist for merge"),
            ));
            right_index += 1;
        }
    }

    merged
}

fn merge_recent_windows_into_retained(windows: SystemEventRecentWindows) -> Vec<Arc<SystemEvent>> {
    let mut retained = windows.recent_events;
    retained.extend(windows.recent_scheduler_events);
    retained.extend(windows.recent_command_events);
    retained.extend(windows.recent_governance_events);
    retained.extend(windows.recent_publication_events);
    retained.sort_by(|left, right| compare_recent_event_order(left, right));
    retained.dedup_by(|left, right| left.event_id == right.event_id);
    retained
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
        SystemEventKind, SystemEventQueryIndex, SystemEventRecentWindows, SystemEventWindowTotals,
        SystemEventsQuery, query_system_events,
    };
    use std::sync::Arc;

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

    #[test]
    fn category_query_uses_category_recent_window_not_only_global_recent_window() {
        let index = SystemEventQueryIndex::from_recent_windows(
            SystemEventWindowTotals {
                total: 201,
                scheduler_total: 1,
                command_total: 200,
                governance_total: 0,
                publication_total: 0,
            },
            SystemEventRecentWindows {
                recent_events: (0..MAX_SYSTEM_EVENTS_LIMIT)
                    .map(|index| {
                        Arc::new(timed_event(
                            &format!("command-{index:03}"),
                            1_000 + u64::try_from(index).expect("index should fit"),
                            SystemEventKind::ScanCommandCompleted,
                        ))
                    })
                    .collect(),
                recent_scheduler_events: vec![Arc::new(timed_event(
                    "scheduler-001",
                    999,
                    SystemEventKind::CollectionScanMaterialized,
                ))],
                recent_command_events: Vec::new(),
                recent_governance_events: Vec::new(),
                recent_publication_events: Vec::new(),
            },
        );

        let page = index.query(
            &SystemEventsQuery::new()
                .with_category(SystemEventCategory::Scheduler)
                .with_limit(5),
        );

        assert_eq!(page.total, 1);
        assert_eq!(page.returned, 1);
        assert_eq!(page.events[0].event_id.as_ref(), "scheduler-001");
    }

    #[test]
    fn system_event_query_index_keeps_truthful_recent_category_pages_without_category_slot_topology()
     {
        category_query_uses_category_recent_window_not_only_global_recent_window();
    }
}
