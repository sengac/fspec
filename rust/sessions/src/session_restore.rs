use crate::replay_content::{Replay, RestoreError};
use crate::SessionManager;
use codelet_core::persistence::load_session;
use codelet_rpc_types::{SessionId, StreamChunk, TokenRestoreState};
use rig::message::Message;
use serde_json::json;

fn persisted_suffix_overlap(
    installed: &[Message],
    incoming: &[Message],
    runtime_suffix: &[Message],
) -> Result<usize, RestoreError> {
    let appended = incoming.strip_prefix(installed).ok_or_else(|| {
        RestoreError::Conflict("cannot reconcile runtime suffix with rewritten replay".into())
    })?;
    let overlap = appended.len().min(runtime_suffix.len());
    if appended[..overlap] != runtime_suffix[..overlap] {
        return Err(RestoreError::Conflict(
            "runtime suffix disagrees with newly persisted history".into(),
        ));
    }
    Ok(overlap)
}

impl SessionManager {
    pub(crate) fn restore_messages(
        &self,
        id: &SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), RestoreError> {
        let uuid = uuid::Uuid::parse_str(&id.value)
            .map_err(|error| RestoreError::Store(error.to_string()))?;
        let session = self.get_session(&id.value).map_err(RestoreError::Store)?;
        let model_replay = Replay::parse(&envelopes)?;
        drop(model_replay.chunks);
        let manifest = load_session(uuid).map_err(RestoreError::Store)?;
        if model_replay.identity.as_ref().is_some_and(|identity| {
            identity
                != &json!({
                    "version":1,"sessionId":uuid,"revision":manifest.revision,
                })
        }) {
            return Err(RestoreError::Conflict(
                "snapshot is not the current committed session".into(),
            ));
        }
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut inner = session.inner.lock().await;
                if inner
                    .restored_replay
                    .as_ref()
                    .is_some_and(|current| current.revision > manifest.revision)
                {
                    return Err(RestoreError::Conflict("installed revision is newer".into()));
                }
                let prefix = inner
                    .restored_replay
                    .as_ref()
                    .map_or(inner.messages.len(), |current| current.runtime_prefix_len);
                let replaced_len = inner
                    .restored_replay
                    .as_ref()
                    .map_or(0, |current| current.replay_len);
                let end = prefix
                    .checked_add(replaced_len)
                    .filter(|end| *end <= inner.messages.len())
                    .ok_or_else(|| {
                        RestoreError::Conflict("installed replay range no longer exists".into())
                    })?;
                inner.block_compaction_inference("restoring authoritative session replay".into());
                let end = if end < inner.messages.len()
                    && inner
                        .restored_replay
                        .as_ref()
                        .is_some_and(|current| current.revision < manifest.revision)
                {
                    end + persisted_suffix_overlap(
                        &inner.messages[prefix..end],
                        &model_replay.messages,
                        &inner.messages[end..],
                    )?
                } else {
                    end
                };
                inner
                    .preflight_compaction_restore(&manifest, &model_replay.envelopes)
                    .map_err(|error| RestoreError::Store(error.to_string()))?;
                inner
                    .restore_compaction_boundary(&manifest)
                    .map_err(|error| RestoreError::Store(error.to_string()))?;
                inner
                    .bind_provider_persistence(uuid)
                    .map_err(|error| RestoreError::Store(error.to_string()))?;
                if manifest.compaction_commit.is_none() {
                    inner
                        .restore_provider_state(&model_replay.envelopes)
                        .map_err(|error| RestoreError::Store(error.to_string()))?;
                }
                let replay_len = model_replay.messages.len();
                inner.messages.splice(prefix..end, model_replay.messages);
                inner.restored_replay = Some(codelet_cli::session::replay::ReplayInstallation {
                    revision: manifest.revision,
                    runtime_prefix_len: prefix,
                    replay_len,
                });
                Ok::<(), RestoreError>(())
            })
        })?;
        session
            .restore_archival_transcript()
            .map_err(|error| RestoreError::Store(error.to_string()))?;
        Ok(())
    }

    pub(crate) fn restore_tokens(
        &self,
        id: &SessionId,
        state: TokenRestoreState,
    ) -> Result<(), RestoreError> {
        let uuid = uuid::Uuid::parse_str(&id.value)
            .map_err(|error| RestoreError::Store(error.to_string()))?;
        let session = self.get_session(&id.value).map_err(RestoreError::Store)?;
        let manifest = load_session(uuid).map_err(RestoreError::Store)?;
        let saved = &manifest.token_usage;
        let supplied = [
            state.current_context,
            state.cumulative_billed_output,
            state.cache_read,
            state.cache_creation,
            state.cumulative_billed_input,
            state.cumulative_billed_output_second,
        ];
        let expected = [
            saved.current_context_tokens,
            saved.cumulative_billed_output,
            saved.cache_read_tokens,
            saved.cache_creation_tokens,
            saved.cumulative_billed_input,
            saved.cumulative_billed_output,
        ];
        for (supplied, expected) in supplied.into_iter().zip(expected) {
            if u64::try_from(supplied).ok() != Some(expected) || u32::try_from(expected).is_err() {
                return Err(RestoreError::Conflict(
                    "saved token counters disagree with committed manifest".into(),
                ));
            }
        }
        let reasoning = u32::try_from(saved.reasoning_tokens)
            .map_err(|_| RestoreError::Conflict("reasoning counter is not representable".into()))?;
        let tokens = crate::committed_publication::saved_tokens(&manifest)
            .map_err(|error| RestoreError::Conflict(error.to_string()))?;
        let committed_snapshot =
            crate::committed_publication::restored_snapshot(&session, &manifest, &tokens)
                .map_err(|error| RestoreError::Conflict(error.to_string()))?;
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut inner = session.inner.lock().await;
                if inner
                    .restored_replay
                    .as_ref()
                    .is_some_and(|current| current.revision != manifest.revision)
                {
                    return Err(RestoreError::Conflict(
                        "token and model history revisions differ".into(),
                    ));
                }
                let tracker = &mut inner.token_tracker;
                tracker.input_tokens = saved.current_context_tokens;
                tracker.output_tokens = saved.cumulative_billed_output;
                tracker.cumulative_billed_input = saved.cumulative_billed_input;
                tracker.cumulative_billed_output = saved.cumulative_billed_output;
                tracker.cache_read_input_tokens = Some(saved.cache_read_tokens);
                tracker.cache_creation_input_tokens = Some(saved.cache_creation_tokens);
                tracker.reasoning_tokens = saved.reasoning_tokens;
                Ok::<(), RestoreError>(())
            })
        })?;
        session.update_tokens(tokens.input_tokens, tokens.output_tokens);
        session.update_reasoning_tokens(reasoning);
        session.handle_output(StreamChunk::TokenUpdate { tokens });
        if let Some(snapshot) = committed_snapshot {
            session.handle_output(StreamChunk::CommittedCompaction { snapshot });
        }
        Ok(())
    }
}
