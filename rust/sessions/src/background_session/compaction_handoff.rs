use super::{BackgroundSession, PromptInput};
use codelet_rpc_types::{CompactionResult, SessionStatus};

/// BUG-516: the text of the internal manual-compaction request. It is never
/// a conversation message; the agent loop runs the compactor round for it.
pub const MANUAL_COMPACTION_REQUEST: &str = "/compact";
use std::sync::atomic::Ordering;

impl BackgroundSession {
    pub fn compaction_epoch(&self) -> u64 {
        self.interrupt_epoch.load(Ordering::Acquire)
    }

    pub fn compaction_cancelled(&self, epoch: u64) -> bool {
        self.is_interrupted.load(Ordering::Acquire) || self.compaction_epoch() != epoch
    }

    /// RPC-418 / BUG-516: manual `/compact`. Validates the session, enters
    /// Compacting and queues one internal manual-compaction request. The
    /// agent loop runs it as the same compactor sub-agent round automatic
    /// compaction uses: no working view, no 'Continue' message and no
    /// ordinary inference. Returns the RPC-421 acknowledgement; the final
    /// numbers ship on the CompactionComplete chunk.
    pub fn start_manual_compaction(&self) -> Result<CompactionResult, String> {
        self.ensure_restored()?;
        let compaction_epoch = self.compaction_epoch();
        let original_tokens = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut inner = self.inner.lock().await;
                if self.compaction_cancelled(compaction_epoch) {
                    return Err("manual compaction cancelled".to_string());
                }
                // Nothing to compact — leave the session untouched.
                if inner.messages.is_empty() {
                    return Err("Nothing to compact - no messages yet".to_string());
                }
                let original_tokens = inner.token_tracker.input_tokens;
                // CMPCT-088: checked before the session enters Compacting.
                let original_tokens_u32 =
                    codelet_common::narrow_count("original tokens", original_tokens)?;
                self.set_status(SessionStatus::Compacting);
                // CMPCT-041: the manual tracker-based pre-compaction snapshot.
                self.store_pre_compaction_tokens(original_tokens_u32);
                // CMPCT-050: close persisted orphan tool calls before the
                // compactor captures the conversation.
                let injected =
                    codelet_cli::interactive_helpers::inject_synthetic_tool_results_for_orphans(
                        &mut inner.messages,
                    );
                if injected > 0 {
                    tracing::warn!(
                        injected,
                        "[compact_session] CMPCT-050: closed {injected} persisted orphan tool_call(s) before compaction"
                    );
                }
                Ok(original_tokens_u32)
            })
        })?;
        if let Err(error) = self.queue_manual_compaction(compaction_epoch) {
            self.set_compaction_progress(None);
            self.set_status(SessionStatus::Idle);
            return Err(error);
        }
        Ok(CompactionResult {
            original_tokens,
            compacted_tokens: 0,
            compression_ratio: 0.0,
            turns_summarized: 0,
            turns_kept: 0,
        })
    }

    /// Queue one internal manual-compaction request for the agent loop.
    pub fn queue_manual_compaction(&self, epoch: u64) -> Result<(), String> {
        if self.compaction_cancelled(epoch) {
            return Err("manual compaction cancelled".into());
        }
        self.input_tx
            .try_send(PromptInput {
                input: MANUAL_COMPACTION_REQUEST.into(),
                thinking_config: None,
                compaction_epoch: Some(epoch),
                interrupt_epoch: epoch,
                submission: None,
            })
            .map_err(|error| format!("Failed to queue manual compaction: {error}"))
    }

    pub async fn discard_cancelled_compaction(&self, epoch: u64) -> bool {
        if !self.compaction_cancelled(epoch) {
            return false;
        }
        self.inner.lock().await.discard_compaction_working_view();
        self.compaction_in_progress.store(false, Ordering::Release);
        self.set_compaction_progress(None);
        self.set_status(SessionStatus::Idle);
        true
    }
}
