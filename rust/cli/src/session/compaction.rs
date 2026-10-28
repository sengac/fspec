use super::preservation::retained_reminders;
use super::Session;
use codelet_core::compaction::validate_dag_candidate;
use codelet_core::persistence::compaction_boundary::*;
use codelet_core::persistence::compaction_commit::{
    commit_compaction_admitted, reconcile_compaction, CompactionAdmission, CompactionCandidate,
    CompactionCommitError,
};
use codelet_core::persistence::{
    envelopes_for_manifest, load_session, CompactionState, SessionManifest,
};
use codelet_providers::request_admission::RequestAdmissionError;
use codelet_providers::selected_request::PreparedSelectedRequest;
use rig::message::Message;
use uuid::Uuid;

impl Session {
    pub fn capture_durable_compaction(
        &self,
        id: Uuid,
    ) -> Result<CompactionSource, CompactionCommitError> {
        let manifest = load_session(id).map_err(CompactionCommitError::Rejected)?;
        let model = self
            .current_model_id()
            .ok_or_else(|| CompactionCommitError::Rejected("missing selected model".into()))?;
        capture_compaction_source(&manifest, &model, "full-current-dag")
    }

    pub fn commit_durable_compaction(
        &mut self,
        source: CompactionSource,
        operation_id: Uuid,
        content: &str,
        mut prepare: impl FnMut(
            &[Message],
            u64,
        ) -> Result<PreparedSelectedRequest, RequestAdmissionError>,
        publish: impl FnOnce(&CommittedGeneration) -> Result<(), String>,
    ) -> Result<SessionManifest, CompactionCommitError> {
        self.block_compaction_inference("durable application is required".into());
        let manifest = load_session(source.session_id).map_err(CompactionCommitError::Rejected)?;
        let model = self
            .current_model_id()
            .ok_or_else(|| CompactionCommitError::Rejected("missing selected model".into()))?;
        if !self.matches_persisted_selection(&source.provider, &source.model) {
            return Err(CompactionCommitError::Rejected(
                "runtime source identity changed".into(),
            ));
        }
        let context = super::preservation::validation_context(self, &manifest, &source)
            .map_err(CompactionCommitError::Rejected)?;
        let dag = validate_dag_candidate(content, &context)
            .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
        let generation = source
            .generation
            .checked_add(1)
            .ok_or_else(|| CompactionCommitError::Rejected("generation exhausted".into()))?;
        let provider_session = self
            .provider_session
            .for_selected_provider(self.current_provider_name());
        let metadata = provider_session
            .prepare_compaction_generation(
                generation,
                &model,
                &envelopes_for_manifest(&manifest).map_err(CompactionCommitError::Rejected)?,
            )
            .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
        let candidate = CompactionCandidate {
            operation_id,
            checkpoint: CompactionCheckpoint {
                provider: source.provider.clone(),
                model: model.clone(),
                generation,
                payload: serde_json::to_value(&metadata.metadata)
                    .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?,
            },
            source,
            dag,
            input_tokens: 0,
            output_reservation: 0,
            token_provenance: CompactionTokenProvenance::Estimated,
        };
        let mut installation = None;
        let committed = commit_compaction_admitted(candidate, &model, |current, candidate| {
            self.validate_current_compaction_history(current, &candidate.source)?;
            let context =
                super::preservation::validation_context(self, current, &candidate.source)?;
            let checked = validate_dag_candidate(&candidate.dag.content, &context)
                .map_err(|error| error.to_string())?;
            if checked != candidate.dag {
                return Err("authoritative preservation obligations changed".into());
            }
            let mut projected = current.clone();
            projected.compaction = Some(CompactionState {
                summary: candidate.dag.content.clone(),
                compacted_before_index: candidate.source.prefix.len(),
                compacted_at: chrono::Utc::now(),
            });
            projected.compaction_commit = None;
            let reminders = retained_reminders(self, &candidate.source)?;
            let messages = replay_messages(&projected, &reminders)?;
            let prepared = prepare(&messages, metadata.generation)
                .map_err(|error| format!("selected request admission: {error}"))?;
            if prepared.generation() != metadata.generation {
                return Err("admission certificate has wrong generation".into());
            }
            let authoritative = provider_session
                .prepare_compaction_generation(
                    generation,
                    &model,
                    &envelopes_for_manifest(current)?,
                )
                .map_err(|error| error.to_string())?;
            if authoritative != metadata {
                return Err("provider checkpoint changed during admission".into());
            }
            let admitted = CompactionAdmission {
                input_tokens: prepared.input_tokens(),
                output_reservation: prepared.output_reservation(),
                provenance: CompactionTokenProvenance::Estimated,
            };
            installation = Some(messages);
            Ok(admitted)
        })
        .inspect_err(|error| {
            if let Some(identity) = error.promoted_identity() {
                self.pending_compaction_commit = Some(identity.clone());
            }
        })?;
        let boundary = committed
            .compaction_commit
            .as_ref()
            .ok_or_else(|| CompactionCommitError::Rejected("missing committed boundary".into()))?;
        self.pending_compaction_commit = Some(boundary.identity.clone());
        let failed = |reason| CompactionCommitError::Publication {
            identity: boundary.identity.clone(),
            reason,
        };
        provider_session
            .install_committed_generation(generation, &metadata.metadata)
            .map_err(|error| failed(error.to_string()))?;
        self.messages =
            installation.ok_or_else(|| failed("missing admitted installation".into()))?;
        self.compaction_working_messages = None;
        self.turns.clear();
        self.annotations.clear();
        publish(boundary).map_err(failed)?;
        self.reconcile_durable_compaction(&boundary.identity)
    }

    pub fn reconcile_durable_compaction(
        &mut self,
        identity: &CompactionIdentity,
    ) -> Result<SessionManifest, CompactionCommitError> {
        if self.pending_compaction_commit.as_ref() != Some(identity) {
            return Err(CompactionCommitError::Rejected(
                "runtime does not own this promoted identity".into(),
            ));
        }
        let manifest =
            load_session(identity.session_id).map_err(CompactionCommitError::Rejected)?;
        validate_committed_boundary(&manifest).map_err(CompactionCommitError::Rejected)?;
        let boundary = manifest
            .compaction_commit
            .as_ref()
            .filter(|boundary| &boundary.identity == identity)
            .ok_or_else(|| {
                CompactionCommitError::Rejected("identity is no longer authoritative".into())
            })?;
        let metadata = serde_json::from_value(boundary.checkpoint.payload.clone())
            .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
        let reminders =
            retained_reminders(self, &boundary.source).map_err(CompactionCommitError::Rejected)?;
        let messages =
            replay_messages(&manifest, &reminders).map_err(CompactionCommitError::Rejected)?;
        self.provider_session
            .for_compaction_checkpoint(self.current_provider_name(), &metadata)
            .install_committed_generation(identity.generation, &metadata)
            .map_err(|error| CompactionCommitError::Publication {
                identity: identity.clone(),
                reason: error.to_string(),
            })?;
        self.messages = messages;
        self.compaction_working_messages = None;
        self.token_tracker.input_tokens = boundary.input_tokens;
        let reconciled = reconcile_compaction(identity)?;
        self.restore_durable_tokens(&reconciled);
        self.pending_compaction_commit = None;
        self.compaction_inference_block = None;
        self.install_durable_summary(&reconciled);
        Ok(reconciled)
    }
}

pub(super) fn replay_messages(
    manifest: &SessionManifest,
    reminders: &[Message],
) -> Result<Vec<Message>, String> {
    let mut messages = reminders.to_vec();
    for encoded in envelopes_for_manifest(manifest)? {
        let envelope: serde_json::Value =
            serde_json::from_str(&encoded).map_err(|error| error.to_string())?;
        if let Some(message) = super::provider::conversational_message(&envelope)? {
            messages.push(message);
        }
    }
    Ok(messages)
}
