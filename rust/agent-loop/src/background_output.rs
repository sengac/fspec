//! BackgroundOutput + BackgroundProgressEmitter (RPC-072 lift from
//! `rust/napi/src/agent_loop.rs:1459-1768`).
//!
//! These two types implement
//! [`codelet_cli::interactive::StreamOutput`] — the sink the rig
//! streaming engine emits [`StreamEvent`](codelet_cli::interactive::StreamEvent)
//! values into. The job of [`BackgroundOutput`] is to translate every
//! variant into the equivalent
//! [`StreamChunk`](codelet_rpc_types::StreamChunk) and forward it to
//! [`BackgroundSession::handle_output`] for fan-out to subscribers
//! (TUI / RPC / bridge), while also driving per-turn persistence
//! (REFAC-007) and post_tool_use lifecycle hooks (HOOK-013).
//!
//! Verbatim lift of the canonical NAPI implementation. The single
//! NAPI-side `crate::inject_summary_handler::should_idle_on_done` call
//! site is rewired to [`crate::inject_summary_handler::should_idle_on_done`]
//! (the lifted copy in this crate). `crate::types::*` imports collapse
//! to [`codelet_rpc_types`] re-exports.

use std::sync::Arc;

use codelet_core::lifecycle_hooks::{run_post_tool, HookMessageLevel};
use codelet_core::persistence::AssistantContent;
use codelet_rpc_types::{
    ContextFillInfo, NotificationSeverity, SessionState, SessionStatus, StreamChunk, TokenTracker,
    ToolCallInfo, ToolProgressInfo, ToolResultInfo,
};
use codelet_sessions::background_session::BackgroundSession;

use crate::inject_summary_handler::should_idle_on_done;
use crate::persist::{
    persist_assistant_message_internal, persist_token_state, persist_tool_result_internal,
};
use crate::stream_loop_detector::{
    build_loop_abort_marker_note, build_loop_abort_recovery_message, LoopEscalationOutcome,
    LoopEscalationPolicy, StreamLoopDetector,
};

/// RIG-014: Default cooldown between loop-detector triggers before the
/// escalation policy aborts the in-flight stream.
pub const RIG014_LOOP_ABORT_COOLDOWN_SECS: u64 = 30;

/// Output handler for background sessions that implements StreamOutput.
///
/// REFAC-007: Accumulates assistant content blocks during streaming and
/// persists the complete assistant message on Done.
///
/// RIG-014: Feeds every Thinking/Text delta into per-channel streaming
/// loop detectors (`thinking_loop_detector` / `text_loop_detector`). The
/// detectors are independent per channel so a loop in one channel cannot
/// be masked by fresh content in the other. On escalation to abort the
/// session's existing interrupt machinery cancels the in-flight provider
/// stream, the degenerate tail is dropped from persisted content, and a
/// corrective note is staged for the next turn.
pub struct BackgroundOutput {
    pub(crate) session: Arc<BackgroundSession>,
    /// REFAC-007: Accumulated assistant content blocks for current turn
    assistant_content: std::sync::Mutex<Vec<AssistantContent>>,
    /// REFAC-007: Current provider name for message envelope
    provider: String,
    /// HOOK-013: Track last tool call name+args for post_tool_use hooks
    last_tool_call: std::sync::Mutex<Option<(String, serde_json::Value)>>,
    /// RIG-014: Streaming loop detector for the thinking channel.
    thinking_loop_detector: std::sync::Mutex<StreamLoopDetector>,
    /// RIG-014: Streaming loop detector for the text channel.
    text_loop_detector: std::sync::Mutex<StreamLoopDetector>,
    /// RIG-014: Shared warn→abort escalation policy across both channels.
    loop_escalation_policy: std::sync::Mutex<LoopEscalationPolicy>,
    /// RIG-014: Wall-clock instant of the last loop trigger (elapsed-time
    /// basis for the escalation policy).
    loop_last_trigger_at: std::sync::Mutex<Option<std::time::Instant>>,
    /// RIG-014: True once a loop abort has fired this turn (prevents
    /// re-feeding deltas into the detectors after the stream is cancelled).
    loop_abort_fired: std::sync::Mutex<bool>,
}

impl BackgroundOutput {
    pub fn with_provider(session: Arc<BackgroundSession>, provider: String) -> Self {
        Self {
            session,
            assistant_content: std::sync::Mutex::new(Vec::new()),
            provider,
            last_tool_call: std::sync::Mutex::new(None),
            thinking_loop_detector: std::sync::Mutex::new(StreamLoopDetector::new()),
            text_loop_detector: std::sync::Mutex::new(StreamLoopDetector::new()),
            loop_escalation_policy: std::sync::Mutex::new(LoopEscalationPolicy::new(
                std::time::Duration::from_secs(RIG014_LOOP_ABORT_COOLDOWN_SECS),
            )),
            loop_last_trigger_at: std::sync::Mutex::new(None),
            loop_abort_fired: std::sync::Mutex::new(false),
        }
    }

    /// RIG-014: Reset the per-turn loop detectors. Called at the start of
    /// each turn so detector windows do not carry over from the previous
    /// turn. (The `BackgroundOutput` is created per turn, so this is a
    /// defensive reset for the case where the same instance is reused.)
    pub fn reset_turn_loop_detectors(&self) {
        if let Ok(mut det) = self.thinking_loop_detector.lock() {
            det.reset();
        }
        if let Ok(mut det) = self.text_loop_detector.lock() {
            det.reset();
        }
        if let Ok(mut fired) = self.loop_abort_fired.lock() {
            *fired = false;
        }
    }

    /// RIG-014: Feed a thinking or text delta into the appropriate channel
    /// detector and apply the escalation policy. Returns `true` if the
    /// escalation policy escalated to abort (caller should trigger the
    /// session interrupt).
    fn feed_loop_detectors(&self, channel: &str, delta: &str) -> bool {
        // Skip feeding after an abort has already fired this turn — the
        // stream is being cancelled and further deltas are noise.
        if let Ok(fired) = self.loop_abort_fired.lock() {
            if *fired {
                return false;
            }
        }

        let det = if channel == "thinking" {
            &self.thinking_loop_detector
        } else {
            &self.text_loop_detector
        };

        let signal = {
            let Ok(mut guard) = det.lock() else {
                return false;
            };
            guard.feed(delta)
        };

        let Some(signal) = signal else {
            return false;
        };

        // Compute elapsed seconds since the previous trigger.
        let elapsed_secs = {
            let Ok(mut last) = self.loop_last_trigger_at.lock() else {
                return false;
            };
            let elapsed = last
                .as_ref()
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            *last = Some(std::time::Instant::now());
            elapsed
        };

        let outcome = {
            let Ok(mut policy) = self.loop_escalation_policy.lock() else {
                return false;
            };
            policy.on_trigger(signal.clone(), elapsed_secs)
        };

        match outcome {
            LoopEscalationOutcome::Warn => {
                tracing::warn!(
                    session = %self.session.id,
                    channel,
                    ?signal,
                    "[RIG-014] loop detector warning (streaming continues)"
                );
                false
            }
            LoopEscalationOutcome::Abort => {
                tracing::warn!(
                    session = %self.session.id,
                    channel,
                    ?signal,
                    "[RIG-014] loop detector abort — cancelling in-flight stream"
                );
                // Mark the abort as fired so further deltas are ignored.
                if let Ok(mut fired) = self.loop_abort_fired.lock() {
                    *fired = true;
                }
                // Append the marker note to the persisted content (the
                // degenerate tail is dropped because we stop accumulating
                // further deltas and the stream is cancelled).
                self.add_assistant_content(AssistantContent::Text {
                    text: build_loop_abort_marker_note(),
                });
                // Stage the corrective note for the next turn (session-level
                // so it survives the per-turn BackgroundOutput instance and
                // is consumed by the agent loop at the top of the next turn).
                let note = build_loop_abort_recovery_message(&signal, None);
                self.session.set_pending_loop_abort_note(note);
                // Drive the existing interrupt machinery to cancel the
                // in-flight provider stream. The stream loop checks
                // is_interrupted each iteration and stops.
                self.session.interrupt();
                true
            }
        }
    }

    /// REFAC-007: Add an assistant content block
    fn add_assistant_content(&self, content: AssistantContent) {
        let mut guard = self
            .assistant_content
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.push(content);
    }

    /// REFAC-007: Take all accumulated content (clears the buffer)
    fn take_assistant_content(&self) -> Vec<AssistantContent> {
        let mut guard = self
            .assistant_content
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *guard)
    }

    /// PROV-039: Persist the accumulated assistant message with optional stop_reason
    fn persist_assistant_message_with_stop_reason(&self, stop_reason: Option<String>) {
        let content = self.take_assistant_content();
        if content.is_empty() {
            return;
        }

        if let Err(e) = persist_assistant_message_internal(
            &self.session.id,
            &self.provider,
            content,
            stop_reason,
        ) {
            tracing::error!("REFAC-007: Failed to persist assistant message: {}", e);
        }
    }

    /// REFAC-007: Persist the accumulated assistant message (no stop_reason — for error/interrupt paths)
    fn persist_assistant_message(&self) {
        self.persist_assistant_message_with_stop_reason(None);
    }
}

impl codelet_cli::interactive::StreamOutput for BackgroundOutput {
    fn emit(&self, event: codelet_cli::interactive::StreamEvent) {
        use codelet_cli::interactive::StreamEvent;

        let chunk = match event {
            StreamEvent::Text(ref text) => {
                // RIG-014: Feed the text delta into the text-channel loop
                // detector. On abort the degenerate delta is NOT
                // accumulated (the tail is dropped) and the in-flight
                // stream is cancelled via the session interrupt.
                let loop_abort = self.feed_loop_detectors("text", text);
                if !loop_abort {
                    // REFAC-007: Accumulate text for later persistence
                    self.add_assistant_content(AssistantContent::Text {
                        text: text.clone(),
                    });
                }
                StreamChunk::text(text.clone())
            }
            StreamEvent::Thinking(ref thinking) => {
                // RIG-014: Feed the thinking delta into the thinking-channel
                // loop detector (independent window from the text channel).
                // On abort the degenerate delta is NOT accumulated.
                let loop_abort = self.feed_loop_detectors("thinking", thinking);
                if !loop_abort {
                    // REFAC-007: Accumulate thinking for later persistence
                    self.add_assistant_content(AssistantContent::Thinking {
                        thinking: thinking.clone(),
                        signature: None,
                    });
                }
                StreamChunk::thinking(thinking.clone())
            }
            StreamEvent::ToolCall(ref tc) => {
                // REFAC-007: Accumulate tool call for later persistence
                let input_value = serde_json::from_str(&tc.args.to_string())
                    .unwrap_or_else(|_| serde_json::Value::String(tc.args.to_string()));
                self.add_assistant_content(AssistantContent::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: input_value.clone(),
                });

                // HOOK-013: Capture tool call info for post_tool_use hooks
                if let Ok(mut last) = self.last_tool_call.lock() {
                    *last = Some((tc.name.clone(), input_value));
                }

                StreamChunk::tool_call(ToolCallInfo {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.args.to_string(),
                })
            }
            StreamEvent::ToolResult(ref tr) => {
                // REFAC-007: Persist accumulated assistant content BEFORE tool result
                // This ensures correct message order: user → assistant(text+tool_use) → tool_result → assistant(final)
                self.persist_assistant_message();

                // REFAC-007: Persist tool result immediately
                if let Err(e) =
                    persist_tool_result_internal(&self.session.id, &tr.id, &tr.content, tr.is_error)
                {
                    tracing::error!("REFAC-007: Failed to persist tool result: {}", e);
                }

                // HOOK-013: Run post_tool_use hooks (fire-and-forget with context injection)
                if let Some(ref hooks) = self.session.lifecycle_hooks {
                    if !hooks.post_tool_use.is_empty() {
                        let tool_name_for_hook = self
                            .last_tool_call
                            .lock()
                            .ok()
                            .and_then(|guard| guard.as_ref().map(|(name, _)| name.clone()));

                        if let Some(tool_name) = tool_name_for_hook {
                            let hooks_clone = hooks.clone();
                            let ctx = self.session.hook_context();
                            let tool_input = self
                                .last_tool_call
                                .lock()
                                .ok()
                                .and_then(|guard| guard.as_ref().map(|(_, input)| input.clone()))
                                .unwrap_or(serde_json::Value::Null);
                            let tool_response = tr.content.clone();
                            let session_for_hook = self.session.clone();

                            tokio::spawn(async move {
                                let outcome = run_post_tool(
                                    &hooks_clone,
                                    &ctx,
                                    &tool_name,
                                    &tool_input,
                                    &tool_response,
                                )
                                .await;
                                for context_text in &outcome.additional_context {
                                    session_for_hook.handle_output(StreamChunk::user_notification(
                                        format!("Hook context: {}", context_text),
                                        NotificationSeverity::Info,
                                    ));
                                }
                                for msg in &outcome.messages {
                                    if msg.level == HookMessageLevel::Warning
                                        || msg.level == HookMessageLevel::Error
                                    {
                                        tracing::warn!(
                                            "[HOOK-013] post_tool_use hook: {}",
                                            msg.content
                                        );
                                    }
                                }
                            });
                        }
                    }
                }

                // (Old KGRAPH entity pipeline was here — removed in KGRAPH-024 dual-graph migration)
                // CODE-009: FspecTool now uses fspec_handler (like pause_handler).
                StreamChunk::tool_result(ToolResultInfo {
                    tool_call_id: tr.id.clone(),
                    content: tr.content.clone(),
                    is_error: tr.is_error,
                })
            }
            StreamEvent::ToolProgress(tp) => StreamChunk::tool_progress(ToolProgressInfo {
                tool_call_id: tp.tool_call_id,
                tool_name: tp.tool_name,
                output_chunk: tp.output_chunk,
                is_stderr: tp.is_stderr,
            }),
            // NAPI-010: StreamEvent::Status messages are user-visible notifications
            StreamEvent::Status(status) => {
                StreamChunk::user_notification(status, NotificationSeverity::Info)
            }
            StreamEvent::Tokens(info) => {
                // Update cached tokens for sync access
                self.session
                    .update_tokens(info.input_tokens as u32, info.output_tokens as u32);
                if let Some(r) = info.reasoning_tokens {
                    self.session.update_reasoning_tokens(r as u32);
                }
                StreamChunk::token_update(TokenTracker {
                    input_tokens: info.input_tokens as u32,
                    output_tokens: info.output_tokens as u32,
                    cache_read_input_tokens: info.cache_read_input_tokens.map(|v| v as u32),
                    cache_creation_input_tokens: info.cache_creation_input_tokens.map(|v| v as u32),
                    tokens_per_second: info.tokens_per_second,
                    cumulative_billed_input: None,
                    cumulative_billed_output: None,
                    reasoning_tokens: info.reasoning_tokens.map(|v| v as u32),
                })
            }
            StreamEvent::ContextFill(info) => StreamChunk::context_fill_update(ContextFillInfo {
                fill_percentage: info.fill_percentage,
                effective_tokens: info.effective_tokens as f64,
                threshold: info.threshold as f64,
                context_window: info.context_window as f64,
            }),
            StreamEvent::Error(error) => {
                // REFAC-007: Persist any accumulated content before error
                self.persist_assistant_message();
                StreamChunk::error(error)
            }
            StreamEvent::Interrupted(queued) => {
                // REFAC-007: Persist any accumulated content on interrupt
                self.persist_assistant_message();
                StreamChunk::interrupted(queued)
            }
            StreamEvent::Done(stop_reason) => {
                // PROV-039: Persist accumulated assistant message with real stop_reason from provider
                self.persist_assistant_message_with_stop_reason(stop_reason);

                // REFAC-007 Rule [31]: Persist token state on Done chunk
                // TOKEN-003: carry the session-cumulative reasoning value
                let (input_tokens, output_tokens, reasoning_tokens) = self.session.get_tokens();
                if let Err(e) = persist_token_state(&self.session.id, input_tokens, output_tokens, reasoning_tokens.unwrap_or(0)) {
                    tracing::error!("REFAC-007: Failed to persist token state: {}", e);
                }

                // (Old KGRAPH entity pipeline flush was here — removed in KGRAPH-024 dual-graph migration)

                // Do NOT set Idle when compaction or pending DAG is active.
                if should_idle_on_done(
                    &self.session.compaction_in_progress,
                    &self.session.pending_dag_content,
                ) {
                    // NAPI-009-FIX: Set status to Idle BEFORE emitting Done chunk.
                    self.session.set_status(SessionStatus::Idle);
                }
                StreamChunk::done()
            }
            // UX-002: Structured compaction events
            StreamEvent::CompactionStarted => {
                self.session.set_status(SessionStatus::Compacting);
                // CMPCT-041: snapshot through the shared BackgroundSession
                // accessor so both AUTO twins and the manual writers agree
                // on an equivalent basis (see the accessor docs).
                self.session.snapshot_pre_compaction_tokens();
                StreamChunk::session_state_change(SessionState::Compacting)
            }
            StreamEvent::CompactionProgress(progress) => {
                // UX-002: Update session's compaction progress for TypeScript to poll
                self.session.update_compaction_progress(
                    progress.phase.clone(),
                    progress.current,
                    progress.total,
                );
                return; // Progress is polled via sessionGetCompactionProgress, not streamed
            }
            StreamEvent::CompactionComplete(info) => {
                // Fallback handler — in the DAG flow, CompactionComplete is emitted
                // directly by agent_loop via handle_output, not through StreamOutput.
                self.session.set_status(SessionStatus::Idle);
                self.session.set_compaction_progress(None);
                self.session
                    .handle_output(StreamChunk::session_state_change(SessionState::Idle));
                // UX-002: Send STRUCTURED CompactionComplete - no string parsing needed!
                StreamChunk::compaction_complete(codelet_rpc_types::CompactionResult {
                    original_tokens: info.original_tokens,
                    compacted_tokens: info.compacted_tokens,
                    compression_ratio: info.compression_ratio * 100.0,
                    turns_summarized: 0,
                    turns_kept: 0,
                })
            }
            StreamEvent::CompactionFailed { reason } => {
                self.session.set_status(SessionStatus::Idle);
                self.session.set_compaction_progress(None);
                self.session
                    .handle_output(StreamChunk::session_state_change(SessionState::Idle));
                StreamChunk::user_notification(
                    format!("Compaction failed: {reason}"),
                    NotificationSeverity::Warning,
                )
            }
            StreamEvent::CompactionContinuing => {
                self.session.set_status(SessionStatus::Running);
                StreamChunk::session_state_change(SessionState::Running)
            }
            // CONT-007: live continue/goal counter snapshot — map to the
            // state-only chunk, DROPPING the CLI-only transition reason.
            // CONT-008: EXCEPT GoalSatisfied — the engine cleared an active
            // goal (shared done() teardown), so this twin, which owns the
            // BackgroundSession, writes the chrome goal state back through
            // the shared guarded helper and marks the wire chunk with
            // goalCleared so the TUI drops its 🎯 cache.
            StreamEvent::ContinueState(cs) => {
                let goal_cleared = cs.reason
                    == codelet_cli::interactive::ContinueStateReason::GoalSatisfied;
                if goal_cleared {
                    self.session.clear_goal_state_if_unchanged_since_sync();
                }
                StreamChunk::continue_state_update(codelet_rpc_types::ContinueStateInfo {
                    enabled: cs.enabled,
                    budget: cs.budget,
                    nudges_used: cs.nudges_used,
                    goal_active: cs.goal_active,
                    effective_budget: cs.effective_budget,
                    goal_cleared,
                    done_rejections: cs.done_rejections,
                })
            }
        };

        self.session.handle_output(chunk);
    }

    fn progress_emitter(
        &self,
    ) -> Option<std::sync::Arc<dyn codelet_cli::interactive::StreamOutput>> {
        Some(std::sync::Arc::new(BackgroundProgressEmitter {
            session: self.session.clone(),
        }))
    }
}

/// Progress emitter for background sessions - can be captured in 'static closures
pub struct BackgroundProgressEmitter {
    pub session: Arc<BackgroundSession>,
}

impl codelet_cli::interactive::StreamOutput for BackgroundProgressEmitter {
    fn emit(&self, event: codelet_cli::interactive::StreamEvent) {
        // Only handle ToolProgress events
        if let codelet_cli::interactive::StreamEvent::ToolProgress(tp) = event {
            let chunk = StreamChunk::tool_progress(ToolProgressInfo {
                tool_call_id: tp.tool_call_id,
                tool_name: tp.tool_name,
                output_chunk: tp.output_chunk,
                is_stderr: tp.is_stderr,
            });
            self.session.handle_output(chunk);
        }
    }
}
