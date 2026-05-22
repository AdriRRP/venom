use crate::{
    AcceptRiskChange, AcceptRiskResult, AddCollectionComponentChange, AddCollectionComponentResult,
    ArtifactRef, AssignCollectionContextProfileChange, AssignCollectionContextProfileResult,
    AssignComponentTagChange, AssignComponentTagResult, AssignContextProfileChange,
    AssignContextProfileResult, AssignTagContextProfileChange, AssignTagContextProfileResult,
    BindArtifactChange, BindArtifactResult, BulkAcceptRiskResult, BulkGovernanceQuery,
    BulkReopenFindingResult, BulkSuppressFindingResult, CollectionRegistration, CollectionSource,
    CollectionSourceMode, ComponentRegistration, ComponentTagRegistration,
    ConfigureCollectionScanScheduleChange, ConfigureCollectionScanScheduleResult,
    ConfigureCollectionSourceChange, ConfigureCollectionSourceResult,
    ConfigureIntegrationRuntimeChange, ConfigureIntegrationRuntimeResult, ConfigureProviderChange,
    ConfigureProviderResult, ContextProfileRegistration, EvidenceFreshness, FindingChangeSet,
    FindingDecision, FindingGovernance, FindingIngestion, FindingIngestionError, FindingReadModel, FindingRef,
    IntegrationEventPublicationFailure, IntegrationEventPublisher, IntegrationRuntimeConfig,
    MaterializeCollectionSourceChange, MaterializeCollectionSourceResult, PackageCoordinate,
    PendingIntegrationEvent, ProviderScanReport, PublishIntegrationEventsResult,
    RegisterCollectionChange, RegisterCollectionResult, RegisterComponentChange,
    RegisterComponentResult, RegisterComponentTagChange, RegisterComponentTagResult,
    RegisterContextProfileChange, RegisterContextProfileResult, RemoveCollectionComponentChange,
    RemoveCollectionComponentResult, ReopenFindingChange, ReopenFindingResult, ReportedFinding,
    RiskAcceptance, Severity, SuppressFindingChange, SuppressFindingResult, Suppression,
    SystemEvent, SystemEventKind, SystemEventsPage, SystemEventsQuery,
    findings::finding_read_model::canonicalize_reported_findings,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const SYSTEM_EVENT_LOG_CAPACITY: usize = 512;

/// Minimal durable state boundary for the current domain slice.
///
/// The source of truth is a local append-only JSON-lines history. In-memory
/// domain state and read models are reconstructed from that history at open
/// time and are only swapped in after a durable append succeeds.
#[derive(Debug, Clone)]
pub struct DurableState {
    history_path: PathBuf,
    ingestion: FindingIngestion,
    governance: FindingGovernance,
    read_model: FindingReadModel,
    integration_runtime_config: Option<IntegrationRuntimeConfig>,
    applied_scan_commands: BTreeMap<Box<str>, FindingChangeSet>,
    pending_integration_events: VecDeque<PendingIntegrationEvent>,
    system_events: VecDeque<SystemEvent>,
}

impl DurableState {
    /// Open or create one local durable history and rebuild domain state from it.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the history cannot be read, parsed,
    /// or replayed into a valid domain state.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, DurableStateError> {
        let history_path = path.into();
        if let Some(parent) = history_path.parent() {
            std::fs::create_dir_all(parent).map_err(DurableStateError::Io)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&history_path)
            .map_err(DurableStateError::Io)?;

        let mut state = Self {
            history_path,
            ingestion: FindingIngestion::default(),
            governance: FindingGovernance::default(),
            read_model: FindingReadModel::default(),
            integration_runtime_config: None,
            applied_scan_commands: BTreeMap::new(),
            pending_integration_events: VecDeque::new(),
            system_events: VecDeque::new(),
        };
        state.rebuild_from_history()?;
        Ok(state)
    }

    #[must_use]
    pub const fn ingestion(&self) -> &FindingIngestion {
        &self.ingestion
    }

    #[must_use]
    pub const fn read_model(&self) -> &FindingReadModel {
        &self.read_model
    }

    #[must_use]
    pub const fn governance(&self) -> &FindingGovernance {
        &self.governance
    }

    #[must_use]
    pub const fn integration_runtime_config(&self) -> Option<&IntegrationRuntimeConfig> {
        self.integration_runtime_config.as_ref()
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
    /// Returns [`DurableStateError`] when publication outcome persistence fails.
    pub async fn publish_pending_integration_events(
        &mut self,
        max_events: usize,
        publisher: &(impl IntegrationEventPublisher + Sync),
    ) -> Result<PublishIntegrationEventsResult, DurableStateError> {
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
                    self.append_event(&DurableEvent::IntegrationEventPublished {
                        event_id: event.event_id.clone(),
                        occurred_at_unix_ms,
                    })?;
                    self.remove_pending_integration_event(event.event_id.as_ref());
                    result.published += 1;
                    self.push_system_event(SystemEvent {
                        event_id: format!("durable-state-published-live-{occurred_at_unix_ms}")
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
                    self.append_event(&DurableEvent::IntegrationEventPublicationFailed {
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
                        event_id: format!(
                            "durable-state-publish-failed-live-{occurred_at_unix_ms}"
                        )
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

    /// Durably register a managed component.
    ///
    /// The append-only history records only business-relevant state changes.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn register_component(
        &mut self,
        registration: ComponentRegistration,
    ) -> Result<RegisterComponentResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register(registration.clone());
        if result.change == RegisterComponentChange::Registered {
            self.append_event(&DurableEvent::ComponentRegistered {
                registration: StoredComponentRegistration::from(registration),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably register one managed transversal component tag.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn register_component_tag(
        &mut self,
        registration: ComponentTagRegistration,
    ) -> Result<RegisterComponentTagResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register_component_tag(registration.clone());
        if result.change == RegisterComponentTagChange::Registered {
            self.append_event(&DurableEvent::ComponentTagRegistered {
                registration: StoredComponentTagRegistration::from(registration),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably bind one immutable artifact to a managed component.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn bind_artifact(
        &mut self,
        component_key: &str,
        artifact: ArtifactRef,
    ) -> Result<BindArtifactResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.bind_artifact(component_key, artifact.clone());
        if result.change == BindArtifactChange::Bound {
            self.append_event(&DurableEvent::ArtifactBound {
                component_key: component_key.into(),
                artifact,
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably configure one provider runtime for a managed component.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn configure_provider(
        &mut self,
        component_key: &str,
        provider_key: impl Into<Box<str>>,
    ) -> Result<ConfigureProviderResult, DurableStateError> {
        let provider_key = provider_key.into();
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.configure_provider(component_key, provider_key.clone());
        if result.change == ConfigureProviderChange::Configured {
            self.append_event(&DurableEvent::ComponentProviderConfigured {
                component_key: component_key.into(),
                provider_key,
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably register one reusable execution-context profile.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn register_context_profile(
        &mut self,
        registration: ContextProfileRegistration,
    ) -> Result<RegisterContextProfileResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register_context_profile(registration.clone());
        if result.change == RegisterContextProfileChange::Registered {
            self.append_event(&DurableEvent::ContextProfileRegistered {
                registration: StoredContextProfileRegistration::from(registration),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably assign one managed context profile to one managed component.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn assign_context_profile(
        &mut self,
        component_key: &str,
        profile_key: &str,
    ) -> Result<AssignContextProfileResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.assign_context_profile(component_key, profile_key);
        if result.change == AssignContextProfileChange::Assigned {
            self.append_event(&DurableEvent::ComponentContextProfileAssigned {
                component_key: component_key.into(),
                profile_key: profile_key.into(),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably assign one managed tag to one managed component.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn assign_component_tag(
        &mut self,
        tag_key: &str,
        component_key: &str,
    ) -> Result<AssignComponentTagResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.assign_component_tag(tag_key, component_key);
        if result.change == AssignComponentTagChange::Assigned {
            self.append_event(&DurableEvent::ComponentTagged {
                tag_key: tag_key.into(),
                component_key: component_key.into(),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably assign one managed context profile to one managed tag.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn assign_context_profile_for_tag(
        &mut self,
        tag_key: &str,
        profile_key: &str,
    ) -> Result<AssignTagContextProfileResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.assign_context_profile_for_tag(tag_key, profile_key);
        if result.change == AssignTagContextProfileChange::Assigned {
            self.append_event(&DurableEvent::TagContextProfileAssigned {
                tag_key: tag_key.into(),
                profile_key: profile_key.into(),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably assign one managed context profile across one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn assign_context_profile_for_collection(
        &mut self,
        collection_key: &str,
        profile_key: &str,
    ) -> Result<AssignCollectionContextProfileResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result =
            candidate_inventory.assign_context_profile_for_collection(collection_key, profile_key);
        if result.change == AssignCollectionContextProfileChange::Assigned {
            self.append_event(&DurableEvent::CollectionContextProfileAssigned {
                collection_key: collection_key.into(),
                profile_key: profile_key.into(),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably create one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn register_collection(
        &mut self,
        registration: CollectionRegistration,
    ) -> Result<RegisterCollectionResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.register_collection(registration.clone());
        if result.change == RegisterCollectionChange::Created {
            self.append_event(&DurableEvent::CollectionRegistered {
                registration: StoredCollectionRegistration::from(registration),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably add one managed component to one collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn add_component_to_collection(
        &mut self,
        collection_key: &str,
        component_key: &str,
    ) -> Result<AddCollectionComponentResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.add_component_to_collection(collection_key, component_key);
        if result.change == AddCollectionComponentChange::Added {
            self.append_event(&DurableEvent::CollectionComponentAdded {
                collection_key: collection_key.into(),
                component_key: component_key.into(),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably remove one managed component from one collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn remove_component_from_collection(
        &mut self,
        collection_key: &str,
        component_key: &str,
    ) -> Result<RemoveCollectionComponentResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result =
            candidate_inventory.remove_component_from_collection(collection_key, component_key);
        if result.change == RemoveCollectionComponentChange::Removed {
            self.append_event(&DurableEvent::CollectionComponentRemoved {
                collection_key: collection_key.into(),
                component_key: component_key.into(),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably configure one declared source for one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn configure_collection_source(
        &mut self,
        collection_key: &str,
        source: CollectionSource,
    ) -> Result<ConfigureCollectionSourceResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result =
            candidate_inventory.configure_collection_source(collection_key, source.clone());
        if result.change == ConfigureCollectionSourceChange::Configured {
            self.append_event(&DurableEvent::CollectionSourceConfigured {
                collection_key: collection_key.into(),
                source: StoredCollectionSource::from(source),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably materialize one declared source into collection membership.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn materialize_collection_source(
        &mut self,
        collection_key: &str,
    ) -> Result<MaterializeCollectionSourceResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.materialize_collection_source(collection_key);
        if result.change == MaterializeCollectionSourceChange::Materialized {
            self.append_event(&DurableEvent::CollectionSourceMaterialized {
                collection_key: collection_key.into(),
                added_component_keys: result.added_component_keys.clone(),
                removed_component_keys: result.removed_component_keys.clone(),
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably configure one periodic scan schedule for one managed collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn configure_collection_scan_schedule(
        &mut self,
        collection_key: &str,
        cadence_minutes: u32,
        freshness: EvidenceFreshness,
        next_due_at_unix_ms: u64,
    ) -> Result<ConfigureCollectionScanScheduleResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.configure_collection_scan_schedule(
            collection_key,
            cadence_minutes,
            freshness,
            next_due_at_unix_ms,
        );
        if result.change == ConfigureCollectionScanScheduleChange::Configured {
            self.append_event(&DurableEvent::CollectionScanScheduleConfigured {
                collection_key: collection_key.into(),
                cadence_minutes,
                freshness,
                next_due_at_unix_ms,
            })?;
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably record one collection schedule materialization.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn record_collection_scan_materialization(
        &mut self,
        collection_key: &str,
        next_due_at_unix_ms: u64,
        materialized_at_unix_ms: u64,
        enqueued_commands: u32,
    ) -> Result<ConfigureCollectionScanScheduleResult, DurableStateError> {
        let mut candidate_inventory = self.ingestion.inventory().clone();
        let result = candidate_inventory.record_collection_scan_materialization(
            collection_key,
            next_due_at_unix_ms,
            materialized_at_unix_ms,
            enqueued_commands,
        );
        if result.change == ConfigureCollectionScanScheduleChange::Configured {
            self.append_event(&DurableEvent::CollectionScanScheduleMaterialized {
                collection_key: collection_key.into(),
                next_due_at_unix_ms,
                materialized_at_unix_ms,
                enqueued_commands,
            })?;
            self.push_system_event(SystemEvent {
                event_id: format!(
                    "durable-state-scheduler-live-{collection_key}-{materialized_at_unix_ms}"
                )
                .into_boxed_str(),
                occurred_at_unix_ms: materialized_at_unix_ms,
                kind: SystemEventKind::CollectionScanMaterialized,
                collection_key: Some(collection_key.into()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: Some(enqueued_commands),
                retryable: None,
                detail: Some(
                    format!("next due {next_due_at_unix_ms}, enqueued {enqueued_commands}")
                        .into_boxed_str(),
                ),
            });
            *self.ingestion.inventory_mut() = candidate_inventory;
        }
        Ok(result)
    }

    /// Durably configure the integration publication runtime.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn configure_integration_runtime(
        &mut self,
        config: IntegrationRuntimeConfig,
    ) -> Result<ConfigureIntegrationRuntimeResult, DurableStateError> {
        let change = if self.integration_runtime_config.as_ref() == Some(&config) {
            ConfigureIntegrationRuntimeChange::Unchanged
        } else {
            self.append_event(&DurableEvent::IntegrationRuntimeConfigured {
                config: config.clone(),
            })?;
            self.integration_runtime_config = Some(config.clone());
            ConfigureIntegrationRuntimeChange::Configured
        };
        Ok(ConfigureIntegrationRuntimeResult { change, config })
    }

    /// Durably record one accepted provider snapshot and update the projection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::Ingestion`] when the report violates
    /// managed ownership, or another [`DurableStateError`] when the durable
    /// append or history format fails.
    pub fn record_scan_report(
        &mut self,
        report: &ProviderScanReport,
    ) -> Result<FindingChangeSet, DurableStateError> {
        self.record_scan_report_internal(None, report)
    }

    /// Durably record one scan report for one canonical scan command.
    ///
    /// Repeated calls with the same `command_id` reuse the already durable
    /// change set instead of mutating findings again.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError`] when the durable append fails.
    pub fn record_scan_report_for_command(
        &mut self,
        command_id: &str,
        report: &ProviderScanReport,
    ) -> Result<FindingChangeSet, DurableStateError> {
        if let Some(change_set) = self.applied_scan_commands.get(command_id) {
            return Ok(change_set.clone());
        }
        self.record_scan_report_internal(Some(command_id), report)
    }

    fn record_scan_report_internal(
        &mut self,
        command_id: Option<&str>,
        report: &ProviderScanReport,
    ) -> Result<FindingChangeSet, DurableStateError> {
        let mut candidate_ingestion = self.ingestion.clone();
        let mut candidate_read_model = self.read_model.clone();
        let change_set = candidate_ingestion
            .record_scan_report(report)
            .map_err(DurableStateError::Ingestion)?;
        candidate_read_model.record_scan_report(report);
        let pending_integration_event = PendingIntegrationEvent::finding_changes_observed(
            report.component_key.clone(),
            report.artifact.clone(),
            report.provider_key.clone(),
            report.freshness,
            report.observed_at,
            change_set.clone(),
        );
        self.append_event(&DurableEvent::ProviderScanRecorded {
            command_id: command_id.map(Into::into),
            report: StoredProviderScanReport::from_report(report)?,
            change_set: Some(change_set.clone()),
            pending_integration_event: Box::new(Some(pending_integration_event.clone())),
        })?;
        self.ingestion = candidate_ingestion;
        self.read_model = candidate_read_model;
        if let Some(command_id) = command_id {
            self.applied_scan_commands
                .insert(command_id.into(), change_set.clone());
        }
        self.pending_integration_events
            .push_back(pending_integration_event);
        Ok(change_set)
    }

    /// Durably accept the risk of one currently active finding.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingFinding`] when the finding is not
    /// currently active, or another [`DurableStateError`] when the durable
    /// append fails.
    pub fn accept_risk(
        &mut self,
        finding: FindingRef,
        acceptance: RiskAcceptance,
    ) -> Result<AcceptRiskResult, DurableStateError> {
        if !self.read_model.has_active_finding(&finding) {
            return Err(DurableStateError::MissingFinding(
                "cannot accept risk for an inactive finding".into(),
            ));
        }

        let mut candidate_governance = self.governance.clone();
        let mut candidate_read_model = self.read_model.clone();
        let result = candidate_governance.accept_risk(finding.clone(), acceptance.clone());
        if result.change == AcceptRiskChange::Accepted {
            let component_key = finding.component_key.clone();
            let detail = acceptance.reason.clone();
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::FindingRiskAccepted {
                finding: StoredFindingRef::from(finding.clone()),
                acceptance: acceptance.clone(),
                occurred_at_unix_ms,
            })?;
            candidate_read_model.accept_risk(finding, acceptance);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.push_system_event(SystemEvent {
                event_id: format!("durable-state-risk-accepted-live-{occurred_at_unix_ms}")
                    .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingRiskAccepted,
                collection_key: None,
                component_key: Some(component_key),
                command_id: None,
                integration_event_id: None,
                finding_count: Some(1),
                retryable: None,
                detail: Some(detail),
            });
        }

        Ok(result)
    }

    /// Durably accept risk for all open active findings matched inside one collection scope.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingCollection`] when the collection is
    /// unknown, or another [`DurableStateError`] when the durable append fails.
    pub fn accept_risk_for_collection(
        &mut self,
        collection_key: &str,
        query: &BulkGovernanceQuery,
        acceptance: RiskAcceptance,
    ) -> Result<BulkAcceptRiskResult, DurableStateError> {
        let scope = self
            .ingestion
            .inventory()
            .collection_scoped_artifacts(collection_key)
            .ok_or_else(|| {
                DurableStateError::MissingCollection(
                    format!("unknown collection: {collection_key}").into_boxed_str(),
                )
            })?;
        let findings = self
            .read_model
            .collect_bulk_governance_finding_refs(&scope, query);
        let targeted = findings.len();

        let changed_findings = findings
            .into_iter()
            .filter(|finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::RiskAccepted(existing)) if existing == &acceptance
                )
            })
            .map(StoredFindingRef::from)
            .collect::<Vec<_>>();

        let accepted = changed_findings.len();
        if accepted > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::FindingsRiskAccepted {
                collection_key: collection_key.into(),
                findings: changed_findings.clone(),
                acceptance: acceptance.clone(),
                occurred_at_unix_ms,
            })?;
            for finding in changed_findings
                .iter()
                .cloned()
                .map(StoredFindingRef::into_domain)
            {
                self.governance.accept_risk(finding.clone(), acceptance.clone());
                self.read_model.accept_risk(finding, acceptance.clone());
            }
            self.push_system_event(SystemEvent {
                event_id: format!(
                    "durable-state-risk-accepted-many-live-{collection_key}-{occurred_at_unix_ms}"
                )
                .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsRiskAccepted,
                collection_key: Some(collection_key.into()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(accepted).ok(),
                retryable: None,
                detail: Some(acceptance.reason.clone()),
            });
        }

        Ok(BulkAcceptRiskResult {
            targeted,
            accepted,
            unchanged: targeted.saturating_sub(accepted),
            acceptance,
        })
    }

    /// Durably accept risk for all open active findings matched inside one tag scope.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingTag`] when the tag is unknown, or
    /// another [`DurableStateError`] when the durable append fails.
    pub fn accept_risk_for_tag(
        &mut self,
        tag_key: &str,
        query: &BulkGovernanceQuery,
        acceptance: RiskAcceptance,
    ) -> Result<BulkAcceptRiskResult, DurableStateError> {
        let scope = self
            .ingestion
            .inventory()
            .tag_scoped_artifacts(tag_key)
            .ok_or_else(|| DurableStateError::MissingTag(tag_key.into()))?;
        let findings = self
            .read_model
            .collect_bulk_governance_finding_refs(&scope, query);
        let targeted = findings.len();

        let changed_findings = findings
            .into_iter()
            .filter(|finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::RiskAccepted(existing)) if existing == &acceptance
                )
            })
            .map(StoredFindingRef::from)
            .collect::<Vec<_>>();

        let accepted = changed_findings.len();
        if accepted > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::TagFindingsRiskAccepted {
                tag_key: tag_key.into(),
                findings: changed_findings.clone(),
                acceptance: acceptance.clone(),
                occurred_at_unix_ms,
            })?;
            for finding in changed_findings
                .iter()
                .cloned()
                .map(StoredFindingRef::into_domain)
            {
                self.governance.accept_risk(finding.clone(), acceptance.clone());
                self.read_model.accept_risk(finding, acceptance.clone());
            }
            self.push_system_event(SystemEvent {
                event_id: format!(
                    "durable-state-tag-risk-accepted-live-{tag_key}-{occurred_at_unix_ms}"
                )
                .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsRiskAccepted,
                collection_key: None,
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(accepted).ok(),
                retryable: None,
                detail: Some(format!("tag {tag_key}: {}", acceptance.reason).into_boxed_str()),
            });
        }

        Ok(BulkAcceptRiskResult {
            targeted,
            accepted,
            unchanged: targeted.saturating_sub(accepted),
            acceptance,
        })
    }

    /// Durably reopen one governed active finding back to the canonical open state.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingFinding`] when the finding is not
    /// currently active, or another [`DurableStateError`] when the durable
    /// append fails.
    pub fn reopen_finding(
        &mut self,
        finding: &FindingRef,
    ) -> Result<ReopenFindingResult, DurableStateError> {
        if !self.read_model.has_active_finding(finding) {
            return Err(DurableStateError::MissingFinding(
                "cannot reopen an inactive finding".into(),
            ));
        }

        let mut candidate_governance = self.governance.clone();
        let mut candidate_read_model = self.read_model.clone();
        let result = candidate_governance.reopen(finding);
        if result.change == ReopenFindingChange::Reopened {
            let component_key = finding.component_key.clone();
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::FindingReopened {
                finding: StoredFindingRef::from(finding.clone()),
                occurred_at_unix_ms,
            })?;
            candidate_read_model.reopen(finding);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.push_system_event(SystemEvent {
                event_id: format!("durable-state-reopened-live-{occurred_at_unix_ms}")
                    .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingReopened,
                collection_key: None,
                component_key: Some(component_key),
                command_id: None,
                integration_event_id: None,
                finding_count: Some(1),
                retryable: None,
                detail: None,
            });
        }

        Ok(result)
    }

    /// Durably suppress one currently active finding.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingFinding`] when the finding is not
    /// currently active, or another [`DurableStateError`] when the durable
    /// append fails.
    pub fn suppress_finding(
        &mut self,
        finding: FindingRef,
        suppression: Suppression,
    ) -> Result<SuppressFindingResult, DurableStateError> {
        if !self.read_model.has_active_finding(&finding) {
            return Err(DurableStateError::MissingFinding(
                "cannot suppress an inactive finding".into(),
            ));
        }

        let mut candidate_governance = self.governance.clone();
        let mut candidate_read_model = self.read_model.clone();
        let result = candidate_governance.suppress(finding.clone(), suppression.clone());
        if result.change == SuppressFindingChange::Suppressed {
            let component_key = finding.component_key.clone();
            let detail = suppression.reason.clone();
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::FindingSuppressed {
                finding: StoredFindingRef::from(finding.clone()),
                suppression: suppression.clone(),
                occurred_at_unix_ms,
            })?;
            candidate_read_model.suppress(finding, suppression);
            self.governance = candidate_governance;
            self.read_model = candidate_read_model;
            self.push_system_event(SystemEvent {
                event_id: format!("durable-state-suppressed-live-{occurred_at_unix_ms}")
                    .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingSuppressed,
                collection_key: None,
                component_key: Some(component_key),
                command_id: None,
                integration_event_id: None,
                finding_count: Some(1),
                retryable: None,
                detail: Some(detail),
            });
        }

        Ok(result)
    }

    /// Durably suppress one filtered open cohort of findings inside one collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingCollection`] when the collection is
    /// unknown, or another [`DurableStateError`] when the durable append fails.
    pub fn suppress_findings_for_collection(
        &mut self,
        collection_key: &str,
        query: &BulkGovernanceQuery,
        suppression: Suppression,
    ) -> Result<BulkSuppressFindingResult, DurableStateError> {
        let scope = self
            .ingestion
            .inventory()
            .collection_scoped_artifacts(collection_key)
            .ok_or_else(|| DurableStateError::MissingCollection(collection_key.into()))?;
        let findings = self
            .read_model
            .collect_bulk_governance_finding_refs(&scope, query);
        let targeted = findings.len();

        let changed_findings = findings
            .into_iter()
            .filter(|finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::Suppressed(existing)) if existing == &suppression
                )
            })
            .map(StoredFindingRef::from)
            .collect::<Vec<_>>();

        let suppressed = changed_findings.len();
        if suppressed > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::FindingsSuppressed {
                collection_key: collection_key.into(),
                findings: changed_findings.clone(),
                suppression: suppression.clone(),
                occurred_at_unix_ms,
            })?;
            for finding in changed_findings
                .iter()
                .cloned()
                .map(StoredFindingRef::into_domain)
            {
                self.governance.suppress(finding.clone(), suppression.clone());
                self.read_model.suppress(finding, suppression.clone());
            }
            self.push_system_event(SystemEvent {
                event_id: format!(
                    "durable-state-suppressed-many-live-{collection_key}-{occurred_at_unix_ms}"
                )
                .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsSuppressed,
                collection_key: Some(collection_key.into()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(suppressed).ok(),
                retryable: None,
                detail: Some(suppression.reason.clone()),
            });
        }

        Ok(BulkSuppressFindingResult {
            targeted,
            suppressed,
            unchanged: targeted.saturating_sub(suppressed),
            suppression,
        })
    }

    /// Durably suppress all open active findings matched inside one tag scope.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingTag`] when the tag is unknown, or
    /// another [`DurableStateError`] when the durable append fails.
    pub fn suppress_findings_for_tag(
        &mut self,
        tag_key: &str,
        query: &BulkGovernanceQuery,
        suppression: Suppression,
    ) -> Result<BulkSuppressFindingResult, DurableStateError> {
        let scope = self
            .ingestion
            .inventory()
            .tag_scoped_artifacts(tag_key)
            .ok_or_else(|| DurableStateError::MissingTag(tag_key.into()))?;
        let findings = self
            .read_model
            .collect_bulk_governance_finding_refs(&scope, query);
        let targeted = findings.len();

        let changed_findings = findings
            .into_iter()
            .filter(|finding| {
                !matches!(
                    self.governance.decision(finding),
                    Some(FindingDecision::Suppressed(existing)) if existing == &suppression
                )
            })
            .map(StoredFindingRef::from)
            .collect::<Vec<_>>();

        let suppressed = changed_findings.len();
        if suppressed > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::TagFindingsSuppressed {
                tag_key: tag_key.into(),
                findings: changed_findings.clone(),
                suppression: suppression.clone(),
                occurred_at_unix_ms,
            })?;
            for finding in changed_findings
                .iter()
                .cloned()
                .map(StoredFindingRef::into_domain)
            {
                self.governance.suppress(finding.clone(), suppression.clone());
                self.read_model.suppress(finding, suppression.clone());
            }
            self.push_system_event(SystemEvent {
                event_id: format!(
                    "durable-state-tag-suppressed-live-{tag_key}-{occurred_at_unix_ms}"
                )
                .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsSuppressed,
                collection_key: None,
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(suppressed).ok(),
                retryable: None,
                detail: Some(format!("tag {tag_key}: {}", suppression.reason).into_boxed_str()),
            });
        }

        Ok(BulkSuppressFindingResult {
            targeted,
            suppressed,
            unchanged: targeted.saturating_sub(suppressed),
            suppression,
        })
    }

    /// Durably reopen one filtered governed cohort of findings inside one collection.
    ///
    /// # Errors
    ///
    /// Returns [`DurableStateError::MissingCollection`] when the collection is
    /// unknown, or another [`DurableStateError`] when the durable append fails.
    pub fn reopen_findings_for_collection(
        &mut self,
        collection_key: &str,
        query: &BulkGovernanceQuery,
    ) -> Result<BulkReopenFindingResult, DurableStateError> {
        let scope = self
            .ingestion
            .inventory()
            .collection_scoped_artifacts(collection_key)
            .ok_or_else(|| DurableStateError::MissingCollection(collection_key.into()))?;
        let findings = self
            .read_model
            .collect_bulk_governance_finding_refs(&scope, query);
        let targeted = findings.len();

        let reopened_findings = findings
            .into_iter()
            .filter(|finding| self.governance.decision(finding).is_some())
            .map(StoredFindingRef::from)
            .collect::<Vec<_>>();

        let reopened = reopened_findings.len();
        if reopened > 0 {
            let occurred_at_unix_ms = current_unix_millis()?;
            self.append_event(&DurableEvent::FindingsReopened {
                collection_key: collection_key.into(),
                findings: reopened_findings.clone(),
                occurred_at_unix_ms,
            })?;
            for finding in reopened_findings
                .iter()
                .cloned()
                .map(StoredFindingRef::into_domain)
            {
                self.governance.reopen(&finding);
                self.read_model.reopen(&finding);
            }
            self.push_system_event(SystemEvent {
                event_id: format!(
                    "durable-state-reopened-many-live-{collection_key}-{occurred_at_unix_ms}"
                )
                .into_boxed_str(),
                occurred_at_unix_ms,
                kind: SystemEventKind::FindingsReopened,
                collection_key: Some(collection_key.into()),
                component_key: None,
                command_id: None,
                integration_event_id: None,
                finding_count: u32::try_from(reopened).ok(),
                retryable: None,
                detail: None,
            });
        }

        Ok(BulkReopenFindingResult {
            targeted,
            reopened,
            unchanged: targeted.saturating_sub(reopened),
        })
    }

    fn rebuild_from_history(&mut self) -> Result<(), DurableStateError> {
        let file = File::open(&self.history_path).map_err(DurableStateError::Io)?;
        let reader = BufReader::new(file);
        self.ingestion = FindingIngestion::default();
        self.governance = FindingGovernance::default();
        self.read_model = FindingReadModel::default();
        self.integration_runtime_config = None;
        self.applied_scan_commands.clear();
        self.pending_integration_events.clear();
        self.system_events.clear();

        for (line_index, line) in reader.lines().enumerate() {
            let line = line.map_err(DurableStateError::Io)?;
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str::<DurableEvent>(&line).map_err(|error| {
                DurableStateError::CorruptHistory {
                    line: line_index + 1,
                    reason: error.to_string().into_boxed_str(),
                }
            })?;
            self.apply_event(event, line_index + 1)?;
        }

        Ok(())
    }

    fn apply_event(&mut self, event: DurableEvent, line: usize) -> Result<(), DurableStateError> {
        match event {
            DurableEvent::ComponentRegistered { .. }
            | DurableEvent::ComponentTagRegistered { .. }
            | DurableEvent::ContextProfileRegistered { .. }
            | DurableEvent::ArtifactBound { .. }
            | DurableEvent::ComponentProviderConfigured { .. }
            | DurableEvent::ComponentContextProfileAssigned { .. }
            | DurableEvent::ComponentTagged { .. }
            | DurableEvent::TagContextProfileAssigned { .. }
            | DurableEvent::CollectionContextProfileAssigned { .. }
            | DurableEvent::CollectionRegistered { .. }
            | DurableEvent::CollectionComponentAdded { .. }
            | DurableEvent::CollectionComponentRemoved { .. }
            | DurableEvent::CollectionSourceConfigured { .. }
            | DurableEvent::CollectionSourceMaterialized { .. }
            | DurableEvent::CollectionScanScheduleConfigured { .. }
            | DurableEvent::CollectionScanScheduleMaterialized { .. } => {
                self.apply_inventory_event(event, line)
            }
            DurableEvent::IntegrationRuntimeConfigured { config } => {
                self.integration_runtime_config = Some(config);
                Ok(())
            }
            DurableEvent::ProviderScanRecorded {
                command_id,
                report,
                change_set,
                pending_integration_event,
            } => self.apply_provider_scan_recorded(
                command_id,
                report,
                change_set,
                *pending_integration_event,
                line,
            ),
            DurableEvent::FindingRiskAccepted { .. }
            | DurableEvent::FindingsRiskAccepted { .. }
            | DurableEvent::TagFindingsRiskAccepted { .. }
            | DurableEvent::FindingSuppressed { .. }
            | DurableEvent::FindingReopened { .. }
            | DurableEvent::FindingsSuppressed { .. }
            | DurableEvent::TagFindingsSuppressed { .. }
            | DurableEvent::FindingsReopened { .. } => {
                self.apply_governance_event(event, line);
                Ok(())
            }
            DurableEvent::IntegrationEventPublished {
                event_id,
                occurred_at_unix_ms,
            } => {
                self.apply_published_event(event_id, occurred_at_unix_ms, line);
                Ok(())
            }
            DurableEvent::IntegrationEventPublicationFailed {
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
        }
    }

    fn apply_governance_event(&mut self, event: DurableEvent, line: usize) {
        match event {
            DurableEvent::FindingRiskAccepted {
                finding,
                acceptance,
                occurred_at_unix_ms,
            } => self.apply_risk_accepted_event(finding, acceptance, occurred_at_unix_ms, line),
            DurableEvent::FindingsRiskAccepted {
                collection_key,
                findings,
                acceptance,
                occurred_at_unix_ms,
            } => self.apply_risk_accepted_many_event(
                collection_key,
                findings,
                acceptance,
                occurred_at_unix_ms,
                line,
            ),
            DurableEvent::TagFindingsRiskAccepted {
                tag_key,
                findings,
                acceptance,
                occurred_at_unix_ms,
            } => self.apply_tag_risk_accepted_many_event(
                &tag_key,
                findings,
                &acceptance,
                occurred_at_unix_ms,
                line,
            ),
            DurableEvent::FindingSuppressed {
                finding,
                suppression,
                occurred_at_unix_ms,
            } => self.apply_suppressed_event(finding, suppression, occurred_at_unix_ms, line),
            DurableEvent::FindingReopened {
                finding,
                occurred_at_unix_ms,
            } => self.apply_reopened_event(&finding, occurred_at_unix_ms, line),
            DurableEvent::FindingsSuppressed {
                collection_key,
                findings,
                suppression,
                occurred_at_unix_ms,
            } => self.apply_suppressed_many_event(
                collection_key,
                findings,
                suppression,
                occurred_at_unix_ms,
                line,
            ),
            DurableEvent::TagFindingsSuppressed {
                tag_key,
                findings,
                suppression,
                occurred_at_unix_ms,
            } => self.apply_tag_suppressed_many_event(
                &tag_key,
                findings,
                &suppression,
                occurred_at_unix_ms,
                line,
            ),
            DurableEvent::FindingsReopened {
                collection_key,
                findings,
                occurred_at_unix_ms,
            } => {
                self.apply_reopened_many_event(
                    collection_key,
                    &findings,
                    occurred_at_unix_ms,
                    line,
                );
            }
            _ => unreachable!("only governance durable events belong in apply_governance_event"),
        }
    }

    fn apply_inventory_event(
        &mut self,
        event: DurableEvent,
        line: usize,
    ) -> Result<(), DurableStateError> {
        match event {
            DurableEvent::ComponentRegistered { registration } => {
                self.apply_component_registered(registration, line)
            }
            DurableEvent::ComponentTagRegistered { registration } => {
                self.apply_component_tag_registered(registration, line)
            }
            DurableEvent::ArtifactBound {
                component_key,
                artifact,
            } => self.apply_artifact_bound(component_key.as_ref(), artifact, line),
            DurableEvent::ComponentProviderConfigured {
                component_key,
                provider_key,
            } => {
                self.apply_provider_configured(component_key.as_ref(), provider_key.as_ref(), line)
            }
            DurableEvent::ContextProfileRegistered { registration } => {
                self.apply_context_profile_registered(registration, line)
            }
            DurableEvent::ComponentContextProfileAssigned {
                component_key,
                profile_key,
            } => self.apply_context_profile_assigned(
                component_key.as_ref(),
                profile_key.as_ref(),
                line,
            ),
            DurableEvent::ComponentTagged {
                tag_key,
                component_key,
            } => self.apply_component_tagged(tag_key.as_ref(), component_key.as_ref(), line),
            DurableEvent::TagContextProfileAssigned {
                tag_key,
                profile_key,
            } => self.apply_tag_context_profile_assigned(
                tag_key.as_ref(),
                profile_key.as_ref(),
                line,
            ),
            DurableEvent::CollectionContextProfileAssigned {
                collection_key,
                profile_key,
            } => self.apply_collection_context_profile_assigned(
                collection_key.as_ref(),
                profile_key.as_ref(),
                line,
            ),
            DurableEvent::CollectionRegistered { .. }
            | DurableEvent::CollectionComponentAdded { .. }
            | DurableEvent::CollectionComponentRemoved { .. } => {
                self.apply_collection_membership_event(event, line)
            }
            DurableEvent::CollectionSourceConfigured { .. }
            | DurableEvent::CollectionSourceMaterialized { .. } => {
                self.apply_collection_source_event(event, line)
            }
            DurableEvent::CollectionScanScheduleConfigured { .. }
            | DurableEvent::CollectionScanScheduleMaterialized { .. } => {
                self.apply_collection_schedule_event(event, line)
            }
            DurableEvent::IntegrationRuntimeConfigured { .. }
            | DurableEvent::ProviderScanRecorded { .. }
            | DurableEvent::FindingRiskAccepted { .. }
            | DurableEvent::FindingsRiskAccepted { .. }
            | DurableEvent::TagFindingsRiskAccepted { .. }
            | DurableEvent::FindingSuppressed { .. }
            | DurableEvent::TagFindingsSuppressed { .. }
            | DurableEvent::FindingsSuppressed { .. }
            | DurableEvent::FindingReopened { .. }
            | DurableEvent::FindingsReopened { .. }
            | DurableEvent::IntegrationEventPublished { .. }
            | DurableEvent::IntegrationEventPublicationFailed { .. } => {
                unreachable!("non-inventory durable event routed to inventory replay")
            }
        }
    }

    fn apply_collection_membership_event(
        &mut self,
        event: DurableEvent,
        line: usize,
    ) -> Result<(), DurableStateError> {
        match event {
            DurableEvent::CollectionRegistered { registration } => {
                self.apply_collection_registered(registration, line)
            }
            DurableEvent::CollectionComponentAdded {
                collection_key,
                component_key,
            } => self.apply_collection_component_added(
                collection_key.as_ref(),
                component_key.as_ref(),
                line,
            ),
            DurableEvent::CollectionComponentRemoved {
                collection_key,
                component_key,
            } => self.apply_collection_component_removed(
                collection_key.as_ref(),
                component_key.as_ref(),
                line,
            ),
            _ => unreachable!("non-membership event routed to membership replay"),
        }
    }

    fn apply_collection_source_event(
        &mut self,
        event: DurableEvent,
        line: usize,
    ) -> Result<(), DurableStateError> {
        match event {
            DurableEvent::CollectionSourceConfigured {
                collection_key,
                source,
            } => self.apply_collection_source_configured(
                collection_key.as_ref(),
                source.into_domain(),
                line,
            ),
            DurableEvent::CollectionSourceMaterialized {
                collection_key,
                added_component_keys,
                removed_component_keys,
            } => self.apply_collection_source_materialized(
                collection_key.as_ref(),
                added_component_keys,
                removed_component_keys,
                line,
            ),
            _ => unreachable!("non-source event routed to source replay"),
        }
    }

    fn apply_collection_schedule_event(
        &mut self,
        event: DurableEvent,
        line: usize,
    ) -> Result<(), DurableStateError> {
        match event {
            DurableEvent::CollectionScanScheduleConfigured {
                collection_key,
                cadence_minutes,
                freshness,
                next_due_at_unix_ms,
            } => self.apply_collection_scan_schedule_configured(
                collection_key.as_ref(),
                cadence_minutes,
                freshness,
                next_due_at_unix_ms,
                line,
            ),
            DurableEvent::CollectionScanScheduleMaterialized {
                collection_key,
                next_due_at_unix_ms,
                materialized_at_unix_ms,
                enqueued_commands,
            } => {
                self.apply_collection_scan_schedule_materialized(
                    collection_key.as_ref(),
                    next_due_at_unix_ms,
                    materialized_at_unix_ms,
                    enqueued_commands,
                    line,
                )?;
                self.push_system_event(SystemEvent {
                    event_id: format!("durable-state-scheduler-{line}").into_boxed_str(),
                    occurred_at_unix_ms: materialized_at_unix_ms,
                    kind: SystemEventKind::CollectionScanMaterialized,
                    collection_key: Some(collection_key),
                    component_key: None,
                    command_id: None,
                    integration_event_id: None,
                    finding_count: Some(enqueued_commands),
                    retryable: None,
                    detail: Some(
                        format!("next due {next_due_at_unix_ms}, enqueued {enqueued_commands}")
                            .into_boxed_str(),
                    ),
                });
                Ok(())
            }
            _ => unreachable!("non-schedule event routed to schedule replay"),
        }
    }

    fn apply_risk_accepted_event(
        &mut self,
        finding: StoredFindingRef,
        acceptance: RiskAcceptance,
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let component_key = finding.component_key.clone();
        let detail = acceptance.reason.clone();
        self.apply_finding_risk_accepted(finding, acceptance);
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-risk-accepted-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingRiskAccepted,
            collection_key: None,
            component_key: Some(component_key),
            command_id: None,
            integration_event_id: None,
            finding_count: Some(1),
            retryable: None,
            detail: Some(detail),
        });
    }

    fn apply_risk_accepted_many_event(
        &mut self,
        collection_key: Box<str>,
        findings: Vec<StoredFindingRef>,
        acceptance: RiskAcceptance,
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let finding_count = u32::try_from(findings.len()).ok();
        for finding in findings {
            self.apply_finding_risk_accepted(finding, acceptance.clone());
        }
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-risk-accepted-many-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingsRiskAccepted,
            collection_key: Some(collection_key),
            component_key: None,
            command_id: None,
            integration_event_id: None,
            finding_count,
            retryable: None,
            detail: Some(acceptance.reason),
        });
    }

    fn apply_suppressed_event(
        &mut self,
        finding: StoredFindingRef,
        suppression: Suppression,
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let component_key = finding.component_key.clone();
        let detail = suppression.reason.clone();
        self.apply_finding_suppressed(finding, suppression);
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-suppressed-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingSuppressed,
            collection_key: None,
            component_key: Some(component_key),
            command_id: None,
            integration_event_id: None,
            finding_count: Some(1),
            retryable: None,
            detail: Some(detail),
        });
    }

    fn apply_suppressed_many_event(
        &mut self,
        collection_key: Box<str>,
        findings: Vec<StoredFindingRef>,
        suppression: Suppression,
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let finding_count = u32::try_from(findings.len()).ok();
        for finding in findings {
            self.apply_finding_suppressed(finding, suppression.clone());
        }
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-suppressed-many-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingsSuppressed,
            collection_key: Some(collection_key),
            component_key: None,
            command_id: None,
            integration_event_id: None,
            finding_count,
            retryable: None,
            detail: Some(suppression.reason),
        });
    }

    fn apply_tag_risk_accepted_many_event(
        &mut self,
        tag_key: &str,
        findings: Vec<StoredFindingRef>,
        acceptance: &RiskAcceptance,
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let finding_count = u32::try_from(findings.len()).ok();
        for finding in findings {
            self.apply_finding_risk_accepted(finding, acceptance.clone());
        }
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-tag-risk-accepted-many-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingsRiskAccepted,
            collection_key: None,
            component_key: None,
            command_id: None,
            integration_event_id: None,
            finding_count,
            retryable: None,
            detail: Some(format!("tag {tag_key}: {}", acceptance.reason).into_boxed_str()),
        });
    }

    fn apply_tag_suppressed_many_event(
        &mut self,
        tag_key: &str,
        findings: Vec<StoredFindingRef>,
        suppression: &Suppression,
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let finding_count = u32::try_from(findings.len()).ok();
        for finding in findings {
            self.apply_finding_suppressed(finding, suppression.clone());
        }
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-tag-suppressed-many-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingsSuppressed,
            collection_key: None,
            component_key: None,
            command_id: None,
            integration_event_id: None,
            finding_count,
            retryable: None,
            detail: Some(format!("tag {tag_key}: {}", suppression.reason).into_boxed_str()),
        });
    }

    fn apply_reopened_event(
        &mut self,
        finding: &StoredFindingRef,
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let component_key = finding.component_key.clone();
        self.apply_finding_reopened(finding);
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-reopened-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingReopened,
            collection_key: None,
            component_key: Some(component_key),
            command_id: None,
            integration_event_id: None,
            finding_count: Some(1),
            retryable: None,
            detail: None,
        });
    }

    fn apply_reopened_many_event(
        &mut self,
        collection_key: Box<str>,
        findings: &[StoredFindingRef],
        occurred_at_unix_ms: u64,
        line: usize,
    ) {
        let finding_count = u32::try_from(findings.len()).ok();
        for finding in findings {
            self.apply_finding_reopened(finding);
        }
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-reopened-many-{line}").into_boxed_str(),
            occurred_at_unix_ms,
            kind: SystemEventKind::FindingsReopened,
            collection_key: Some(collection_key),
            component_key: None,
            command_id: None,
            integration_event_id: None,
            finding_count,
            retryable: None,
            detail: None,
        });
    }

    fn apply_published_event(&mut self, event_id: Box<str>, occurred_at_unix_ms: u64, line: usize) {
        self.remove_pending_integration_event(event_id.as_ref());
        self.push_system_event(SystemEvent {
            event_id: format!("durable-state-published-{line}").into_boxed_str(),
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
            event_id: format!("durable-state-publish-failed-{line}").into_boxed_str(),
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

    fn apply_component_registered(
        &mut self,
        registration: StoredComponentRegistration,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .register(registration.into_domain());
        match result.change {
            RegisterComponentChange::Registered | RegisterComponentChange::Unchanged => Ok(()),
            RegisterComponentChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "conflicting component registration".into(),
            }),
        }
    }

    fn apply_component_tag_registered(
        &mut self,
        registration: StoredComponentTagRegistration,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .register_component_tag(registration.into_domain());
        match result.change {
            RegisterComponentTagChange::Registered | RegisterComponentTagChange::Unchanged => {
                Ok(())
            }
            RegisterComponentTagChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "conflicting component tag registration".into(),
            }),
        }
    }

    fn apply_artifact_bound(
        &mut self,
        component_key: &str,
        artifact: ArtifactRef,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .bind_artifact(component_key, artifact);
        match result.change {
            BindArtifactChange::Bound | BindArtifactChange::Unchanged => Ok(()),
            BindArtifactChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid artifact binding".into(),
            }),
        }
    }

    fn apply_provider_configured(
        &mut self,
        component_key: &str,
        provider_key: &str,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .configure_provider(component_key, provider_key);
        match result.change {
            ConfigureProviderChange::Configured | ConfigureProviderChange::Unchanged => Ok(()),
            ConfigureProviderChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid provider configuration".into(),
            }),
        }
    }

    fn apply_collection_registered(
        &mut self,
        registration: StoredCollectionRegistration,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .register_collection(registration.into_domain());
        match result.change {
            RegisterCollectionChange::Created | RegisterCollectionChange::Unchanged => Ok(()),
            RegisterCollectionChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "conflicting collection registration".into(),
            }),
        }
    }

    fn apply_context_profile_registered(
        &mut self,
        registration: StoredContextProfileRegistration,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .register_context_profile(registration.into_domain());
        match result.change {
            RegisterContextProfileChange::Registered | RegisterContextProfileChange::Unchanged => {
                Ok(())
            }
            RegisterContextProfileChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "conflicting context profile registration".into(),
            }),
        }
    }

    fn apply_context_profile_assigned(
        &mut self,
        component_key: &str,
        profile_key: &str,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .assign_context_profile(component_key, profile_key);
        match result.change {
            AssignContextProfileChange::Assigned | AssignContextProfileChange::Unchanged => Ok(()),
            AssignContextProfileChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid component context profile assignment".into(),
            }),
        }
    }

    fn apply_component_tagged(
        &mut self,
        tag_key: &str,
        component_key: &str,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .assign_component_tag(tag_key, component_key);
        match result.change {
            AssignComponentTagChange::Assigned | AssignComponentTagChange::Unchanged => Ok(()),
            AssignComponentTagChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid component tag assignment".into(),
            }),
        }
    }

    fn apply_tag_context_profile_assigned(
        &mut self,
        tag_key: &str,
        profile_key: &str,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .assign_context_profile_for_tag(tag_key, profile_key);
        match result.change {
            AssignTagContextProfileChange::Assigned | AssignTagContextProfileChange::Unchanged => {
                Ok(())
            }
            AssignTagContextProfileChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid tag context profile assignment".into(),
            }),
        }
    }

    fn apply_collection_context_profile_assigned(
        &mut self,
        collection_key: &str,
        profile_key: &str,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .assign_context_profile_for_collection(collection_key, profile_key);
        match result.change {
            AssignCollectionContextProfileChange::Assigned
            | AssignCollectionContextProfileChange::Unchanged => Ok(()),
            AssignCollectionContextProfileChange::Rejected => {
                Err(DurableStateError::CorruptHistory {
                    line,
                    reason: "invalid collection context profile assignment".into(),
                })
            }
        }
    }

    fn apply_collection_component_added(
        &mut self,
        collection_key: &str,
        component_key: &str,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .add_component_to_collection(collection_key, component_key);
        match result.change {
            AddCollectionComponentChange::Added | AddCollectionComponentChange::Unchanged => Ok(()),
            AddCollectionComponentChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid collection membership add".into(),
            }),
        }
    }

    fn apply_collection_component_removed(
        &mut self,
        collection_key: &str,
        component_key: &str,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .remove_component_from_collection(collection_key, component_key);
        match result.change {
            RemoveCollectionComponentChange::Removed
            | RemoveCollectionComponentChange::Unchanged => Ok(()),
            RemoveCollectionComponentChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid collection membership removal".into(),
            }),
        }
    }

    fn apply_collection_source_configured(
        &mut self,
        collection_key: &str,
        source: CollectionSource,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .configure_collection_source(collection_key, source);
        match result.change {
            ConfigureCollectionSourceChange::Configured
            | ConfigureCollectionSourceChange::Unchanged => Ok(()),
            ConfigureCollectionSourceChange::Rejected => Err(DurableStateError::CorruptHistory {
                line,
                reason: "invalid collection source configuration".into(),
            }),
        }
    }

    fn apply_collection_source_materialized(
        &mut self,
        collection_key: &str,
        added_component_keys: Vec<Box<str>>,
        removed_component_keys: Vec<Box<str>>,
        line: usize,
    ) -> Result<(), DurableStateError> {
        for component_key in added_component_keys {
            self.apply_collection_component_added(collection_key, component_key.as_ref(), line)?;
        }
        for component_key in removed_component_keys {
            self.apply_collection_component_removed(collection_key, component_key.as_ref(), line)?;
        }
        Ok(())
    }

    fn apply_collection_scan_schedule_configured(
        &mut self,
        collection_key: &str,
        cadence_minutes: u32,
        freshness: EvidenceFreshness,
        next_due_at_unix_ms: u64,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .configure_collection_scan_schedule(
                collection_key,
                cadence_minutes,
                freshness,
                next_due_at_unix_ms,
            );
        match result.change {
            ConfigureCollectionScanScheduleChange::Configured
            | ConfigureCollectionScanScheduleChange::Unchanged => Ok(()),
            ConfigureCollectionScanScheduleChange::Rejected => {
                Err(DurableStateError::CorruptHistory {
                    line,
                    reason: "invalid collection scan schedule".into(),
                })
            }
        }
    }

    fn apply_collection_scan_schedule_materialized(
        &mut self,
        collection_key: &str,
        next_due_at_unix_ms: u64,
        materialized_at_unix_ms: u64,
        enqueued_commands: u32,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let result = self
            .ingestion
            .inventory_mut()
            .record_collection_scan_materialization(
                collection_key,
                next_due_at_unix_ms,
                materialized_at_unix_ms,
                enqueued_commands,
            );
        match result.change {
            ConfigureCollectionScanScheduleChange::Configured
            | ConfigureCollectionScanScheduleChange::Unchanged => Ok(()),
            ConfigureCollectionScanScheduleChange::Rejected => {
                Err(DurableStateError::CorruptHistory {
                    line,
                    reason: "invalid collection scan materialization".into(),
                })
            }
        }
    }

    fn apply_provider_scan_recorded(
        &mut self,
        command_id: Option<Box<str>>,
        report: StoredProviderScanReport,
        change_set: Option<FindingChangeSet>,
        pending_integration_event: Option<PendingIntegrationEvent>,
        line: usize,
    ) -> Result<(), DurableStateError> {
        let report = report.into_domain()?;
        let canonical_findings = canonicalize_reported_findings(&report.findings);
        self.ingestion
            .replay_canonical_scan_report(&report, &canonical_findings)
            .map_err(|error| match error {
                FindingIngestionError::UnmanagedComponent
                | FindingIngestionError::UnmanagedArtifact => DurableStateError::CorruptHistory {
                    line,
                    reason: format!("provider report cannot be replayed: {}", error.as_str())
                        .into_boxed_str(),
                },
            })?;
        self.read_model.replay_canonical_scan_report(
            report.component_key.clone(),
            report.artifact.clone(),
            &canonical_findings,
        );
        if let Some(pending_integration_event) = pending_integration_event {
            self.pending_integration_events
                .push_back(pending_integration_event);
        }
        if let (Some(command_id), Some(change_set)) = (command_id, change_set) {
            self.applied_scan_commands.insert(command_id, change_set);
        }
        Ok(())
    }

    fn apply_finding_risk_accepted(
        &mut self,
        finding: StoredFindingRef,
        acceptance: RiskAcceptance,
    ) {
        let finding = finding.into_domain();
        self.governance
            .replay_risk_acceptance(finding.clone(), acceptance.clone());
        self.read_model.replay_risk_acceptance(finding, acceptance);
    }

    fn apply_finding_suppressed(&mut self, finding: StoredFindingRef, suppression: Suppression) {
        let finding = finding.into_domain();
        self.governance
            .replay_suppression(finding.clone(), suppression.clone());
        self.read_model.replay_suppression(finding, suppression);
    }

    fn apply_finding_reopened(&mut self, finding: &StoredFindingRef) {
        let finding = finding.clone().into_domain();
        self.governance.replay_reopen(&finding);
        self.read_model.replay_reopen(&finding);
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

    fn append_event(&self, event: &DurableEvent) -> Result<(), DurableStateError> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.history_path)
            .map_err(DurableStateError::Io)?;
        serde_json::to_writer(&mut file, event).map_err(DurableStateError::Serialize)?;
        file.write_all(b"\n").map_err(DurableStateError::Io)?;
        file.flush().map_err(DurableStateError::Io)?;
        file.sync_all().map_err(DurableStateError::Io)?;
        Ok(())
    }

    fn push_system_event(&mut self, event: SystemEvent) {
        self.system_events.push_front(event);
        while self.system_events.len() > SYSTEM_EVENT_LOG_CAPACITY {
            self.system_events.pop_back();
        }
    }
}

fn current_unix_millis() -> Result<u64, DurableStateError> {
    let duration = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| DurableStateError::Time(error.to_string()))?;
    u64::try_from(duration.as_millis())
        .map_err(|_| DurableStateError::Time("timestamp out of range".into()))
}

/// Canonical failure returned by the local durable state boundary.
#[derive(Debug)]
pub enum DurableStateError {
    Io(io::Error),
    Serialize(serde_json::Error),
    CorruptHistory { line: usize, reason: Box<str> },
    Ingestion(FindingIngestionError),
    MissingCollection(Box<str>),
    MissingTag(Box<str>),
    MissingFinding(Box<str>),
    Time(String),
}

impl DurableStateError {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "io-error",
            Self::Serialize(_) => "serialization-error",
            Self::CorruptHistory { .. } => "corrupt-history",
            Self::Ingestion(FindingIngestionError::UnmanagedComponent) => "unmanaged-component",
            Self::Ingestion(FindingIngestionError::UnmanagedArtifact) => "unmanaged-artifact",
            Self::MissingCollection(_) => "missing-collection",
            Self::MissingTag(_) => "missing-tag",
            Self::MissingFinding(_) => "missing-finding",
            Self::Time(_) => "invalid-time",
        }
    }
}

impl core::fmt::Display for DurableStateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Serialize(error) => write!(f, "serialization error: {error}"),
            Self::CorruptHistory { line, reason } => {
                write!(f, "corrupt history at line {line}: {reason}")
            }
            Self::Ingestion(error) => write!(f, "ingestion error: {}", error.as_str()),
            Self::MissingCollection(error) => write!(f, "missing collection: {error}"),
            Self::MissingTag(error) => write!(f, "missing tag: {error}"),
            Self::MissingFinding(error) => write!(f, "missing finding: {error}"),
            Self::Time(error) => write!(f, "time error: {error}"),
        }
    }
}

impl std::error::Error for DurableStateError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum DurableEvent {
    ComponentRegistered {
        registration: StoredComponentRegistration,
    },
    ComponentTagRegistered {
        registration: StoredComponentTagRegistration,
    },
    ContextProfileRegistered {
        registration: StoredContextProfileRegistration,
    },
    CollectionRegistered {
        registration: StoredCollectionRegistration,
    },
    ArtifactBound {
        component_key: Box<str>,
        artifact: ArtifactRef,
    },
    ComponentProviderConfigured {
        component_key: Box<str>,
        provider_key: Box<str>,
    },
    ComponentContextProfileAssigned {
        component_key: Box<str>,
        profile_key: Box<str>,
    },
    ComponentTagged {
        tag_key: Box<str>,
        component_key: Box<str>,
    },
    TagContextProfileAssigned {
        tag_key: Box<str>,
        profile_key: Box<str>,
    },
    CollectionContextProfileAssigned {
        collection_key: Box<str>,
        profile_key: Box<str>,
    },
    CollectionComponentAdded {
        collection_key: Box<str>,
        component_key: Box<str>,
    },
    CollectionComponentRemoved {
        collection_key: Box<str>,
        component_key: Box<str>,
    },
    CollectionSourceConfigured {
        collection_key: Box<str>,
        source: StoredCollectionSource,
    },
    CollectionSourceMaterialized {
        collection_key: Box<str>,
        added_component_keys: Vec<Box<str>>,
        removed_component_keys: Vec<Box<str>>,
    },
    CollectionScanScheduleConfigured {
        collection_key: Box<str>,
        cadence_minutes: u32,
        freshness: EvidenceFreshness,
        next_due_at_unix_ms: u64,
    },
    CollectionScanScheduleMaterialized {
        collection_key: Box<str>,
        next_due_at_unix_ms: u64,
        materialized_at_unix_ms: u64,
        enqueued_commands: u32,
    },
    IntegrationRuntimeConfigured {
        config: IntegrationRuntimeConfig,
    },
    ProviderScanRecorded {
        #[serde(default)]
        command_id: Option<Box<str>>,
        report: StoredProviderScanReport,
        #[serde(default)]
        change_set: Option<FindingChangeSet>,
        #[serde(default)]
        pending_integration_event: Box<Option<PendingIntegrationEvent>>,
    },
    FindingRiskAccepted {
        finding: StoredFindingRef,
        acceptance: RiskAcceptance,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    FindingsRiskAccepted {
        collection_key: Box<str>,
        findings: Vec<StoredFindingRef>,
        acceptance: RiskAcceptance,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    TagFindingsRiskAccepted {
        tag_key: Box<str>,
        findings: Vec<StoredFindingRef>,
        acceptance: RiskAcceptance,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    FindingSuppressed {
        finding: StoredFindingRef,
        suppression: Suppression,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    FindingReopened {
        finding: StoredFindingRef,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    FindingsSuppressed {
        collection_key: Box<str>,
        findings: Vec<StoredFindingRef>,
        suppression: Suppression,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    TagFindingsSuppressed {
        tag_key: Box<str>,
        findings: Vec<StoredFindingRef>,
        suppression: Suppression,
        #[serde(default)]
        occurred_at_unix_ms: u64,
    },
    FindingsReopened {
        collection_key: Box<str>,
        findings: Vec<StoredFindingRef>,
        #[serde(default)]
        occurred_at_unix_ms: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredComponentRegistration {
    component_key: Box<str>,
    name: Box<str>,
}

impl From<ComponentRegistration> for StoredComponentRegistration {
    fn from(value: ComponentRegistration) -> Self {
        Self {
            component_key: value.component_key,
            name: value.name,
        }
    }
}

impl StoredComponentRegistration {
    fn into_domain(self) -> ComponentRegistration {
        ComponentRegistration::new(self.component_key, self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredComponentTagRegistration {
    tag_key: Box<str>,
    name: Box<str>,
}

impl From<ComponentTagRegistration> for StoredComponentTagRegistration {
    fn from(value: ComponentTagRegistration) -> Self {
        Self {
            tag_key: value.tag_key,
            name: value.name,
        }
    }
}

impl StoredComponentTagRegistration {
    fn into_domain(self) -> ComponentTagRegistration {
        ComponentTagRegistration::new(self.tag_key, self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCollectionRegistration {
    collection_key: Box<str>,
    name: Box<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredContextProfileRegistration {
    profile_key: Box<str>,
    name: Box<str>,
    internet_exposed: Option<bool>,
    production: Option<bool>,
    mission_critical: Option<bool>,
    vpn_restricted: Option<bool>,
    non_privileged_user: Option<bool>,
}

impl From<ContextProfileRegistration> for StoredContextProfileRegistration {
    fn from(value: ContextProfileRegistration) -> Self {
        Self {
            profile_key: value.profile_key,
            name: value.name,
            internet_exposed: value.internet_exposed,
            production: value.production,
            mission_critical: value.mission_critical,
            vpn_restricted: value.vpn_restricted,
            non_privileged_user: value.non_privileged_user,
        }
    }
}

impl StoredContextProfileRegistration {
    fn into_domain(self) -> ContextProfileRegistration {
        let mut registration = ContextProfileRegistration::overlay(self.profile_key, self.name);
        if let Some(value) = self.internet_exposed {
            registration = registration.with_internet_exposed(value);
        }
        if let Some(value) = self.production {
            registration = registration.with_production(value);
        }
        if let Some(value) = self.mission_critical {
            registration = registration.with_mission_critical(value);
        }
        if let Some(value) = self.vpn_restricted {
            registration = registration.with_vpn_restricted(value);
        }
        if let Some(value) = self.non_privileged_user {
            registration = registration.with_non_privileged_user(value);
        }
        registration
    }
}

impl From<CollectionRegistration> for StoredCollectionRegistration {
    fn from(value: CollectionRegistration) -> Self {
        Self {
            collection_key: value.collection_key,
            name: value.name,
        }
    }
}

impl StoredCollectionRegistration {
    fn into_domain(self) -> CollectionRegistration {
        CollectionRegistration::new(self.collection_key, self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum StoredCollectionSource {
    ComponentList {
        mode: StoredCollectionSourceMode,
        component_keys: Vec<Box<str>>,
    },
}

impl From<CollectionSource> for StoredCollectionSource {
    fn from(value: CollectionSource) -> Self {
        match value {
            CollectionSource::ComponentList(source) => Self::ComponentList {
                mode: StoredCollectionSourceMode::from(source.mode),
                component_keys: source.component_keys,
            },
        }
    }
}

impl StoredCollectionSource {
    fn into_domain(self) -> CollectionSource {
        match self {
            Self::ComponentList {
                mode,
                component_keys,
            } => CollectionSource::ComponentList(crate::ComponentListCollectionSource::new(
                mode.into_domain(),
                component_keys,
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredCollectionSourceMode {
    Replace,
    Reconcile,
}

impl From<CollectionSourceMode> for StoredCollectionSourceMode {
    fn from(value: CollectionSourceMode) -> Self {
        match value {
            CollectionSourceMode::Replace => Self::Replace,
            CollectionSourceMode::Reconcile => Self::Reconcile,
        }
    }
}

impl StoredCollectionSourceMode {
    const fn into_domain(self) -> CollectionSourceMode {
        match self {
            Self::Replace => CollectionSourceMode::Replace,
            Self::Reconcile => CollectionSourceMode::Reconcile,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredProviderScanReport {
    provider_key: Box<str>,
    component_key: Box<str>,
    artifact: ArtifactRef,
    observed_at: StoredObservedAt,
    freshness: EvidenceFreshness,
    knowledge_revision: Option<Box<str>>,
    findings: Vec<StoredReportedFinding>,
}

impl StoredProviderScanReport {
    pub(crate) fn from_report(report: &ProviderScanReport) -> Result<Self, DurableStateError> {
        Ok(Self {
            provider_key: report.provider_key.clone(),
            component_key: report.component_key.clone(),
            artifact: report.artifact.clone(),
            observed_at: StoredObservedAt::from_system_time(report.observed_at)?,
            freshness: report.freshness,
            knowledge_revision: report.knowledge_revision.clone(),
            findings: report
                .findings
                .iter()
                .cloned()
                .map(StoredReportedFinding::from)
                .collect(),
        })
    }

    pub(crate) fn into_domain(self) -> Result<ProviderScanReport, DurableStateError> {
        let observed_at = self.observed_at.into_system_time()?;
        let mut report = ProviderScanReport::new(
            self.provider_key,
            self.component_key,
            self.artifact,
            observed_at,
            self.freshness,
            self.findings
                .into_iter()
                .map(StoredReportedFinding::into_domain)
                .collect(),
        );
        report.knowledge_revision = self.knowledge_revision;
        Ok(report)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum StoredObservedAt {
    UnixMillis(i64),
    LegacyRfc3339(Box<str>),
}

impl StoredObservedAt {
    fn from_system_time(value: std::time::SystemTime) -> Result<Self, DurableStateError> {
        let duration = value
            .duration_since(UNIX_EPOCH)
            .map_err(|error| DurableStateError::Time(error.to_string()))?;
        let millis = i64::try_from(duration.as_millis())
            .map_err(|_| DurableStateError::Time("timestamp out of range".into()))?;
        Ok(Self::UnixMillis(millis))
    }

    fn into_system_time(self) -> Result<std::time::SystemTime, DurableStateError> {
        match self {
            Self::UnixMillis(millis) => {
                let millis = u64::try_from(millis).map_err(|_| {
                    DurableStateError::Time("negative timestamp not supported".into())
                })?;
                Ok(UNIX_EPOCH + Duration::from_millis(millis))
            }
            Self::LegacyRfc3339(value) => OffsetDateTime::parse(&value, &Rfc3339)
                .map_err(|error| DurableStateError::Time(error.to_string()))
                .map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredReportedFinding {
    vulnerability_id: Box<str>,
    provider_finding_key: Option<Box<str>>,
    package: StoredPackageCoordinate,
    fix_version: Option<Box<str>>,
    severity: Severity,
    aliases: Vec<Box<str>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredFindingRef {
    component_key: Box<str>,
    artifact: ArtifactRef,
    vulnerability_id: Box<str>,
    package: StoredPackageCoordinate,
}

impl From<FindingRef> for StoredFindingRef {
    fn from(value: FindingRef) -> Self {
        Self {
            component_key: value.component_key,
            artifact: value.artifact,
            vulnerability_id: value.vulnerability_id,
            package: StoredPackageCoordinate::from(value.package),
        }
    }
}

impl StoredFindingRef {
    fn into_domain(self) -> FindingRef {
        FindingRef::new(
            self.component_key,
            self.artifact,
            self.vulnerability_id,
            self.package.into_domain(),
        )
    }
}

impl From<ReportedFinding> for StoredReportedFinding {
    fn from(value: ReportedFinding) -> Self {
        Self {
            vulnerability_id: value.vulnerability_id,
            provider_finding_key: value.provider_finding_key,
            package: StoredPackageCoordinate::from(value.package),
            fix_version: value.fix_version,
            severity: value.severity,
            aliases: value.aliases,
        }
    }
}

impl StoredReportedFinding {
    fn into_domain(self) -> ReportedFinding {
        ReportedFinding {
            vulnerability_id: self.vulnerability_id,
            provider_finding_key: self.provider_finding_key,
            package: self.package.into_domain(),
            fix_version: self.fix_version,
            severity: self.severity,
            aliases: self.aliases,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPackageCoordinate {
    name: Box<str>,
    version: Box<str>,
    purl: Option<Box<str>>,
}

impl From<PackageCoordinate> for StoredPackageCoordinate {
    fn from(value: PackageCoordinate) -> Self {
        Self {
            name: value.name,
            version: value.version,
            purl: value.purl,
        }
    }
}

impl StoredPackageCoordinate {
    fn into_domain(self) -> PackageCoordinate {
        PackageCoordinate {
            name: self.name,
            version: self.version,
            purl: self.purl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DurableState;
    use crate::{
        ArtifactKind, ArtifactRef, BulkGovernanceQuery, CollectionRegistration,
        ComponentRegistration, ConfigureProviderChange, EvidenceFreshness, FindingGovernanceState,
        IntegrationEvent, IntegrationEventPublishError, IntegrationEventPublisher,
        IntegrationRuntimeConfig, PackageCoordinate, PendingIntegrationEvent, ProviderScanReport,
        ReportedFinding, RiskAcceptance,
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

    fn report(findings: Vec<ReportedFinding>) -> ProviderScanReport {
        ProviderScanReport::new(
            "fixture-provider",
            "component:payments-api",
            artifact(),
            SystemTime::UNIX_EPOCH,
            EvidenceFreshness::Deterministic,
            findings,
        )
        .with_knowledge_revision("fixture-db:2026-05-14")
    }

    #[test]
    fn replay_rebuilds_inventory_and_active_findings() {
        let path = temp_path("durable-state-replay");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("scan report should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert!(
            rebuilt
                .ingestion()
                .inventory()
                .is_managed("component:payments-api")
        );
        assert!(
            rebuilt
                .ingestion()
                .inventory()
                .component_owns_artifact("component:payments-api", &artifact())
        );
        assert_eq!(
            rebuilt
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            1
        );
    }

    #[test]
    fn replay_rebuilds_release_collections() {
        let path = temp_path("durable-state-collections");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .expect("collection should persist");
        let _ = state
            .add_component_to_collection("release:2026.05", "component:payments-api")
            .expect("collection membership should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert!(
            rebuilt
                .ingestion()
                .inventory()
                .is_collection_managed("release:2026.05")
        );
        assert_eq!(
            rebuilt
                .ingestion()
                .inventory()
                .collection_members("release:2026.05"),
            Some(vec![Box::<str>::from("component:payments-api")])
        );
    }

    #[test]
    fn replay_keeps_collection_scan_schedules() {
        let path = temp_path("durable-state-collection-schedules");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .expect("collection should persist");
        let _ = state
            .configure_collection_scan_schedule(
                "release:2026.05",
                60,
                EvidenceFreshness::Deterministic,
                1_000,
            )
            .expect("schedule should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert_eq!(
            rebuilt
                .ingestion()
                .inventory()
                .collection_scan_schedule("release:2026.05"),
            Some(crate::CollectionScanSchedule {
                cadence_minutes: 60,
                freshness: EvidenceFreshness::Deterministic,
                next_due_at_unix_ms: 1_000,
                last_materialized_at_unix_ms: None,
                last_enqueued_commands: None,
            })
        );
    }

    #[test]
    fn replay_keeps_collection_schedule_materialization_metadata() {
        let path = temp_path("durable-state-collection-schedule-runs");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .expect("collection should persist");
        let _ = state
            .add_component_to_collection("release:2026.05", "component:payments-api")
            .expect("collection membership should persist");
        let _ = state
            .configure_collection_scan_schedule(
                "release:2026.05",
                60,
                EvidenceFreshness::Deterministic,
                1_000,
            )
            .expect("schedule should persist");
        let _ = state
            .record_collection_scan_materialization("release:2026.05", 3_601_500, 1_500, 1)
            .expect("materialization should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert_eq!(
            rebuilt
                .ingestion()
                .inventory()
                .collection_scan_schedule("release:2026.05"),
            Some(crate::CollectionScanSchedule {
                cadence_minutes: 60,
                freshness: EvidenceFreshness::Deterministic,
                next_due_at_unix_ms: 3_601_500,
                last_materialized_at_unix_ms: Some(1_500),
                last_enqueued_commands: Some(1),
            })
        );
    }

    #[test]
    fn new_history_stores_observed_at_as_unix_millis() {
        let path = temp_path("durable-state-unix-millis");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("scan report should persist");

        let history = fs::read_to_string(&path).expect("history should be readable");

        assert!(history.contains("\"observed_at\":0"));
    }

    #[test]
    fn replay_keeps_legacy_rfc3339_history_compatible() {
        let path = temp_path("durable-state-legacy-rfc3339");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("scan report should persist");

        let history = fs::read_to_string(&path).expect("history should be readable");
        let legacy_history = history.replace(
            "\"observed_at\":0",
            "\"observed_at\":\"1970-01-01T00:00:00Z\"",
        );
        fs::write(&path, legacy_history).expect("legacy history should be writable");

        let rebuilt = DurableState::open(&path).expect("legacy durable state should replay");

        assert_eq!(
            rebuilt
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            1
        );
    }

    #[test]
    fn replay_keeps_withdrawn_findings_inactive() {
        let path = temp_path("durable-state-withdrawal");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("first report should persist");
        let _ = state
            .record_scan_report(&report(Vec::new()))
            .expect("withdrawal snapshot should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert_eq!(
            rebuilt
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            0
        );
    }

    #[test]
    fn replay_keeps_provider_runtime_configuration() {
        let path = temp_path("durable-state-provider");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");

        let result = state
            .configure_provider("component:payments-api", "fixture-provider")
            .expect("provider config should persist");

        assert_eq!(result.change, ConfigureProviderChange::Configured);

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert_eq!(
            rebuilt
                .ingestion()
                .inventory()
                .configured_provider("component:payments-api"),
            Some("fixture-provider")
        );
    }

    #[test]
    fn replay_keeps_integration_runtime_configuration() {
        let path = temp_path("durable-state-integration-runtime");
        let mut state = DurableState::open(&path).expect("durable state should open");

        let result = state
            .configure_integration_runtime(IntegrationRuntimeConfig::Http {
                endpoint_url: "http://127.0.0.1:38080/publish".into(),
                timeout_ms: 3_000,
            })
            .expect("integration runtime config should persist");

        assert_eq!(result.change.as_str(), "configured");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");

        assert_eq!(
            rebuilt.integration_runtime_config(),
            Some(&IntegrationRuntimeConfig::Http {
                endpoint_url: "http://127.0.0.1:38080/publish".into(),
                timeout_ms: 3_000,
            })
        );
    }

    #[test]
    fn replay_keeps_pending_integration_events_for_provider_reports() {
        let path = temp_path("durable-state-outbox");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("scan report should persist");

        let rebuilt = DurableState::open(&path).expect("durable state should replay");
        assert_eq!(rebuilt.pending_integration_events().len(), 1);
        assert!(matches!(
            rebuilt.pending_integration_events()[0].event,
            IntegrationEvent::FindingChangesObserved { .. }
        ));
    }

    #[test]
    fn command_scoped_scan_report_recording_is_idempotent() {
        let path = temp_path("durable-state-command-idempotence");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");

        let first = state
            .record_scan_report_for_command(
                "scan-command-1",
                &report(vec![ReportedFinding::new(
                    "CVE-2026-0001",
                    PackageCoordinate::new("openssl", "3.0.0"),
                )]),
            )
            .expect("first scan report should persist");
        let second = state
            .record_scan_report_for_command(
                "scan-command-1",
                &report(vec![ReportedFinding::new(
                    "CVE-2026-0001",
                    PackageCoordinate::new("openssl", "3.0.0"),
                )]),
            )
            .expect("second scan report should reuse the durable change set");

        assert_eq!(first, second);
        assert_eq!(
            state
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            1
        );

        let mut rebuilt = DurableState::open(&path).expect("durable state should replay");
        let replayed = rebuilt
            .record_scan_report_for_command(
                "scan-command-1",
                &report(vec![ReportedFinding::new(
                    "CVE-2026-0001",
                    PackageCoordinate::new("openssl", "3.0.0"),
                )]),
            )
            .expect("replayed state should keep command-scoped idempotence");

        assert_eq!(replayed, first);
        assert_eq!(
            rebuilt
                .read_model()
                .active_finding_count("component:payments-api", &artifact()),
            1
        );
    }

    #[test]
    fn bulk_collection_risk_acceptance_targets_the_full_matching_cohort() {
        let path = temp_path("durable-state-bulk-cohort");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .register_collection(CollectionRegistration::new(
                "release:2026.05",
                "May Release",
            ))
            .expect("collection should persist");
        let _ = state
            .add_component_to_collection("release:2026.05", "component:payments-api")
            .expect("collection membership should persist");
        let findings = (0..205)
            .map(|index| {
                ReportedFinding::new(
                    format!("CVE-2026-{index:04}"),
                    PackageCoordinate::new(format!("pkg-{index:04}"), "1.0.0"),
                )
                .with_severity(crate::Severity::High)
            })
            .collect::<Vec<_>>();
        let _ = state
            .record_scan_report(&report(findings))
            .expect("scan report should persist");

        let result = state
            .accept_risk_for_collection(
                "release:2026.05",
                &BulkGovernanceQuery::new(FindingGovernanceState::Open),
                RiskAcceptance::new("Accepted whole release"),
            )
            .expect("bulk risk acceptance should persist");

        assert_eq!(result.targeted, 205);
        assert_eq!(result.accepted, 205);
        assert_eq!(result.unchanged, 0);
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
    async fn successful_publication_removes_pending_integration_event() {
        let path = temp_path("durable-state-publish-success");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("scan report should persist");

        let result = state
            .publish_pending_integration_events(1, &SuccessPublisher)
            .await
            .expect("publication should persist");
        assert_eq!(result.published, 1);
        assert_eq!(state.pending_integration_events().len(), 0);

        let rebuilt = DurableState::open(&path).expect("durable state should replay");
        assert_eq!(rebuilt.pending_integration_events().len(), 0);
    }

    #[tokio::test]
    async fn failed_publication_keeps_pending_integration_event() {
        let path = temp_path("durable-state-publish-failure");
        let mut state = DurableState::open(&path).expect("durable state should open");
        let _ = state
            .register_component(ComponentRegistration::new(
                "component:payments-api",
                "Payments API",
            ))
            .expect("registration should persist");
        let _ = state
            .bind_artifact("component:payments-api", artifact())
            .expect("artifact binding should persist");
        let _ = state
            .record_scan_report(&report(vec![ReportedFinding::new(
                "CVE-2026-0001",
                PackageCoordinate::new("openssl", "3.0.0"),
            )]))
            .expect("scan report should persist");

        let result = state
            .publish_pending_integration_events(1, &FailingPublisher)
            .await
            .expect("failed publication outcome should persist");
        assert_eq!(result.published, 0);
        assert_eq!(state.pending_integration_events().len(), 1);

        let rebuilt = DurableState::open(&path).expect("durable state should replay");
        assert_eq!(rebuilt.pending_integration_events().len(), 1);
    }
}
