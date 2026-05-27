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
    retained_events: Vec<RetainedSystemEvent>,
    recent_events: Vec<u16>,
    recent_scheduler_head: Option<u16>,
    recent_scheduler_len: usize,
    recent_command_head: Option<u16>,
    recent_command_len: usize,
    recent_governance_head: Option<u16>,
    recent_governance_len: usize,
    recent_publication_head: Option<u16>,
    recent_publication_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetainedSystemEvent {
    event: Arc<SystemEvent>,
    next_same_category: Option<u16>,
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
            recent_scheduler_head: None,
            recent_scheduler_len: 0,
            recent_command_head: None,
            recent_command_len: 0,
            recent_governance_head: None,
            recent_governance_len: 0,
            recent_publication_head: None,
            recent_publication_len: 0,
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
        match self.retained_events[usize::from(slot)].event.category() {
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
        self.prepend_category_slot(
            self.retained_events[usize::from(slot)].event.category(),
            slot,
        );
        self.retain_only_windowed_events();
    }

    #[must_use]
    pub fn merged(left: &Self, right: &Self) -> Self {
        Self::from_recent_windows(
            merge_window_totals(&left.window_totals(), &right.window_totals()),
            merge_recent_windows(&left.recent_windows(), &right.recent_windows()),
        )
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
                .recent_events
                .iter()
                .filter_map(|slot| {
                    self.retained_events
                        .get(usize::from(*slot))
                        .map(|entry| Arc::clone(&entry.event))
                })
                .collect(),
            recent_scheduler_events: collect_category_recent_events(
                &self.collect_category_recent_slots(SystemEventCategory::Scheduler),
                &self.retained_events,
            ),
            recent_command_events: collect_category_recent_events(
                &self.collect_category_recent_slots(SystemEventCategory::Command),
                &self.retained_events,
            ),
            recent_governance_events: collect_category_recent_events(
                &self.collect_category_recent_slots(SystemEventCategory::Governance),
                &self.retained_events,
            ),
            recent_publication_events: collect_category_recent_events(
                &self.collect_category_recent_slots(SystemEventCategory::Publication),
                &self.retained_events,
            ),
        }
    }

    #[must_use]
    pub fn from_recent_windows(
        totals: SystemEventWindowTotals,
        windows: SystemEventRecentWindows,
    ) -> Self {
        let mut index = Self::new();
        index.total = totals.total;
        index.scheduler_total = totals.scheduler_total;
        index.command_total = totals.command_total;
        index.governance_total = totals.governance_total;
        index.publication_total = totals.publication_total;
        load_recent_window(
            &mut index.retained_event_slots,
            &mut index.retained_events,
            &mut index.recent_events,
            windows.recent_events,
        );
        index.load_recent_category_window(
            SystemEventCategory::Scheduler,
            windows.recent_scheduler_events,
        );
        index.load_recent_category_window(
            SystemEventCategory::Command,
            windows.recent_command_events,
        );
        index.load_recent_category_window(
            SystemEventCategory::Governance,
            windows.recent_governance_events,
        );
        index.load_recent_category_window(
            SystemEventCategory::Publication,
            windows.recent_publication_events,
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

        let events = query.category.map_or_else(
            || {
                self.recent_events
                    .iter()
                    .filter_map(|slot| {
                        self.retained_events
                            .get(usize::from(*slot))
                            .map(|entry| Arc::clone(&entry.event))
                    })
                    .take(limit)
                    .collect::<Vec<_>>()
            },
            |category| {
                let mut events = Vec::with_capacity(limit);
                let mut cursor = self.category_recent_head(category);
                while let Some(slot) = cursor {
                    let Some(entry) = self.retained_events.get(usize::from(slot)) else {
                        break;
                    };
                    events.push(Arc::clone(&entry.event));
                    if events.len() == limit {
                        break;
                    }
                    cursor = entry.next_same_category;
                }
                events
            },
        );
        SystemEventsPage {
            total,
            returned: events.len(),
            limit,
            events,
        }
    }

    const fn category_recent_head(&self, category: SystemEventCategory) -> Option<u16> {
        match category {
            SystemEventCategory::Scheduler => self.recent_scheduler_head,
            SystemEventCategory::Command => self.recent_command_head,
            SystemEventCategory::Governance => self.recent_governance_head,
            SystemEventCategory::Publication => self.recent_publication_head,
        }
    }

    const fn category_recent_len(&self, category: SystemEventCategory) -> usize {
        match category {
            SystemEventCategory::Scheduler => self.recent_scheduler_len,
            SystemEventCategory::Command => self.recent_command_len,
            SystemEventCategory::Governance => self.recent_governance_len,
            SystemEventCategory::Publication => self.recent_publication_len,
        }
    }

    const fn set_category_recent_window(
        &mut self,
        category: SystemEventCategory,
        head: Option<u16>,
        len: usize,
    ) {
        match category {
            SystemEventCategory::Scheduler => {
                self.recent_scheduler_head = head;
                self.recent_scheduler_len = len;
            }
            SystemEventCategory::Command => {
                self.recent_command_head = head;
                self.recent_command_len = len;
            }
            SystemEventCategory::Governance => {
                self.recent_governance_head = head;
                self.recent_governance_len = len;
            }
            SystemEventCategory::Publication => {
                self.recent_publication_head = head;
                self.recent_publication_len = len;
            }
        }
    }

    fn prepend_category_slot(&mut self, category: SystemEventCategory, slot: u16) {
        let head = self.category_recent_head(category);
        self.retained_events[usize::from(slot)].next_same_category = head;
        let len = self.category_recent_len(category).saturating_add(1);
        self.set_category_recent_window(category, Some(slot), len.min(MAX_SYSTEM_EVENTS_LIMIT));
        if len > MAX_SYSTEM_EVENTS_LIMIT {
            self.trim_category_window(category);
        }
    }

    fn trim_category_window(&mut self, category: SystemEventCategory) {
        let Some(mut cursor) = self.category_recent_head(category) else {
            self.set_category_recent_window(category, None, 0);
            return;
        };
        for _ in 1..MAX_SYSTEM_EVENTS_LIMIT {
            let Some(next) = self.retained_events[usize::from(cursor)].next_same_category else {
                self.set_category_recent_window(category, Some(cursor), MAX_SYSTEM_EVENTS_LIMIT);
                return;
            };
            cursor = next;
        }
        self.retained_events[usize::from(cursor)].next_same_category = None;
        self.set_category_recent_window(
            category,
            self.category_recent_head(category),
            MAX_SYSTEM_EVENTS_LIMIT,
        );
    }

    fn load_recent_category_window(
        &mut self,
        category: SystemEventCategory,
        events: Vec<Arc<SystemEvent>>,
    ) {
        for event in events.into_iter().rev() {
            let slot = register_retained_event(
                &mut self.retained_event_slots,
                &mut self.retained_events,
                event,
            );
            self.prepend_category_slot(category, slot);
        }
    }

    fn collect_category_recent_slots(&self, category: SystemEventCategory) -> Vec<u16> {
        let mut slots = Vec::with_capacity(self.category_recent_len(category));
        let mut cursor = self.category_recent_head(category);
        while let Some(slot) = cursor {
            let Some(entry) = self.retained_events.get(usize::from(slot)) else {
                break;
            };
            slots.push(slot);
            cursor = entry.next_same_category;
        }
        slots
    }

    fn retain_only_windowed_events(&mut self) {
        let mut retained_slots = BTreeSet::<u16>::new();
        for slot in &self.recent_events {
            retained_slots.insert(*slot);
        }
        for category in [
            SystemEventCategory::Scheduler,
            SystemEventCategory::Command,
            SystemEventCategory::Governance,
            SystemEventCategory::Publication,
        ] {
            let mut cursor = self.category_recent_head(category);
            while let Some(slot) = cursor {
                retained_slots.insert(slot);
                cursor = self
                    .retained_events
                    .get(usize::from(slot))
                    .and_then(|entry| entry.next_same_category);
            }
        }
        let mut remap = BTreeMap::<u16, u16>::new();
        let mut next_events = Vec::with_capacity(retained_slots.len());
        for retained_slot in retained_slots {
            let next_slot = u16::try_from(next_events.len())
                .expect("bounded retained system event windows should fit in u16 slots");
            remap.insert(retained_slot, next_slot);
            let event = self
                .retained_events
                .get(usize::from(retained_slot))
                .expect("retained system event slot should exist");
            next_events.push(RetainedSystemEvent {
                event: Arc::clone(&event.event),
                next_same_category: None,
            });
        }
        let old_events = std::mem::replace(&mut self.retained_events, next_events);
        let old_scheduler_head = self.recent_scheduler_head;
        let old_command_head = self.recent_command_head;
        let old_governance_head = self.recent_governance_head;
        let old_publication_head = self.recent_publication_head;
        remap_recent_slots(&mut self.recent_events, &remap);
        let (scheduler_head, scheduler_len) = remap_category_window(
            old_scheduler_head,
            &old_events,
            &remap,
            &mut self.retained_events,
        );
        let (command_head, command_len) = remap_category_window(
            old_command_head,
            &old_events,
            &remap,
            &mut self.retained_events,
        );
        let (governance_head, governance_len) = remap_category_window(
            old_governance_head,
            &old_events,
            &remap,
            &mut self.retained_events,
        );
        let (publication_head, publication_len) = remap_category_window(
            old_publication_head,
            &old_events,
            &remap,
            &mut self.retained_events,
        );
        self.recent_scheduler_head = scheduler_head;
        self.recent_scheduler_len = scheduler_len;
        self.recent_command_head = command_head;
        self.recent_command_len = command_len;
        self.recent_governance_head = governance_head;
        self.recent_governance_len = governance_len;
        self.recent_publication_head = publication_head;
        self.recent_publication_len = publication_len;
        self.retained_event_slots = self
            .retained_events
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                (
                    entry.event.event_id.clone(),
                    u16::try_from(index)
                        .expect("bounded retained system event windows should fit in u16 slots"),
                )
            })
            .collect();
    }
}

fn load_recent_window(
    retained_event_slots: &mut BTreeMap<Box<str>, u16>,
    retained_events: &mut Vec<RetainedSystemEvent>,
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
    retained_events: &mut Vec<RetainedSystemEvent>,
    event: Arc<SystemEvent>,
) -> u16 {
    if let Some(existing) = retained_event_slots.get(event.event_id.as_ref()) {
        retained_events[usize::from(*existing)].event = event;
        return *existing;
    }

    let slot =
        u16::try_from(retained_events.len()).expect("bounded retained event window should fit");
    retained_event_slots.insert(event.event_id.clone(), slot);
    retained_events.push(RetainedSystemEvent {
        event,
        next_same_category: None,
    });
    slot
}

fn push_bounded_slot(events: &mut Vec<u16>, slot: u16) {
    if events.len() == MAX_SYSTEM_EVENTS_LIMIT {
        events.pop();
    }
    events.insert(0, slot);
}

fn collect_category_recent_events(
    slots: &[u16],
    events: &[RetainedSystemEvent],
) -> Vec<Arc<SystemEvent>> {
    slots
        .iter()
        .filter_map(|slot| {
            events
                .get(usize::from(*slot))
                .map(|entry| Arc::clone(&entry.event))
        })
        .collect()
}

fn merge_window_totals(
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

fn merge_recent_windows(
    left: &SystemEventRecentWindows,
    right: &SystemEventRecentWindows,
) -> SystemEventRecentWindows {
    SystemEventRecentWindows {
        recent_events: merge_recent_arc_events(&left.recent_events, &right.recent_events),
        recent_scheduler_events: merge_recent_arc_events(
            &left.recent_scheduler_events,
            &right.recent_scheduler_events,
        ),
        recent_command_events: merge_recent_arc_events(
            &left.recent_command_events,
            &right.recent_command_events,
        ),
        recent_governance_events: merge_recent_arc_events(
            &left.recent_governance_events,
            &right.recent_governance_events,
        ),
        recent_publication_events: merge_recent_arc_events(
            &left.recent_publication_events,
            &right.recent_publication_events,
        ),
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

fn remap_recent_slots(events: &mut [u16], remap: &BTreeMap<u16, u16>) {
    for slot in events.iter_mut() {
        *slot = *remap
            .get(slot)
            .expect("windowed retained system event slot should remap");
    }
}

fn remap_category_window(
    old_head: Option<u16>,
    old_events: &[RetainedSystemEvent],
    remap: &BTreeMap<u16, u16>,
    next_events: &mut [RetainedSystemEvent],
) -> (Option<u16>, usize) {
    let mut old_cursor = old_head;
    let mut new_head = None;
    let mut previous_new = None;
    let mut len = 0;

    while let Some(old_slot) = old_cursor {
        let Some(new_slot) = remap.get(&old_slot).copied() else {
            break;
        };
        if let Some(previous_new_slot) = previous_new {
            next_events[usize::from(previous_new_slot)].next_same_category = Some(new_slot);
        } else {
            new_head = Some(new_slot);
        }
        previous_new = Some(new_slot);
        len += 1;
        old_cursor = old_events
            .get(usize::from(old_slot))
            .and_then(|entry| entry.next_same_category);
    }

    if let Some(last_slot) = previous_new {
        next_events[usize::from(last_slot)].next_same_category = None;
    }

    (new_head, len)
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
}
