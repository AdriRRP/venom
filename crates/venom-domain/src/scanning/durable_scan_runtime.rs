use crate::durable_state::StoredProviderScanReport;
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
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
    command_statuses_snapshot_cache: Arc<BTreeMap<Box<str>, ScanCommandStatus>>,
    system_events_snapshot_cache: Arc<Vec<SystemEvent>>,
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
            command_statuses_snapshot_cache: Arc::new(BTreeMap::new()),
            system_events_snapshot_cache: Arc::new(Vec::new()),
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
        let command_id = self
            .enqueue_batch_with_origin(vec![request], None)?
            .into_iter()
            .next()
            .ok_or_else(|| ScanCommandQueueError::CorruptHistory {
                line: 0,
                reason: "batch enqueue did not return a command id".into(),
            })?;
        Ok(EnqueueScanResult { command_id })
    }

    /// Durably enqueue one canonical scan batch in one append-only queue write.
    ///
    /// # Errors
    ///
    /// Returns [`ScanCommandQueueError`] when the queue cannot durably append
    /// the batch event.
    pub fn enqueue_batch(
        &mut self,
        requests: Vec<ScanRequest>,
    ) -> Result<Vec<Box<str>>, ScanCommandQueueError> {
        self.enqueue_batch_with_origin(requests, None)
    }

    /// Durably enqueue one collection-scheduled batch keyed by the due time that triggered it.
    ///
    /// If the same batch is already durable, returns the existing command ids
    /// instead of duplicating work.
    ///
    /// # Errors
    ///
    /// Returns [`ScanCommandQueueError`] when the queue cannot durably append
    /// the missing command events.
    pub fn enqueue_collection_batch(
        &mut self,
        collection_key: &str,
        due_at_unix_ms: u64,
        requests: Vec<ScanRequest>,
    ) -> Result<Vec<Box<str>>, ScanCommandQueueError> {
        if let Some(existing) = self.find_collection_batch(collection_key, due_at_unix_ms) {
            return Ok(existing);
        }

        let origin = CollectionScheduleOrigin {
            collection_key: collection_key.into(),
            due_at_unix_ms,
        };
        self.enqueue_batch_with_origin(requests, Some(&origin))
    }

    fn enqueue_batch_with_origin(
        &mut self,
        requests: Vec<ScanRequest>,
        collection_schedule_origin: Option<&CollectionScheduleOrigin>,
    ) -> Result<Vec<Box<str>>, ScanCommandQueueError> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let occurred_at_unix_ms = current_unix_millis()?;
        let enqueued = requests
            .into_iter()
            .map(|request| DurableEnqueuedCommand {
                command_id: next_command_id(),
                request,
                collection_schedule_origin: collection_schedule_origin.cloned(),
            })
            .collect::<Vec<_>>();
        self.append_event(&DurableScanEvent::BatchEnqueued {
            commands: enqueued.clone(),
            occurred_at_unix_ms,
        })?;
        self.apply_enqueued_batch(&enqueued, occurred_at_unix_ms);
        Ok(enqueued.into_iter().map(|entry| entry.command_id).collect())
    }

    #[must_use]
    pub fn pending_commands(&self) -> usize {
        self.commands
            .values()
            .filter(|command| !command.status.is_terminal())
            .count()
    }

    #[must_use]
    pub fn command_status(&self, command_id: &str) -> Option<ScanCommandStatus> {
        self.commands.get(command_id).map(|record| record.status)
    }

    #[must_use]
    pub fn command_statuses_snapshot(&self) -> BTreeMap<Box<str>, ScanCommandStatus> {
        self.command_statuses_snapshot_cache.as_ref().clone()
    }

    #[must_use]
    pub fn command_statuses_snapshot_arc(&self) -> Arc<BTreeMap<Box<str>, ScanCommandStatus>> {
        Arc::clone(&self.command_statuses_snapshot_cache)
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
    pub fn system_events_snapshot_arc(&self) -> Arc<Vec<SystemEvent>> {
        Arc::clone(&self.system_events_snapshot_cache)
    }

    fn find_collection_batch(
        &self,
        collection_key: &str,
        due_at_unix_ms: u64,
    ) -> Option<Vec<Box<str>>> {
        let command_ids = self
            .order
            .iter()
            .filter(|command_id| {
                self.commands
                    .get(command_id.as_ref())
                    .and_then(|record| record.collection_schedule_origin.as_ref())
                    .is_some_and(|origin| {
                        origin.collection_key.as_ref() == collection_key
                            && origin.due_at_unix_ms == due_at_unix_ms
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        (!command_ids.is_empty()).then_some(command_ids)
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
                (!record.status.is_terminal()).then_some(record.request.component_key.as_ref())
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
                self.commands
                    .get(command_id.as_ref())
                    .is_some_and(|record| !record.status.is_terminal())
            })
            .cloned()
        else {
            return Ok(RunNextScanResult::Idle);
        };

        let Some(command) = self.commands.get(command_id.as_ref()).cloned() else {
            return Err(ScanCommandQueueError::CorruptHistory {
                line: 0,
                reason: "pending scan command missing from in-memory queue".into(),
            });
        };

        if let Some(stored_report) = command.captured_report {
            return self.resume_captured_report(&command_id, &stored_report, state);
        }

        let outcome = match provider.scan(&command.request).await {
            Ok(report) => {
                if let Err(violation) = validate_provider_scan_report(
                    provider.provider_key(),
                    &command.request,
                    &report,
                ) {
                    RunNextScanResult::Failed(FailedScanCommand {
                        command_id: command_id.clone(),
                        error_code: "provider-error".into(),
                        retryable: false,
                        detail: violation.message().to_owned().into_boxed_str(),
                    })
                } else {
                    self.capture_report(&command_id, &report)?;
                    return self.resume_captured_report(
                        &command_id,
                        &StoredProviderScanReport::from_report(&report)
                            .map_err(ScanCommandQueueError::State)?,
                        state,
                    );
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

    fn resume_captured_report(
        &mut self,
        command_id: &str,
        stored_report: &StoredProviderScanReport,
        state: &mut DurableState,
    ) -> Result<RunNextScanResult, ScanCommandQueueError> {
        let report = stored_report
            .clone()
            .into_domain()
            .map_err(ScanCommandQueueError::State)?;
        let outcome = match state.record_scan_report_for_command(command_id, &report) {
            Ok(change_set) => RunNextScanResult::Completed(CompletedScanCommand {
                command_id: command_id.into(),
                provider_key: report.provider_key.clone(),
                findings_reported: report.findings.len(),
                change_set,
            }),
            Err(DurableStateError::Ingestion(error)) => {
                RunNextScanResult::Failed(FailedScanCommand {
                    command_id: command_id.into(),
                    error_code: error.as_str().into(),
                    retryable: false,
                    detail: "provider report cannot be applied to managed ownership".into(),
                })
            }
            Err(error) => return Err(ScanCommandQueueError::State(error)),
        };
        self.record_outcome(&outcome)?;
        Ok(outcome)
    }

    fn capture_report(
        &mut self,
        command_id: &str,
        report: &crate::ProviderScanReport,
    ) -> Result<(), ScanCommandQueueError> {
        let occurred_at_unix_ms = current_unix_millis()?;
        let stored_report =
            StoredProviderScanReport::from_report(report).map_err(ScanCommandQueueError::State)?;
        self.append_event(&DurableScanEvent::ReportCaptured {
            command_id: command_id.into(),
            report: stored_report.clone(),
            occurred_at_unix_ms,
        })?;
        let Some(command) = self.commands.get_mut(command_id) else {
            return Err(ScanCommandQueueError::CorruptHistory {
                line: 0,
                reason: "captured scan command missing from in-memory queue".into(),
            });
        };
        command.status = ScanCommandStatus::Applying;
        command.captured_report = Some(stored_report);
        self.refresh_command_statuses_snapshot_cache();
        Ok(())
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
        command.captured_report = None;
        self.refresh_command_statuses_snapshot_cache();
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
        command.captured_report = None;
        self.refresh_command_statuses_snapshot_cache();
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

        self.refresh_command_statuses_snapshot_cache();
        self.refresh_system_events_snapshot_cache();

        Ok(())
    }

    fn apply_event(
        &mut self,
        event: DurableScanEvent,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        match event {
            DurableScanEvent::BatchEnqueued {
                commands,
                occurred_at_unix_ms,
            } => self.apply_batch_enqueued_event(&commands, occurred_at_unix_ms, line),
            DurableScanEvent::Enqueued {
                command_id,
                request,
                occurred_at_unix_ms,
                collection_schedule_origin,
            } => self.apply_enqueued_event(
                command_id,
                request,
                occurred_at_unix_ms,
                collection_schedule_origin,
                line,
            ),
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
                &change_set,
                occurred_at_unix_ms,
                *pending_integration_event,
                line,
            ),
            DurableScanEvent::ReportCaptured {
                command_id,
                report,
                occurred_at_unix_ms,
            } => self.apply_report_captured_event(
                command_id.as_ref(),
                report,
                occurred_at_unix_ms,
                line,
            ),
            DurableScanEvent::IntegrationEventPublished {
                event_id,
                occurred_at_unix_ms,
            } => {
                self.apply_published_event(event_id, occurred_at_unix_ms, line);
                Ok(())
            }
            DurableScanEvent::IntegrationEventPublicationFailed {
                event_id,
                occurred_at_unix_ms,
                retryable,
                detail,
            } => {
                self.apply_publish_failed_event(
                    event_id,
                    occurred_at_unix_ms,
                    retryable,
                    detail,
                    line,
                );
                Ok(())
            }
            DurableScanEvent::Failed {
                command_id,
                occurred_at_unix_ms,
                retryable,
                detail,
                ..
            } => self.apply_failed_event(command_id, occurred_at_unix_ms, retryable, detail, line),
        }
    }

    fn apply_batch_enqueued_event(
        &mut self,
        commands: &[DurableEnqueuedCommand],
        occurred_at_unix_ms: u64,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        for command in commands {
            if self.commands.contains_key(command.command_id.as_ref()) {
                return Err(ScanCommandQueueError::CorruptHistory {
                    line,
                    reason: "duplicate scan command id".into(),
                });
            }
        }
        for (index, command) in commands.iter().enumerate() {
            self.order.push(command.command_id.clone());
            self.commands.insert(
                command.command_id.clone(),
                ScanCommandRecord {
                    request: command.request.clone(),
                    status: ScanCommandStatus::Pending,
                    collection_schedule_origin: command.collection_schedule_origin.clone(),
                    captured_report: None,
                },
            );
            let collection_key = command
                .collection_schedule_origin
                .as_ref()
                .map(|origin| origin.collection_key.clone());
            self.push_system_event(SystemEvent {
                event_id: format!("scan-command-enqueued-{line}-{index}").into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::ScanCommandEnqueued,
                collection_key,
                component_key: Some(command.request.component_key.clone()),
                command_id: Some(command.command_id.clone()),
                integration_event_id: None,
                finding_count: None,
                retryable: None,
                detail: None,
            });
        }
        Ok(())
    }

    fn apply_report_captured_event(
        &mut self,
        command_id: &str,
        report: StoredProviderScanReport,
        _occurred_at_unix_ms: u64,
        line: usize,
    ) -> Result<(), ScanCommandQueueError> {
        let Some(record) = self.commands.get_mut(command_id) else {
            return Err(ScanCommandQueueError::CorruptHistory {
                line,
                reason: "captured report without prior enqueue".into(),
            });
        };
        if record.status != ScanCommandStatus::Pending {
            return Err(ScanCommandQueueError::CorruptHistory {
                line,
                reason: "captured report for non-pending scan command".into(),
            });
        }
        record.status = ScanCommandStatus::Applying;
        record.captured_report = Some(report);
        self.refresh_command_statuses_snapshot_cache();
        Ok(())
    }

    fn apply_enqueued_event(
        &mut self,
        command_id: Box<str>,
        request: ScanRequest,
        occurred_at_unix_ms: u64,
        collection_schedule_origin: Option<CollectionScheduleOrigin>,
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
                collection_schedule_origin: collection_schedule_origin.clone(),
                captured_report: None,
            },
        );
        self.refresh_command_statuses_snapshot_cache();
        let component_key = self
            .commands
            .get(command_id.as_ref())
            .map(|record| record.request.component_key.clone());
        let collection_key = collection_schedule_origin.map(|origin| origin.collection_key);
        self.push_system_event(SystemEvent {
            event_id: format!("scan-command-enqueued-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::ScanCommandEnqueued,
            collection_key,
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
        change_set: &FindingChangeSet,
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
        if let Some(record) = self.commands.get_mut(command_id.as_ref()) {
            record.captured_report = None;
        }
        self.refresh_command_statuses_snapshot_cache();
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

    fn apply_published_event(&mut self, event_id: Box<str>, occurred_at_unix_ms: u64, line: usize) {
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
    }

    fn apply_publish_failed_event(
        &mut self,
        event_id: Box<str>,
        occurred_at_unix_ms: u64,
        retryable: bool,
        detail: Box<str>,
        line: usize,
    ) {
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
        if let Some(record) = self.commands.get_mut(command_id.as_ref()) {
            record.captured_report = None;
        }
        self.refresh_command_statuses_snapshot_cache();
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
        if !matches!(
            record.status,
            ScanCommandStatus::Pending | ScanCommandStatus::Applying
        ) {
            return Err(ScanCommandQueueError::CorruptHistory {
                line,
                reason: "duplicate terminal state for scan command".into(),
            });
        }
        record.status = status;
        self.refresh_command_statuses_snapshot_cache();
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
        self.refresh_system_events_snapshot_cache();
    }

    fn apply_enqueued_batch(
        &mut self,
        commands: &[DurableEnqueuedCommand],
        occurred_at_unix_ms: u64,
    ) {
        for command in commands {
            self.order.push(command.command_id.clone());
            self.commands.insert(
                command.command_id.clone(),
                ScanCommandRecord {
                    request: command.request.clone(),
                    status: ScanCommandStatus::Pending,
                    collection_schedule_origin: command.collection_schedule_origin.clone(),
                    captured_report: None,
                },
            );
            let collection_key = command
                .collection_schedule_origin
                .as_ref()
                .map(|origin| origin.collection_key.clone());
            self.push_system_event(SystemEvent {
                event_id: format!("scan-command-enqueued-live-{}", command.command_id)
                    .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::ScanCommandEnqueued,
                collection_key,
                component_key: Some(command.request.component_key.clone()),
                command_id: Some(command.command_id.clone()),
                integration_event_id: None,
                finding_count: None,
                retryable: None,
                detail: None,
            });
        }
        self.refresh_command_statuses_snapshot_cache();
    }

    fn refresh_system_events_snapshot_cache(&mut self) {
        self.system_events_snapshot_cache = Arc::new(self.system_events.iter().cloned().collect());
    }

    fn refresh_command_statuses_snapshot_cache(&mut self) {
        self.command_statuses_snapshot_cache = Arc::new(
            self.commands
                .iter()
                .map(|(command_id, record)| (command_id.clone(), record.status))
                .collect(),
        );
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
    Applying,
    Completed,
    Failed,
}

impl ScanCommandStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applying => "applying",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

#[derive(Debug, Clone)]
struct ScanCommandRecord {
    request: ScanRequest,
    status: ScanCommandStatus,
    collection_schedule_origin: Option<CollectionScheduleOrigin>,
    captured_report: Option<StoredProviderScanReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CollectionScheduleOrigin {
    collection_key: Box<str>,
    due_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DurableEnqueuedCommand {
    command_id: Box<str>,
    request: ScanRequest,
    #[serde(default)]
    collection_schedule_origin: Option<CollectionScheduleOrigin>,
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
    BatchEnqueued {
        commands: Vec<DurableEnqueuedCommand>,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    Enqueued {
        command_id: Box<str>,
        request: ScanRequest,
        #[serde(default)]
        occurred_at_unix_ms: u64,
        #[serde(default)]
        collection_schedule_origin: Option<CollectionScheduleOrigin>,
    },
    ReportCaptured {
        command_id: Box<str>,
        report: StoredProviderScanReport,
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
    use std::fs;
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

    #[derive(Debug)]
    struct PanicProvider;

    impl FindingProvider for PanicProvider {
        fn provider_key(&self) -> &'static str {
            "fixture-provider"
        }

        async fn scan<'a>(
            &'a self,
            _request: &'a ScanRequest,
        ) -> Result<ProviderScanReport, FindingProviderError> {
            panic!("applying commands must not rescan the provider")
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

    #[test]
    fn collection_batch_enqueue_is_idempotent_for_the_same_due_origin() {
        let queue_path = temp_path("durable-runtime-collection-batch");
        let (_state, request) = durable_inventory();
        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");

        let first = runtime
            .enqueue_collection_batch("release:2026.05", 1_000, vec![request.clone()])
            .expect("first batch should persist");
        let second = runtime
            .enqueue_collection_batch("release:2026.05", 1_000, vec![request])
            .expect("second batch should reuse the durable batch");

        assert_eq!(first, second);
        assert_eq!(runtime.pending_commands(), 1);
    }

    #[test]
    fn enqueue_batch_persists_multiple_commands_in_one_history_event() {
        let queue_path = temp_path("durable-runtime-batch-event");
        let (_state, request) = durable_inventory();
        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");

        let command_ids = runtime
            .enqueue_batch(vec![request.clone(), request])
            .expect("batch enqueue should persist");

        assert_eq!(command_ids.len(), 2);
        assert_eq!(runtime.pending_commands(), 2);

        let history = fs::read_to_string(&queue_path).expect("history should be readable");
        assert_eq!(history.lines().count(), 1);
        assert!(history.contains("\"event\":\"batch_enqueued\""));
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

    #[tokio::test]
    async fn applying_scan_command_resumes_without_rescanning_provider() {
        let state_path = temp_path("durable-runtime-apply-state");
        let queue_path = temp_path("durable-runtime-apply-queue");
        let mut state = DurableState::open(&state_path).expect("durable state should open");
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
        let provider = FakeProvider::success(vec![ReportedFinding::new(
            "CVE-2026-0001",
            PackageCoordinate::new("openssl", "3.0.0"),
        )]);
        let report = provider
            .scan(&request)
            .await
            .expect("fixture provider should create one report");

        let mut runtime = ScanCommandQueue::open(&queue_path).expect("runtime should open");
        let enqueue = runtime.enqueue(request).expect("enqueue should persist");
        runtime
            .capture_report(enqueue.command_id.as_ref(), &report)
            .expect("captured report should persist");
        let _ = state
            .record_scan_report_for_command(enqueue.command_id.as_ref(), &report)
            .expect("state application should persist");

        let mut rebuilt_runtime =
            ScanCommandQueue::open(&queue_path).expect("runtime should replay applying state");
        let mut rebuilt_state =
            DurableState::open(&state_path).expect("durable state should replay applied report");

        assert_eq!(
            rebuilt_runtime.command_status(enqueue.command_id.as_ref()),
            Some(ScanCommandStatus::Applying)
        );

        let result = rebuilt_runtime
            .run_next(&mut rebuilt_state, &PanicProvider)
            .await
            .expect("runtime should finalize the applying command");

        assert!(matches!(result, RunNextScanResult::Completed(_)));
        assert_eq!(
            rebuilt_runtime.command_status(enqueue.command_id.as_ref()),
            Some(ScanCommandStatus::Completed)
        );
        assert_eq!(
            rebuilt_state
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            1
        );
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
