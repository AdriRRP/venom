use crate::findings::finding_provider_contract::validate_provider_scan_report;
use crate::{
    DurableState, DurableStateError, FindingChangeSet, FindingProvider,
    IntegrationEventPublicationFailure, IntegrationEventPublisher, PendingIntegrationEvent,
    PublishIntegrationEventsResult, ScanRequest, SystemEvent, SystemEventKind, SystemEventsPage,
    SystemEventsQuery,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SYSTEM_EVENT_LOG_CAPACITY: usize = 512;

/// Minimal durable queue for canonical scan requests.
///
/// The runtime is intentionally single-threaded and explicit for now: queue one
/// command, durably record its terminal status, and let higher layers decide
/// about retries or parallelism later.
#[derive(Debug, Clone)]
pub struct ScanCommandQueue {
    history_path: PathBuf,
    commands: BTreeMap<Box<str>, ScanCommandRecord>,
    order: Vec<Box<str>>,
    pending_integration_events: VecDeque<PendingIntegrationEvent>,
    system_events: VecDeque<SystemEvent>,
}

impl ScanCommandQueue {
    /// Open or create one local durable scan queue and rebuild it from history.
    ///
    /// # Errors
    ///
    /// Returns [`ScanCommandQueueError`] when the queue history cannot be
    /// read, parsed, or replayed safely.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ScanCommandQueueError> {
        let history_path = path.into();
        if let Some(parent) = history_path.parent() {
            std::fs::create_dir_all(parent).map_err(ScanCommandQueueError::Io)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&history_path)
            .map_err(ScanCommandQueueError::Io)?;

        let mut runtime = Self {
            history_path,
            commands: BTreeMap::new(),
            order: Vec::new(),
            pending_integration_events: VecDeque::new(),
            system_events: VecDeque::new(),
        };
        runtime.rebuild_from_history()?;
        Ok(runtime)
    }

    /// Durably enqueue one canonical scan request.
    ///
    /// # Errors
    ///
    /// Returns [`ScanCommandQueueError`] when the queue cannot durably append
    /// the new command event.
    pub fn enqueue(
        &mut self,
        request: ScanRequest,
    ) -> Result<EnqueueScanResult, ScanCommandQueueError> {
        let command_id = next_command_id();
        let occurred_at_unix_ms = current_unix_millis()?;
        self.append_event(&DurableScanEvent::Enqueued {
            command_id: command_id.clone(),
            request: request.clone(),
            occurred_at_unix_ms,
        })?;
        self.order.push(command_id.clone());
        self.commands.insert(
            command_id.clone(),
            ScanCommandRecord {
                request,
                status: ScanCommandStatus::Pending,
            },
        );
        self.push_system_event(SystemEvent {
            event_id: format!("scan-command-enqueued-live-{command_id}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandEnqueued,
            collection_key: None,
            component_key: self
                .commands
                .get(command_id.as_ref())
                .map(|record| record.request.component_key.clone()),
            command_id: Some(command_id.clone()),
            integration_event_id: None,
            finding_count: None,
            retryable: None,
            detail: None,
        });
        Ok(EnqueueScanResult { command_id })
    }

    #[must_use]
    pub fn pending_commands(&self) -> usize {
        self.commands
            .values()
            .filter(|command| command.status == ScanCommandStatus::Pending)
            .count()
    }

    #[must_use]
    pub fn command_status(&self, command_id: &str) -> Option<ScanCommandStatus> {
        self.commands.get(command_id).map(|record| record.status)
    }

    #[must_use]
    pub fn command_statuses_snapshot(&self) -> BTreeMap<Box<str>, ScanCommandStatus> {
        self.commands
            .iter()
            .map(|(command_id, record)| (command_id.clone(), record.status))
            .collect()
    }

    #[must_use]
    pub const fn pending_integration_events(&self) -> &VecDeque<PendingIntegrationEvent> {
        &self.pending_integration_events
    }

    #[must_use]
    pub const fn system_events(&self) -> &VecDeque<SystemEvent> {
        &self.system_events
    }

    #[must_use]
    pub fn query_system_events(&self, query: &SystemEventsQuery) -> SystemEventsPage {
        crate::operations::system_event_trace::query_system_events(self.system_events.iter(), query)
    }

    /// Publish a bounded batch of pending integration events.
    ///
    /// # Errors
    ///
    /// Returns [`ScanCommandQueueError`] when publication outcome persistence fails.
    pub async fn publish_pending_integration_events(
        &mut self,
        max_events: usize,
        publisher: &(impl IntegrationEventPublisher + Sync),
    ) -> Result<PublishIntegrationEventsResult, ScanCommandQueueError> {
        let mut result = PublishIntegrationEventsResult {
            attempted: 0,
            published: 0,
            pending_remaining: self.pending_integration_events.len(),
            last_failure: None,
        };
        if max_events == 0 {
            return Ok(result);
        }

        while result.attempted < max_events {
            let Some(event) = self.pending_integration_events.front().cloned() else {
                break;
            };
            result.attempted += 1;
            match publisher.publish(&event).await {
                Ok(()) => {
                    let occurred_at_unix_ms = current_unix_millis()?;
                    self.append_event(&DurableScanEvent::IntegrationEventPublished {
                        event_id: event.event_id.clone(),
                        occurred_at_unix_ms,
                    })?;
                    self.remove_pending_integration_event(event.event_id.as_ref());
                    result.published += 1;
                    self.push_system_event(SystemEvent {
                        event_id: format!("scan-runtime-published-live-{occurred_at_unix_ms}")
                            .into_boxed_str(),
                        occurred_at_unix_ms,
                        kind: SystemEventKind::IntegrationEventPublished,
                        collection_key: None,
                        component_key: None,
                        command_id: None,
                        integration_event_id: Some(event.event_id),
                        finding_count: None,
                        retryable: None,
                        detail: None,
                    });
                }
                Err(error) => {
                    let occurred_at_unix_ms = current_unix_millis()?;
                    self.append_event(&DurableScanEvent::IntegrationEventPublicationFailed {
                        event_id: event.event_id.clone(),
                        occurred_at_unix_ms,
                        retryable: error.retryable,
                        detail: error.message.clone(),
                    })?;
                    result.last_failure = Some(IntegrationEventPublicationFailure {
                        event_id: event.event_id,
                        retryable: error.retryable,
                        message: error.message,
                    });
                    self.push_system_event(SystemEvent {
                        event_id: format!("scan-runtime-publish-failed-live-{occurred_at_unix_ms}")
                            .into_boxed_str(),
                        occurred_at_unix_ms,
                        kind: SystemEventKind::IntegrationEventPublicationFailed,
                        collection_key: None,
                        component_key: None,
                        command_id: None,
                        integration_event_id: result
                            .last_failure
                            .as_ref()
                            .map(|failure| failure.event_id.clone()),
                        finding_count: None,
                        retryable: result
                            .last_failure
                            .as_ref()
                            .map(|failure| failure.retryable),
                        detail: result
                            .last_failure
                            .as_ref()
                            .map(|failure| failure.message.clone()),
                    });
                    break;
                }
            }
        }

        result.pending_remaining = self.pending_integration_events.len();
        Ok(result)
    }

    #[must_use]
    pub fn next_pending_component_key(&self) -> Option<&str> {
        self.order.iter().find_map(|command_id| {
            self.commands.get(command_id.as_ref()).and_then(|record| {
                (record.status == ScanCommandStatus::Pending)
                    .then_some(record.request.component_key.as_ref())
            })
        })
    }

    /// Run the oldest pending scan command against one provider and durable state.
    ///
    /// # Errors
    ///
    /// Returns [`ScanCommandQueueError`] when durable queue metadata cannot
    /// be written or replayed safely. Provider and ingestion failures are not
    /// hidden: they are durably recorded and returned as a failed run result.
    pub async fn run_next(
        &mut self,
        state: &mut DurableState,
        provider: &(impl FindingProvider + Sync),
    ) -> Result<RunNextScanResult, ScanCommandQueueError> {
        let Some(command_id) = self
            .order
            .iter()
            .find(|command_id| {
                self.command_status(command_id.as_ref()) == Some(ScanCommandStatus::Pending)
            })
            .cloned()
        else {
            return Ok(RunNextScanResult::Idle);
        };

        let Some(request) = self
            .commands
            .get(command_id.as_ref())
            .map(|record| record.request.clone())
        else {
            return Err(ScanCommandQueueError::CorruptHistory {
                line: 0,
                reason: "pending scan command missing from in-memory queue".into(),
            });
        };

        let outcome = match provider.scan(&request).await {
            Ok(report) => {
                if let Err(violation) =
                    validate_provider_scan_report(provider.provider_key(), &request, &report)
                {
                    RunNextScanResult::Failed(FailedScanCommand {
                        command_id: command_id.clone(),
                        error_code: "provider-error".into(),
                        retryable: false,
                        detail: violation.message().to_owned().into_boxed_str(),
                    })
                } else {
                    match state.record_scan_report(&report) {
                        Ok(change_set) => RunNextScanResult::Completed(CompletedScanCommand {
                            command_id: command_id.clone(),
                            provider_key: report.provider_key.clone(),
                            findings_reported: report.findings.len(),
                            change_set,
                        }),
                        Err(DurableStateError::Ingestion(error)) => {
                            RunNextScanResult::Failed(FailedScanCommand {
                                command_id: command_id.clone(),
                                error_code: error.as_str().into(),
                                retryable: false,
                                detail: "provider report cannot be applied to managed ownership"
                                    .into(),
                            })
                        }
                        Err(error) => return Err(ScanCommandQueueError::State(error)),
                    }
                }
            }
            Err(error) => RunNextScanResult::Failed(FailedScanCommand {
                command_id: command_id.clone(),
                error_code: "provider-error".into(),
                retryable: error.retryable,
                detail: error.message,
            }),
        };

        self.record_outcome(&outcome)?;
        Ok(outcome)
    }

    fn record_outcome(&mut self, outcome: &RunNextScanResult) -> Result<(), ScanCommandQueueError> {
        match outcome {
            RunNextScanResult::Idle => Ok(()),
            RunNextScanResult::Completed(result) => self.record_completed_outcome(result),
            RunNextScanResult::Failed(result) => self.record_failed_outcome(result),
        }
    }

    fn record_completed_outcome(
        &mut self,
        result: &CompletedScanCommand,
    ) -> Result<(), ScanCommandQueueError> {
        let command = self
            .commands
            .get(result.command_id.as_ref())
            .ok_or_else(|| ScanCommandQueueError::CorruptHistory {
                line: 0,
                reason: "completed scan command missing from in-memory queue".into(),
            })?;
        let occurred_at_unix_ms = current_unix_millis()?;
        let pending_integration_event = PendingIntegrationEvent::scan_command_completed(
            result.command_id.as_ref(),
            command.request.component_key.clone(),
            command.request.artifact.clone(),
            result.provider_key.clone(),
            command.request.freshness,
            result.findings_reported,
            result.change_set.clone(),
        );
        self.append_event(&DurableScanEvent::Completed {
            command_id: result.command_id.clone(),
            provider_key: result.provider_key.clone(),
            findings_reported: result.findings_reported,
            change_set: result.change_set.clone(),
            occurred_at_unix_ms,
            pending_integration_event: Box::new(Some(pending_integration_event.clone())),
        })?;
        let Some(command) = self.commands.get_mut(result.command_id.as_ref()) else {
            return Err(ScanCommandQueueError::CorruptHistory {
                line: 0,
                reason: "completed scan command missing from in-memory queue".into(),
            });
        };
        let component_key = command.request.component_key.clone();
        command.status = ScanCommandStatus::Completed;
        self.pending_integration_events
            .push_back(pending_integration_event);
        self.push_system_event(SystemEvent {
            event_id: format!(
                "scan-command-completed-live-{}-{occurred_at_unix_ms}",
                result.command_id
            )
            .into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandCompleted,
            collection_key: None,
            component_key: Some(component_key),
            command_id: Some(result.command_id.clone()),
            integration_event_id: None,
            finding_count: u32::try_from(result.findings_reported).ok(),
            retryable: None,
            detail: Some(
                format!(
                    "discovered {}, repeated {}, withdrawn {}, active {}",
                    result.change_set.discovered,
                    result.change_set.repeated,
                    result.change_set.withdrawn,
                    result.change_set.active
                )
                .into_boxed_str(),
            ),
        });
        Ok(())
    }

    fn record_failed_outcome(
        &mut self,
        result: &FailedScanCommand,
    ) -> Result<(), ScanCommandQueueError> {
        let component_key = self
            .commands
            .get(result.command_id.as_ref())
            .map(|record| record.request.component_key.clone());
        let occurred_at_unix_ms = current_unix_millis()?;
        self.append_event(&DurableScanEvent::Failed {
            command_id: result.command_id.clone(),
            occurred_at_unix_ms,
            error_code: result.error_code.clone(),
            retryable: result.retryable,
            detail: result.detail.clone(),
        })?;
        let Some(command) = self.commands.get_mut(result.command_id.as_ref()) else {
            return Err(ScanCommandQueueError::CorruptHistory {
                line: 0,
                reason: "failed scan command missing from in-memory queue".into(),
            });
        };
        command.status = ScanCommandStatus::Failed;
        self.push_system_event(SystemEvent {
            event_id: format!(
                "scan-command-failed-live-{}-{occurred_at_unix_ms}",
                result.command_id
            )
            .into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandFailed,
            collection_key: None,
            component_key,
            command_id: Some(result.command_id.clone()),
            integration_event_id: None,
            finding_count: None,
            retryable: Some(result.retryable),
            detail: Some(result.detail.clone()),
        });
        Ok(())
    }

    fn rebuild_from_history(&mut self) -> Result<(), ScanCommandQueueError> {
        let file = File::open(&self.history_path).map_err(ScanCommandQueueError::Io)?;
        let reader = BufReader::new(file);
        self.commands.clear();
        self.order.clear();
        self.pending_integration_events.clear();
        self.system_events.clear();

        for (line_index, line) in reader.lines().enumerate() {
            let line = line.map_err(ScanCommandQueueError::Io)?;
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str::<DurableScanEvent>(&line).map_err(|error| {
                ScanCommandQueueError::CorruptHistory {
                    line: line_index + 1,
                    reason: error.to_string().into_boxed_str(),
                }
            })?;
            self.apply_event(event, line_index + 1)?;
        }

        Ok(())
    }

    fn apply_event(
        &mut self,
        event: DurableScanEvent,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        match event {
            DurableScanEvent::Enqueued {
                command_id,
                request,
                occurred_at_unix_ms,
            } => self.apply_enqueued_event(command_id, request, occurred_at_unix_ms, line),
            DurableScanEvent::Completed {
                command_id,
                findings_reported,
                change_set,
                occurred_at_unix_ms,
                pending_integration_event,
                ..
            } => self.apply_completed_event(
                command_id,
                findings_reported,
                change_set,
                occurred_at_unix_ms,
                *pending_integration_event,
                line,
            ),
            DurableScanEvent::IntegrationEventPublished {
                event_id,
                occurred_at_unix_ms,
            } => self.apply_published_event(event_id, occurred_at_unix_ms, line),
            DurableScanEvent::IntegrationEventPublicationFailed {
                event_id,
                occurred_at_unix_ms,
                retryable,
                detail,
            } => self.apply_publish_failed_event(
                event_id,
                occurred_at_unix_ms,
                retryable,
                detail,
                line,
            ),
            DurableScanEvent::Failed {
                command_id,
                occurred_at_unix_ms,
                retryable,
                detail,
                ..
            } => self.apply_failed_event(command_id, occurred_at_unix_ms, retryable, detail, line),
        }
    }

    fn apply_enqueued_event(
        &mut self,
        command_id: Box<str>,
        request: ScanRequest,
        occurred_at_unix_ms: u64,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        if self.commands.contains_key(command_id.as_ref()) {
            return Err(ScanCommandQueueError::CorruptHistory {
                line,
                reason: "duplicate scan command id".into(),
            });
        }
        self.order.push(command_id.clone());
        self.commands.insert(
            command_id.clone(),
            ScanCommandRecord {
                request,
                status: ScanCommandStatus::Pending,
            },
        );
        let component_key = self
            .commands
            .get(command_id.as_ref())
            .map(|record| record.request.component_key.clone());
        self.push_system_event(SystemEvent {
            event_id: format!("scan-command-enqueued-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandEnqueued,
            collection_key: None,
            component_key,
            command_id: Some(command_id),
            integration_event_id: None,
            finding_count: None,
            retryable: None,
            detail: None,
        });
        Ok(())
    }

    fn apply_completed_event(
        &mut self,
        command_id: Box<str>,
        findings_reported: usize,
        change_set: FindingChangeSet,
        occurred_at_unix_ms: u64,
        pending_integration_event: Option<PendingIntegrationEvent>,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        let component_key = self
            .commands
            .get(command_id.as_ref())
            .map(|record| record.request.component_key.clone());
        if let Some(pending_integration_event) = pending_integration_event {
            self.pending_integration_events
                .push_back(pending_integration_event);
        }
        self.mark_terminal(line, &command_id, ScanCommandStatus::Completed)?;
        self.push_system_event(SystemEvent {
            event_id: format!("scan-command-completed-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandCompleted,
            collection_key: None,
            component_key,
            command_id: Some(command_id),
            integration_event_id: None,
            finding_count: u32::try_from(findings_reported).ok(),
            retryable: None,
            detail: Some(
                format!(
                    "discovered {}, repeated {}, withdrawn {}, active {}",
                    change_set.discovered,
                    change_set.repeated,
                    change_set.withdrawn,
                    change_set.active
                )
                .into_boxed_str(),
            ),
        });
        Ok(())
    }

    fn apply_published_event(
        &mut self,
        event_id: Box<str>,
        occurred_at_unix_ms: u64,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        self.remove_pending_integration_event(event_id.as_ref());
        self.push_system_event(SystemEvent {
            event_id: format!("scan-runtime-published-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::IntegrationEventPublished,
            collection_key: None,
            component_key: None,
            command_id: None,
            integration_event_id: Some(event_id),
            finding_count: None,
            retryable: None,
            detail: None,
        });
        Ok(())
    }

    fn apply_publish_failed_event(
        &mut self,
        event_id: Box<str>,
        occurred_at_unix_ms: u64,
        retryable: bool,
        detail: Box<str>,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        self.push_system_event(SystemEvent {
            event_id: format!("scan-runtime-publish-failed-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::IntegrationEventPublicationFailed,
            collection_key: None,
            component_key: None,
            command_id: None,
            integration_event_id: Some(event_id),
            finding_count: None,
            retryable: Some(retryable),
            detail: Some(detail),
        });
        Ok(())
    }

    fn apply_failed_event(
        &mut self,
        command_id: Box<str>,
        occurred_at_unix_ms: u64,
        retryable: bool,
        detail: Box<str>,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        let component_key = self
            .commands
            .get(command_id.as_ref())
            .map(|record| record.request.component_key.clone());
        self.mark_terminal(line, &command_id, ScanCommandStatus::Failed)?;
        self.push_system_event(SystemEvent {
            event_id: format!("scan-command-failed-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandFailed,
            collection_key: None,
            component_key,
            command_id: Some(command_id),
            integration_event_id: None,
            finding_count: None,
            retryable: Some(retryable),
            detail: Some(detail),
        });
        Ok(())
    }

    fn remove_pending_integration_event(&mut self, event_id: &str) {
        if self
            .pending_integration_events
            .front()
            .is_some_and(|event| event.event_id.as_ref() == event_id)
        {
            self.pending_integration_events.pop_front();
            return;
        }

        if let Some(index) = self
            .pending_integration_events
            .iter()
            .position(|event| event.event_id.as_ref() == event_id)
        {
            self.pending_integration_events.remove(index);
        }
    }

    fn mark_terminal(
        &mut self,
        line: usize,
        command_id: &str,
        status: ScanCommandStatus,
    ) -> Result<(), ScanCommandQueueError> {
        let Some(record) = self.commands.get_mut(command_id) else {
            return Err(ScanCommandQueueError::CorruptHistory {
                line,
                reason: "terminal event without prior enqueue".into(),
            });
        };
        if record.status != ScanCommandStatus::Pending {
            return Err(ScanCommandQueueError::CorruptHistory {
                line,
                reason: "duplicate terminal state for scan command".into(),
            });
        }
        record.status = status;
        Ok(())
    }

    fn append_event(&self, event: &DurableScanEvent) -> Result<(), ScanCommandQueueError> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.history_path)
            .map_err(ScanCommandQueueError::Io)?;
        serde_json::to_writer(&mut file, event).map_err(ScanCommandQueueError::Serialize)?;
        file.write_all(b"\n").map_err(ScanCommandQueueError::Io)?;
        file.flush().map_err(ScanCommandQueueError::Io)?;
        file.sync_all().map_err(ScanCommandQueueError::Io)?;
        Ok(())
    }

    fn push_system_event(&mut self, event: SystemEvent) {
        self.system_events.push_front(event);
        while self.system_events.len() > SYSTEM_EVENT_LOG_CAPACITY {
            self.system_events.pop_back();
        }
    }
}

fn current_unix_millis() -> Result<u64, ScanCommandQueueError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ScanCommandQueueError::CorruptHistory {
            line: 0,
            reason: error.to_string().into_boxed_str(),
        })?;
    u64::try_from(duration.as_millis()).map_err(|_| ScanCommandQueueError::CorruptHistory {
        line: 0,
        reason: "timestamp out of range".into(),
    })
}

/// Stable observable status of one durable scan command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanCommandStatus {
    Pending,
    Completed,
    Failed,
}

impl ScanCommandStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
struct ScanCommandRecord {
    request: ScanRequest,
    status: ScanCommandStatus,
}

/// Observable result of enqueuing one durable scan command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueScanResult {
    pub command_id: Box<str>,
}

/// Observable outcome of attempting to run the next queued scan command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunNextScanResult {
    Idle,
    Completed(CompletedScanCommand),
    Failed(FailedScanCommand),
}

/// Completed durable scan command with its applied business result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedScanCommand {
    pub command_id: Box<str>,
    pub provider_key: Box<str>,
    pub findings_reported: usize,
    pub change_set: FindingChangeSet,
}

/// Failed durable scan command with an explicit terminal error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedScanCommand {
    pub command_id: Box<str>,
    pub error_code: Box<str>,
    pub retryable: bool,
    pub detail: Box<str>,
}

/// Canonical failure returned by the durable scan runtime.
#[derive(Debug)]
pub enum ScanCommandQueueError {
    Io(io::Error),
    Serialize(serde_json::Error),
    CorruptHistory { line: usize, reason: Box<str> },
    State(DurableStateError),
}

impl ScanCommandQueueError {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "io-error",
            Self::Serialize(_) => "serialization-error",
            Self::CorruptHistory { .. } => "corrupt-history",
            Self::State(error) => error.as_str(),
        }
    }
}

impl core::fmt::Display for ScanCommandQueueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Serialize(error) => write!(f, "serialization error: {error}"),
            Self::CorruptHistory { line, reason } => {
                write!(f, "corrupt history at line {line}: {reason}")
            }
            Self::State(error) => write!(f, "state error: {error}"),
        }
    }
}

impl std::error::Error for ScanCommandQueueError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum DurableScanEvent {
    Enqueued {
        command_id: Box<str>,
        request: ScanRequest,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    Completed {
        command_id: Box<str>,
        provider_key: Box<str>,
        findings_reported: usize,
        change_set: FindingChangeSet,
        #[serde(default)]
        occurred_at_unix_ms: u64,
        #[serde(default)]
        pending_integration_event: Box<Option<PendingIntegrationEvent>>,
    },
    Failed {
        command_id: Box<str>,
        #[serde(default)]
        occurred_at_unix_ms: u64,
        error_code: Box<str>,
        retryable: bool,
        detail: Box<str>,
    },
    IntegrationEventPublished {
        event_id: Box<str>,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    IntegrationEventPublicationFailed {
        event_id: Box<str>,
        #[serde(default)]
        occurred_at_unix_ms: u64,
        retryable: bool,
        detail: Box<str>,
    },
}

fn next_command_id() -> Box<str> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("scan-command-{nanos}-{counter}").into_boxed_str()
}

#[cfg(test)]
mod tests {
    use super::{RunNextScanResult, ScanCommandQueue, ScanCommandStatus};
    use crate::{
        ArtifactKind, ArtifactRef, ComponentRegistration, DurableState, EvidenceFreshness,
        FindingProvider, FindingProviderError, FindingProviderErrorKind, IntegrationEvent,
        IntegrationEventPublishError, IntegrationEventPublisher, PackageCoordinate,
        PendingIntegrationEvent, ProviderScanReport, ReportedFinding, ScanPlanner, ScanRequest,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("venom-{name}-{nanos}-{counter}.jsonl"))
    }

    fn artifact() -> ArtifactRef {
        ArtifactRef::new(
            ArtifactKind::ContainerImage,
            "registry.example/payments@sha256:111",
        )
    }

    #[derive(Debug, Clone)]
    enum FakeMode {
        Success(Vec<ReportedFinding>),
        Failure(FindingProviderError),
    }

    #[derive(Debug, Clone)]
    struct FakeProvider {
        mode: FakeMode,
    }

    impl FakeProvider {
        fn success(findings: Vec<ReportedFinding>) -> Self {
            Self {
                mode: FakeMode::Success(findings),
            }
        }

        fn failure(error: FindingProviderError) -> Self {
            Self {
                mode: FakeMode::Failure(error),
            }
        }
    }

    impl FindingProvider for FakeProvider {
        fn provider_key(&self) -> &'static str {
            "fixture-provider"
        }

        async fn scan<'a>(
            &'a self,
            request: &'a ScanRequest,
        ) -> Result<ProviderScanReport, FindingProviderError> {
            match &self.mode {
                FakeMode::Success(findings) => Ok(ProviderScanReport::new(
                    "fixture-provider",
                    request.component_key.clone(),
                    request.artifact.clone(),
                    SystemTime::UNIX_EPOCH,
                    request.freshness,
                    findings.clone(),
                )
                .with_knowledge_revision("fixture-db:2026-05-14")),
                FakeMode::Failure(error) => Err(error.clone()),
            }
        }
    }

    fn durable_inventory() -> (DurableState, crate::ScanRequest) {
        let path = temp_path("durable-runtime-state");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact should persist");
        let request = ScanPlanner::new(state.ingestion().inventory())
            .plan(
                "component:payments-api",
                artifact(),
                EvidenceFreshness::Deterministic,
            )
            .expect("planner should create request");
        (state, request)
    }

    #[tokio::test]
    async fn completed_scan_command_updates_state_and_status() {
        let queue_path = temp_path("durable-runtime-queue");
        let (mut state, request) = durable_inventory();
        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");
        let enqueue = runtime.enqueue(request).expect("enqueue should persist");
        let provider = FakeProvider::success(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]);

        let result = runtime
            .run_next(&mut state, &provider)
            .await
            .expect("runtime should record completion");

        assert!(matches!(result, RunNextScanResult::Completed(_)));
        assert_eq!(
            runtime.command_status(enqueue.command_id.as_ref()),
            Some(ScanCommandStatus::Completed)
        );
        assert_eq!(
            state
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            1
        );
    }

    #[tokio::test]
    async fn failed_scan_command_is_terminal_and_explicit() {
        let queue_path = temp_path("durable-runtime-failure");
        let (mut state, request) = durable_inventory();
        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");
        let enqueue = runtime.enqueue(request).expect("enqueue should persist");
        let provider = FakeProvider::failure(FindingProviderError::new(
            FindingProviderErrorKind::Unavailable,
            true,
            "fixture provider unavailable",
        ));

        let result = runtime
            .run_next(&mut state, &provider)
            .await
            .expect("runtime should durably record failure");

        assert!(matches!(result, RunNextScanResult::Failed(_)));
        assert_eq!(
            runtime.command_status(enqueue.command_id.as_ref()),
            Some(ScanCommandStatus::Failed)
        );
        assert_eq!(
            state
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            0
        );
    }

    #[tokio::test]
    async fn completed_scan_command_appends_pending_integration_event() {
        let queue_path = temp_path("durable-runtime-outbox");
        let (mut state, request) = durable_inventory();
        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");
        let enqueue = runtime.enqueue(request).expect("enqueue should persist");
        let provider = FakeProvider::success(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]);

        let result = runtime
            .run_next(&mut state, &provider)
            .await
            .expect("runtime should record completion");
        assert!(matches!(result, RunNextScanResult::Completed(_)));
        assert_eq!(runtime.pending_integration_events().len(), 1);
        assert!(matches!(
            runtime.pending_integration_events()[0].event,
            IntegrationEvent::ScanCommandCompleted { .. }
        ));

        let rebuilt = ScanCommandQueue::open(&queue_path).expect("runtime should replay");
        assert_eq!(
            rebuilt.command_status(enqueue.command_id.as_ref()),
            Some(ScanCommandStatus::Completed)
        );
        assert_eq!(rebuilt.pending_integration_events().len(), 1);
    }

    #[derive(Debug)]
    struct SuccessPublisher;

    impl IntegrationEventPublisher for SuccessPublisher {
        fn publisher_key(&self) -> &'static str {
            "fixture-publisher"
        }

        async fn publish<'a>(
            &'a self,
            _event: &'a PendingIntegrationEvent,
        ) -> Result<(), IntegrationEventPublishError> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingPublisher;

    impl IntegrationEventPublisher for FailingPublisher {
        fn publisher_key(&self) -> &'static str {
            "fixture-publisher"
        }

        async fn publish<'a>(
            &'a self,
            _event: &'a PendingIntegrationEvent,
        ) -> Result<(), IntegrationEventPublishError> {
            Err(IntegrationEventPublishError::new(
                true,
                "publisher unavailable",
            ))
        }
    }

    #[tokio::test]
    async fn successful_publication_removes_pending_runtime_integration_event() {
        let queue_path = temp_path("durable-runtime-publish-success");
        let (mut state, request) = durable_inventory();
        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");
        let _ = runtime.enqueue(request).expect("enqueue should persist");
        let provider = FakeProvider::success(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]);
        let _ = runtime
            .run_next(&mut state, &provider)
            .await
            .expect("runtime should record completion");

        let result = runtime
            .publish_pending_integration_events(1, &SuccessPublisher)
            .await
            .expect("publication should persist");
        assert_eq!(result.published, 1);
        assert_eq!(runtime.pending_integration_events().len(), 0);

        let rebuilt = ScanCommandQueue::open(&queue_path).expect("runtime should replay");
        assert_eq!(rebuilt.pending_integration_events().len(), 0);
    }

    #[tokio::test]
    async fn failed_publication_keeps_pending_runtime_integration_event() {
        let queue_path = temp_path("durable-runtime-publish-failure");
        let (mut state, request) = durable_inventory();
        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");
        let _ = runtime.enqueue(request).expect("enqueue should persist");
        let provider = FakeProvider::success(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]);
        let _ = runtime
            .run_next(&mut state, &provider)
            .await
            .expect("runtime should record completion");

        let result = runtime
            .publish_pending_integration_events(1, &FailingPublisher)
            .await
            .expect("failed publication outcome should persist");
        assert_eq!(result.published, 0);
        assert_eq!(runtime.pending_integration_events().len(), 1);

        let rebuilt = ScanCommandQueue::open(&queue_path).expect("runtime should replay");
        assert_eq!(rebuilt.pending_integration_events().len(), 1);
    }
}
