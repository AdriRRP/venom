use crate::{
    DurableState, DurableStateError, FindingChangeSet, FindingProvider, ScanRequest,
    validate_provider_scan_report,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimal durable queue for canonical scan requests.
///
/// The runtime is intentionally single-threaded and explicit for now: queue one
/// command, durably record its terminal status, and let higher layers decide
/// about retries or parallelism later.
#[derive(Debug, Clone)]
pub struct DurableScanRuntime {
    history_path: PathBuf,
    commands: BTreeMap<Box<str>, ScanCommandRecord>,
    order: Vec<Box<str>>,
}

impl DurableScanRuntime {
    /// Open or create one local durable scan queue and rebuild it from history.
    ///
    /// # Errors
    ///
    /// Returns [`DurableScanRuntimeError`] when the queue history cannot be
    /// read, parsed, or replayed safely.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, DurableScanRuntimeError> {
        let history_path = path.into();
        if let Some(parent) = history_path.parent() {
            std::fs::create_dir_all(parent).map_err(DurableScanRuntimeError::Io)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&history_path)
            .map_err(DurableScanRuntimeError::Io)?;

        let mut runtime = Self {
            history_path,
            commands: BTreeMap::new(),
            order: Vec::new(),
        };
        runtime.rebuild_from_history()?;
        Ok(runtime)
    }

    /// Durably enqueue one canonical scan request.
    ///
    /// # Errors
    ///
    /// Returns [`DurableScanRuntimeError`] when the queue cannot durably append
    /// the new command event.
    pub fn enqueue(
        &mut self,
        request: ScanRequest,
    ) -> Result<EnqueueScanResult, DurableScanRuntimeError> {
        let command_id = next_command_id();
        self.append_event(&DurableScanEvent::Enqueued {
            command_id: command_id.clone(),
            request: request.clone(),
        })?;
        self.order.push(command_id.clone());
        self.commands.insert(
            command_id.clone(),
            ScanCommandRecord {
                request,
                status: ScanCommandStatus::Pending,
            },
        );
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
    /// Returns [`DurableScanRuntimeError`] when durable queue metadata cannot
    /// be written or replayed safely. Provider and ingestion failures are not
    /// hidden: they are durably recorded and returned as a failed run result.
    pub async fn run_next(
        &mut self,
        state: &mut DurableState,
        provider: &(impl FindingProvider + Sync),
    ) -> Result<RunNextScanResult, DurableScanRuntimeError> {
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
            return Err(DurableScanRuntimeError::CorruptHistory {
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
                        detail: violation.message().into(),
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
                        Err(error) => return Err(DurableScanRuntimeError::State(error)),
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

    fn record_outcome(
        &mut self,
        outcome: &RunNextScanResult,
    ) -> Result<(), DurableScanRuntimeError> {
        match outcome {
            RunNextScanResult::Idle => Ok(()),
            RunNextScanResult::Completed(result) => {
                self.append_event(&DurableScanEvent::Completed {
                    command_id: result.command_id.clone(),
                    provider_key: result.provider_key.clone(),
                    findings_reported: result.findings_reported,
                    change_set: result.change_set.clone(),
                })?;
                let Some(command) = self.commands.get_mut(result.command_id.as_ref()) else {
                    return Err(DurableScanRuntimeError::CorruptHistory {
                        line: 0,
                        reason: "completed scan command missing from in-memory queue".into(),
                    });
                };
                command.status = ScanCommandStatus::Completed;
                Ok(())
            }
            RunNextScanResult::Failed(result) => {
                self.append_event(&DurableScanEvent::Failed {
                    command_id: result.command_id.clone(),
                    error_code: result.error_code.clone(),
                    retryable: result.retryable,
                    detail: result.detail.clone(),
                })?;
                let Some(command) = self.commands.get_mut(result.command_id.as_ref()) else {
                    return Err(DurableScanRuntimeError::CorruptHistory {
                        line: 0,
                        reason: "failed scan command missing from in-memory queue".into(),
                    });
                };
                command.status = ScanCommandStatus::Failed;
                Ok(())
            }
        }
    }

    fn rebuild_from_history(&mut self) -> Result<(), DurableScanRuntimeError> {
        let file = File::open(&self.history_path).map_err(DurableScanRuntimeError::Io)?;
        let reader = BufReader::new(file);
        self.commands.clear();
        self.order.clear();

        for (line_index, line) in reader.lines().enumerate() {
            let line = line.map_err(DurableScanRuntimeError::Io)?;
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str::<DurableScanEvent>(&line).map_err(|error| {
                DurableScanRuntimeError::CorruptHistory {
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
    ) -> Result<(), DurableScanRuntimeError> {
        match event {
            DurableScanEvent::Enqueued {
                command_id,
                request,
            } => {
                if self.commands.contains_key(command_id.as_ref()) {
                    return Err(DurableScanRuntimeError::CorruptHistory {
                        line,
                        reason: "duplicate scan command id".into(),
                    });
                }
                self.order.push(command_id.clone());
                self.commands.insert(
                    command_id,
                    ScanCommandRecord {
                        request,
                        status: ScanCommandStatus::Pending,
                    },
                );
                Ok(())
            }
            DurableScanEvent::Completed { command_id, .. } => {
                self.mark_terminal(line, &command_id, ScanCommandStatus::Completed)
            }
            DurableScanEvent::Failed { command_id, .. } => {
                self.mark_terminal(line, &command_id, ScanCommandStatus::Failed)
            }
        }
    }

    fn mark_terminal(
        &mut self,
        line: usize,
        command_id: &str,
        status: ScanCommandStatus,
    ) -> Result<(), DurableScanRuntimeError> {
        let Some(record) = self.commands.get_mut(command_id) else {
            return Err(DurableScanRuntimeError::CorruptHistory {
                line,
                reason: "terminal event without prior enqueue".into(),
            });
        };
        if record.status != ScanCommandStatus::Pending {
            return Err(DurableScanRuntimeError::CorruptHistory {
                line,
                reason: "duplicate terminal state for scan command".into(),
            });
        }
        record.status = status;
        Ok(())
    }

    fn append_event(&self, event: &DurableScanEvent) -> Result<(), DurableScanRuntimeError> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.history_path)
            .map_err(DurableScanRuntimeError::Io)?;
        serde_json::to_writer(&mut file, event).map_err(DurableScanRuntimeError::Serialize)?;
        file.write_all(b"\n").map_err(DurableScanRuntimeError::Io)?;
        file.flush().map_err(DurableScanRuntimeError::Io)?;
        file.sync_all().map_err(DurableScanRuntimeError::Io)?;
        Ok(())
    }
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
pub enum DurableScanRuntimeError {
    Io(io::Error),
    Serialize(serde_json::Error),
    CorruptHistory { line: usize, reason: Box<str> },
    State(DurableStateError),
}

impl DurableScanRuntimeError {
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

impl core::fmt::Display for DurableScanRuntimeError {
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

impl std::error::Error for DurableScanRuntimeError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum DurableScanEvent {
    Enqueued {
        command_id: Box<str>,
        request: ScanRequest,
    },
    Completed {
        command_id: Box<str>,
        provider_key: Box<str>,
        findings_reported: usize,
        change_set: FindingChangeSet,
    },
    Failed {
        command_id: Box<str>,
        error_code: Box<str>,
        retryable: bool,
        detail: Box<str>,
    },
}

fn next_command_id() -> Box<str> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_nanos();
    format!("scan-command-{nanos}").into_boxed_str()
}

#[cfg(test)]
mod tests {
    use super::{DurableScanRuntime, RunNextScanResult, ScanCommandStatus};
    use crate::{
        ArtifactKind, ArtifactRef, ComponentRegistration, DurableState, EvidenceFreshness,
        FindingProvider, FindingProviderError, FindingProviderErrorKind, PackageCoordinate,
        ProviderScanReport, ReportedFinding, ScanPlanner, ScanRequest,
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
        let mut runtime = DurableScanRuntime::open(&queue_path).expect("runtime should open");
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
        let mut runtime = DurableScanRuntime::open(&queue_path).expect("runtime should open");
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
}
