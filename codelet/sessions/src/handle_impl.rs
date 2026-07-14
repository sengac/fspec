//! The production `SessionManagerHandle` impl for the extracted
//! `SessionManager`, added by **RPC-042**.
//!
//! Each method delegates to the existing `SessionManager` or to a
//! `BackgroundSession` looked up via `self.get_session(...)`. The
//! per-session methods return safe defaults when the session is not
//! found so callers (the future `fspec` binary in RPC-044, the
//! `FspecService` in `codelet-rpc`, the WebSocket backend, etc.) can
//! treat the handle as totally robust.
//!
//! **Runtime requirement (RPC-070):** the sync→async bridges for
//! `create_session`, `create_isolated_session`, `test_provider_connection`,
//! and the three `/loop` methods invoke
//! `tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(...))`.
//! They MUST be invoked from a **multi-thread** tokio runtime — the
//! `fspec` binary's `#[tokio::main]` defaults to multi-thread and the
//! NAPI side's `#[napi(tokio_main)]` does too. `block_in_place`
//! temporarily detaches the current worker from the multi-thread
//! scheduler so the nested `block_on` does not panic with
//! "Cannot start a runtime from within a runtime" (the bug the
//! pre-RPC-070 code triggered when called from inside a tarpc handler;
//! see `spec/attachments/RPC-070/root-cause-analysis.md`).
//!
//! Calling these bridges from a single-thread tokio runtime panics
//! inside `block_in_place` itself with a clearer message than the old
//! nested-runtime panic. A `debug_assert!` in `loop_block_on` makes
//! the precondition explicit during development.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Duration, Utc};
use tokio::sync::broadcast;
use tracing;
use uuid::Uuid;

use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, CompactionProgress, CompactionResult, FspecResult,
    HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo, LogRecord, MergeOutcome,
    MergeStatus, MergeStrategy, ModelEntry, PauseState, ProviderCredentialInfo,
    ProviderCredentialInput, RegisteredLoop, ScheduledJob, SessionChangesSummary, SessionId,
    SessionInfo, SessionModel, SessionStatus, SessionTokens, SessionWorktreeInfo, StreamChunk,
    TestConnectionResult, ThinkingConfig, ThinkingLevel, TokenRestoreState, WorkUnitContext,
};

use crate::conversions::{
    approval_choice_to_pause_response, confirm_accept_to_pause_response, pause_state_to_rpc,
};
use crate::session_manager::SessionManager;

/// Parse a wire-portable `SessionId` into a UUID, falling back to
/// `Uuid::nil()` on parse failure so per-session methods can return
/// safe defaults rather than panicking on a malformed id.
fn uuid_from(id: &SessionId) -> Uuid {
    Uuid::parse_str(id.value.as_str()).unwrap_or_else(|_| Uuid::nil())
}

/// Production `SessionManagerHandle` impl for the extracted
/// `SessionManager`.
///
/// **Runtime requirement (RPC-070):** the sync→async bridge methods
/// (`create_session`, `create_isolated_session`,
/// `test_provider_connection`, `loop_add`, `loop_cancel`, `loop_list`)
/// wrap their `Handle::current().block_on(...)` calls in
/// `tokio::task::block_in_place(|| ...)`. The trait MUST be invoked
/// from a multi-thread tokio runtime — the `fspec` binary and the
/// napi `tokio_main` thread both satisfy this. Calling from a
/// single-thread runtime panics inside `block_in_place` with a clearer
/// message than the pre-RPC-070 "Cannot start a runtime from within
/// a runtime" nested-runtime panic.
impl codelet_core::SessionManagerHandle for SessionManager {
    fn list_sessions(&self) -> Vec<SessionInfo> {
        SessionManager::list_sessions(self)
    }

    /// Create a new session, returning its freshly-minted `SessionId`.
    ///
    /// **Runtime requirement (RPC-070):** bridges sync→async via
    /// `tokio::task::block_in_place(|| Handle::current().block_on(...))`.
    /// MUST be invoked from a multi-thread tokio runtime — the nested
    /// `block_on` is only legal because `block_in_place` first detaches
    /// the worker from the scheduler.
    fn create_session(&self, role: Option<String>) -> SessionId {
        let project = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        // PROV-101: NO silent selection fallback. When no default model has
        // been explicitly set, decline creation (empty SessionId) instead of
        // substituting "anthropic/claude-opus-4-5".
        let Some(model) = self.get_default_model() else {
            tracing::error!(
                "create_session declined: no default model set (PROV-101: no anthropic fallback)"
            );
            return SessionId::new(String::new());
        };

        tracing::info!(
            model = %model,
            project = %project,
            role = ?role,
            "create_session: starting session creation via handle"
        );

        let id_string = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { SessionManager::create_session(self, &model, &project).await })
        })
        .unwrap_or_default();

        tracing::info!(
            session_id = %id_string,
            "create_session: session creation returned from SessionManager"
        );

        if let Some(role_str) = role {
            if !role_str.is_empty() {
                if let Ok(session) = self.get_session(&id_string) {
                    session.set_role(role_str.clone());
                    tracing::info!(
                        session_id = %id_string,
                        role = %role_str,
                        "create_session: role set on session"
                    );
                }
            }
        }
        SessionId::new(id_string)
    }

    /// RPC-422: Resume a persisted session by first creating it in memory,
    /// then restoring messages and token state.
    ///
    /// This override ensures the BackgroundSession exists in the in-memory
    /// session map before `restore_session_messages` tries to look it up.
    fn resume_session(&self, session_id: &SessionId) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(&session_id.value)
            .map_err(|e| format!("invalid session id: {e}"))?;

        tracing::info!(
            session_id = %uuid,
            "resume_session: starting session resume"
        );

        // Load the manifest from persistence
        let manifest = codelet_core::persistence::load_session(uuid)
            .map_err(|e| format!("Failed to load session manifest: {}", e))?;

        tracing::info!(
            session_id = %uuid,
            manifest_provider = %manifest.provider,
            manifest_name = %manifest.name,
            manifest_message_count = manifest.messages.len(),
            "resume_session: loaded manifest from disk"
        );

        // Check if the session already exists in memory
        let already_exists = self.get_session(&session_id.value).is_ok();

        tracing::info!(
            session_id = %uuid,
            already_exists,
            "resume_session: checking if session exists in memory"
        );

        // If not in memory, create it from the existing manifest WITHOUT overwriting it
        if !already_exists {
            let model = if manifest.provider.is_empty() {
                self.get_default_model()
                    .ok_or("No default model set and manifest has no provider")?
            } else {
                manifest.provider.clone()
            };
            tracing::info!(
                session_id = %uuid,
                model = %model,
                manifest_message_count = manifest.messages.len(),
                "resume_session: session not in memory, creating from existing manifest"
            );
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async {
                        SessionManager::create_session_from_manifest(self, &manifest, &model)
                            .await
                    })
            })?;
            tracing::info!(
                session_id = %uuid,
                "resume_session: session created in memory from manifest"
            );
        }

        // Now restore messages and token state
        tracing::info!(
            session_id = %uuid,
            "resume_session: restoring messages and token state"
        );
        let envelopes = codelet_core::persistence::get_session_message_envelopes(uuid)?;
        tracing::info!(
            session_id = %uuid,
            envelope_count = envelopes.len(),
            "resume_session: loaded message envelopes"
        );
        let state = TokenRestoreState {
            current_context: manifest.token_usage.current_context_tokens as i64,
            cumulative_billed_output: manifest.token_usage.cumulative_billed_output as i64,
            cache_read: manifest.token_usage.cache_read_tokens as i64,
            cache_creation: manifest.token_usage.cache_creation_tokens as i64,
            cumulative_billed_input: manifest.token_usage.cumulative_billed_input as i64,
            cumulative_billed_output_second: manifest.token_usage.cumulative_billed_output as i64,
        };
        self.restore_session_messages(session_id, envelopes)?;
        self.restore_session_token_state(session_id, state)?;

        tracing::info!(
            session_id = %uuid,
            "resume_session: session resume complete"
        );
        Ok(())
    }

    fn send_input(&self, session_id: &SessionId, text: String) {
        let uuid = uuid_from(session_id);
        if let Ok(session) = self.get_session(&uuid.to_string()) {
            let _ = session.send_input(text, None);
        }
    }

    fn send_input_with_thinking(
        &self,
        session_id: &SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) {
        let uuid = uuid_from(session_id);
        if let Ok(session) = self.get_session(&uuid.to_string()) {
            let thinking_json = thinking.and_then(|t| serde_json::to_string(&t).ok());
            let _ = session.send_input(text, thinking_json);
        }
    }

    fn interrupt(&self, session_id: &SessionId) {
        let uuid = uuid_from(session_id);
        if let Ok(session) = self.get_session(&uuid.to_string()) {
            session.interrupt();
        }
    }

    fn get_session_status(&self, session_id: &SessionId) -> SessionStatus {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .map(|s| s.get_status())
            .unwrap_or(SessionStatus::Idle)
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        SessionManager::chunks_tx(self).subscribe()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        SessionManager::logs_tx(self).subscribe()
    }

    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        SessionManager::chunks_tx(self).clone()
    }

    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        SessionManager::logs_tx(self).clone()
    }

    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        SessionManager::status_changes_tx(self).subscribe()
    }

    fn status_changes_tx(&self) -> broadcast::Sender<(SessionId, SessionStatus)> {
        SessionManager::status_changes_tx(self).clone()
    }

    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        SessionManager::session_created_tx(self).subscribe()
    }

    fn get_session_tokens(&self, session_id: &SessionId) -> SessionTokens {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .map(|s| {
                let (input, output, _reasoning) = s.get_tokens();
                SessionTokens {
                    input_tokens: input as i64,
                    output_tokens: output as i64,
                }
            })
            .unwrap_or(SessionTokens {
                input_tokens: 0,
                output_tokens: 0,
            })
    }

    fn get_session_model(&self, session_id: &SessionId) -> SessionModel {
        use std::sync::atomic::Ordering;
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .map(|s| {
                let provider_id = s
                    .provider_id
                    .read()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();
                let model_id = s
                    .model_id
                    .read()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();
                SessionModel {
                    provider_id,
                    model_id,
                    context_window: s.cached_context_window.load(Ordering::Acquire) as i64,
                    max_output_tokens: s.cached_max_output_tokens.load(Ordering::Acquire) as i64,
                    compaction_threshold: s.cached_compaction_threshold.load(Ordering::Acquire)
                        as i64,
                }
            })
            .unwrap_or(SessionModel {
                provider_id: String::new(),
                model_id: String::new(),
                context_window: 0,
                max_output_tokens: 0,
                compaction_threshold: 0,
            })
    }

    fn get_compaction_progress(&self, session_id: &SessionId) -> Option<CompactionProgress> {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .ok()
            .and_then(|s| s.get_compaction_progress())
            .map(CompactionProgress::from)
    }

    fn get_buffered_output(&self, session_id: &SessionId, limit: u32) -> Vec<StreamChunk> {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .map(|s| s.get_buffered_output(limit as usize))
            .unwrap_or_default()
    }

    fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                // RPC-073: BackgroundSession::clear_history calls
                // `self.inner.blocking_lock()` (see
                // codelet/sessions/src/background_session.rs:1156),
                // which panics with "Cannot block the current thread
                // from within a runtime" when invoked from inside a
                // tokio multi-thread worker (the tarpc dispatcher).
                // Wrap in `tokio::task::block_in_place` so the worker
                // is temporarily detached from the scheduler — same
                // pattern as the other sync→async bridges in this
                // file (`create_session`, `create_isolated_session`,
                // `test_provider_connection`, the three `loop_*`
                // methods).
                tokio::task::block_in_place(|| session.clear_history());
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> {
        // RPC-418: Real in-view DAG compaction for the manual /compact path.
        // Mirrors the NAPI reference `session_compact`
        // (codelet/napi/src/session_bindings.rs:3038-3130): clears the
        // conversation to system-reminders, injects the compaction
        // instruction, resets the token tracker, then kicks the agent loop
        // with "Continue" so it builds the hierarchical summary DAG.
        use codelet_cli::interactive_helpers::execute_compaction;

        let uuid = uuid_from(session_id);
        let session = self
            .get_session(&uuid.to_string())
            .map_err(|_| format!("Session not found: {}", session_id.value.as_str()))?;

        // Bridge sync→async: `session.inner` is a `tokio::sync::Mutex` and
        // `execute_compaction` is async. Mirror the block_in_place +
        // block_on pattern used by `restore_session_messages` (requires a
        // multi-thread runtime — see the module docs). The async block
        // returns the captured token counts so the `inner` lock is dropped
        // BEFORE `send_input` (the agent loop needs the lock — see the
        // `drop(inner)` at session_bindings.rs:3097).
        let original_tokens: u64 = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut inner = session.inner.lock().await;

                // Nothing to compact — leave the session untouched (no
                // status change, no token snapshot).
                if inner.messages.is_empty() {
                    return Err("Nothing to compact - no messages yet".to_string());
                }

                session.set_status(SessionStatus::Compacting);

                let original_tokens = inner.token_tracker.input_tokens;
                // CMPCT-041: route the manual tracker-based snapshot through
                // the shared BackgroundSession accessor (basis unification
                // with the AUTO CompactionStarted writers).
                session.store_pre_compaction_tokens(original_tokens as u32);

                // `None` = manual/agent-initiated compaction (no resume prompt).
                if let Err(e) =
                    execute_compaction(&mut inner, session.compaction_in_progress.clone(), None)
                        .await
                {
                    session.set_compaction_progress(None);
                    session.set_status(SessionStatus::Idle);
                    return Err(format!("Compaction failed: {e}"));
                }

                // RPC-421: do NOT read the tracker here — it measures the
                // post-clear trough (reminders + compaction instruction),
                // not a real reduction. The DAG summary does not exist yet.
                Ok(original_tokens)
            })
        })?;

        // Status contract: on the happy path the session intentionally stays in
        // `Compacting` here — `send_input("Continue")` below flips it to `Running`
        // (NAPI parity, session_bindings.rs). Only the failure paths above revert to
        // `Idle`. Do not add an unconditional Idle reset here or the agent-loop kick
        // would race the status.

        // Lock dropped. Clear progress and kick the agent loop so it builds
        // the DAG via SessionSearch and calls the inject_summary tool.
        session.set_compaction_progress(None);

        // Mirror NAPI: a failed `send_input` is logged, not fatal — the
        // compaction itself already succeeded.
        if let Err(e) = session.send_input("Continue".to_string(), None) {
            tracing::warn!("[compact_session] Failed to send Continue to agent loop: {e}");
            session.set_status(SessionStatus::Idle);
        }

        // RPC-421: acknowledgement-shaped success on the unchanged wire
        // schema. The final compacted size is unknowable at RPC-return time —
        // the agent builds the DAG asynchronously after the "Continue" kick —
        // so this result answers only "did compaction start successfully?".
        // original_tokens is the real pre-compaction snapshot; every other
        // field is the 0-valued sentinel. Consumers MUST NOT present these
        // fields as a reduction: the StreamChunk::CompactionComplete emission
        // (CMPCT-038 apply-site) is the single source of truth for the
        // numbers. NAPI twin: session_bindings.rs session_compact.
        Ok(CompactionResult {
            original_tokens: original_tokens as u32,
            compacted_tokens: 0,
            compression_ratio: 0.0,
            turns_summarized: 0,
            turns_kept: 0,
        })
    }

    fn restore_session_messages(
        &self,
        session_id: &SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String> {
        // RPC-081: NAPI-free port of codelet/napi/src/session_bindings.rs:2401-2567.
        // Walks each envelope JSON's message.content blocks, builds parallel
        // rig::message::Message + StreamChunk vectors, then pushes rig messages
        // into session.inner.messages and dispatches StreamChunks via
        // session.handle_output. Preserves the system-reminder skip rule
        // (text contains both "<system-reminder>" AND "<!-- type:") so stale
        // reminders are NOT replayed — fresh ones are re-injected post-restore.
        let uuid = uuid_from(session_id);
        let session = self
            .get_session(&uuid.to_string())
            .map_err(|_| format!("Session not found: {}", session_id.value.as_str()))?;

        let mut rig_messages: Vec<rig::message::Message> = Vec::new();
        let mut stream_chunks: Vec<StreamChunk> = Vec::new();

        for envelope_json in &envelopes {
            let envelope: serde_json::Value = serde_json::from_str(envelope_json)
                .map_err(|e| format!("Failed to parse envelope: {e}"))?;

            let Some(message) = envelope.get("message") else {
                continue;
            };

            let role = message
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");

            if role == "assistant" {
                let Some(content) = message.get("content") else {
                    continue;
                };
                let Some(arr) = content.as_array() else {
                    continue;
                };

                let mut text_parts: Vec<String> = Vec::new();

                for block in arr {
                    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    match block_type {
                        "thinking" => {
                            if let Some(thinking) = block.get("thinking").and_then(|t| t.as_str()) {
                                if !thinking.is_empty() {
                                    stream_chunks.push(StreamChunk::thinking(thinking.to_string()));
                                }
                            }
                        }
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                text_parts.push(text.to_string());
                                if !text.is_empty() {
                                    stream_chunks.push(StreamChunk::text(text.to_string()));
                                }
                            }
                        }
                        "tool_use" => {
                            let id = block
                                .get("id")
                                .and_then(|i| i.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = block
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string();
                            let input = block
                                .get("input")
                                .map(|i| serde_json::to_string(i).unwrap_or_default())
                                .unwrap_or_default();

                            if !id.is_empty() && !name.is_empty() {
                                stream_chunks.push(StreamChunk::tool_call(
                                    codelet_rpc_types::ToolCallInfo { id, name, input },
                                ));
                            }
                        }
                        _ => {}
                    }
                }

                let joined_text = text_parts.join("");
                if !joined_text.is_empty() {
                    rig_messages.push(rig::message::Message::Assistant {
                        id: None,
                        content: rig::OneOrMany::one(rig::message::AssistantContent::text(
                            joined_text,
                        )),
                    });
                }

                stream_chunks.push(StreamChunk::done());
            } else {
                let Some(content) = message.get("content") else {
                    continue;
                };

                if let Some(arr) = content.as_array() {
                    let mut text_parts: Vec<String> = Vec::new();
                    let mut chunks_for_this_envelope: Vec<StreamChunk> = Vec::new();

                    for block in arr {
                        let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                        match block_type {
                            "text" => {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    text_parts.push(text.to_string());
                                    if !text.is_empty() {
                                        chunks_for_this_envelope
                                            .push(StreamChunk::user_input(text.to_string()));
                                    }
                                }
                            }
                            "tool_result" => {
                                let tool_use_id = block
                                    .get("tool_use_id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let result_content = block
                                    .get("content")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let is_error = block
                                    .get("is_error")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(false);

                                if !tool_use_id.is_empty() {
                                    chunks_for_this_envelope.push(StreamChunk::tool_result(
                                        codelet_rpc_types::ToolResultInfo {
                                            tool_call_id: tool_use_id,
                                            content: result_content,
                                            is_error,
                                        },
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }

                    let joined_text = text_parts.join("");
                    if !joined_text.is_empty() {
                        // Skip system reminders — they'll be re-injected fresh after restore.
                        if joined_text.contains("<system-reminder>")
                            && joined_text.contains("<!-- type:")
                        {
                            continue;
                        }
                        rig_messages.push(rig::message::Message::User {
                            content: rig::OneOrMany::one(rig::message::UserContent::text(
                                joined_text,
                            )),
                        });
                        stream_chunks.push(StreamChunk::done());
                    }

                    // Flush this envelope's stream chunks (text user_input +
                    // tool_result). For system-reminder-text envelopes the
                    // `continue` above skipped this block entirely, so no
                    // chunks are emitted — matching the NAPI behaviour.
                    stream_chunks.extend(chunks_for_this_envelope);
                } else if let Some(s) = content.as_str() {
                    if !s.is_empty() {
                        if s.contains("<system-reminder>") && s.contains("<!-- type:") {
                            continue;
                        }
                        stream_chunks.push(StreamChunk::user_input(s.to_string()));
                        rig_messages.push(rig::message::Message::User {
                            content: rig::OneOrMany::one(rig::message::UserContent::text(
                                s.to_string(),
                            )),
                        });
                        stream_chunks.push(StreamChunk::done());
                    }
                }
            }
        }

        // Push rig messages into session.inner.messages.
        // The trait method is sync; session.inner is a tokio::sync::Mutex,
        // so we bridge via block_in_place + Handle::current.block_on. This
        // mirrors the existing sync→async bridges in this file
        // (create_session, clear_history, etc.).
        if !rig_messages.is_empty() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let mut inner = session.inner.lock().await;
                    for msg in rig_messages {
                        inner.messages.push(msg);
                    }
                });
            });
        }

        // Dispatch stream chunks via session.handle_output for UI replay.
        for chunk in stream_chunks {
            session.handle_output(chunk);
        }

        Ok(())
    }

    fn restore_session_token_state(
        &self,
        session_id: &SessionId,
        _state: TokenRestoreState,
    ) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(_session) => Ok(()),
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn get_work_unit_context(&self, session_id: &SessionId) -> Option<WorkUnitContext> {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .ok()
            .and_then(|s| s.get_work_unit_context())
            .map(WorkUnitContext::from)
    }

    fn set_work_unit_context(
        &self,
        session_id: &SessionId,
        ctx: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                match ctx {
                    Some(c) => {
                        session.set_work_unit_context(Some(c.id), Some(c.title), Some(c.status))
                    }
                    None => session.set_work_unit_context(None, None, None),
                }
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn get_pending_input(&self, session_id: &SessionId) -> Option<String> {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .ok()
            .and_then(|s| s.get_pending_input())
    }

    fn set_pending_input(&self, session_id: &SessionId, text: Option<String>) {
        let uuid = uuid_from(session_id);
        if let Ok(session) = self.get_session(&uuid.to_string()) {
            session.set_pending_input(text);
        }
    }

    fn set_active_session(&self, session_id: &SessionId) {
        let uuid = uuid_from(session_id);
        SessionManager::set_active_session(self, uuid);
    }

    fn clear_active_session(&self) {
        SessionManager::clear_active_session(self);
    }

    fn get_active_session(&self) -> Option<SessionId> {
        SessionManager::get_active_session(self).map(|u| SessionId::new(u.to_string()))
    }

    fn get_effective_cwd(&self, session_id: &SessionId) -> PathBuf {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .ok()
            .map(|s| s.effective_cwd())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    fn get_supervisors(&self, session_id: &SessionId) -> Vec<SessionId> {
        let uuid = uuid_from(session_id);
        SessionManager::get_supervisors(self, uuid)
            .into_iter()
            .map(|u| SessionId::new(u.to_string()))
            .collect()
    }

    /// RPC-061: delegate to ChainOfCommand::add_supervisor (surfaces
    /// the "circular supervision not allowed" / "subordinate already
    /// registered under this supervisor" errors verbatim).
    fn add_supervisor(
        &self,
        subordinate_id: &SessionId,
        supervisor_id: &SessionId,
    ) -> Result<(), String> {
        let sub_uuid = uuid_from(subordinate_id);
        let sup_uuid = uuid_from(supervisor_id);
        SessionManager::add_supervisor(self, sub_uuid, sup_uuid)
    }

    /// RPC-061: delegate to ChainOfCommand::remove_supervisor. The
    /// underlying method is `()`-returning; we wrap into Ok(()).
    fn remove_supervisor(&self, supervisor_id: &SessionId) -> Result<(), String> {
        let sup_uuid = uuid_from(supervisor_id);
        SessionManager::remove_supervisor(self, sup_uuid);
        Ok(())
    }

    /// RPC-061: first subordinate of the supervisor (or None).
    fn get_subordinate(&self, supervisor_id: &SessionId) -> Option<SessionId> {
        let sup_uuid = uuid_from(supervisor_id);
        SessionManager::get_subordinate(self, sup_uuid).map(|u| SessionId::new(u.to_string()))
    }

    /// RPC-061: every subordinate of the supervisor.
    fn get_subordinates(&self, supervisor_id: &SessionId) -> Vec<SessionId> {
        let sup_uuid = uuid_from(supervisor_id);
        SessionManager::get_subordinates(self, sup_uuid)
            .into_iter()
            .map(|u| SessionId::new(u.to_string()))
            .collect()
    }

    /// RPC-061: queue a supervisor message onto the subordinate's
    /// BackgroundSession `receive_incoming_message` mpsc channel.
    /// Returns Err("Session not found: …") when the subordinate id
    /// does not match a live session.
    fn receive_incoming_message(
        &self,
        subordinate_id: &SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        let uuid = uuid_from(subordinate_id);
        let session = self.get_session(&uuid.to_string())?;
        let bridge_images = message.images.map(|imgs| {
            imgs.into_iter()
                .map(|img| crate::background_session::BridgeImageData {
                    data: img.data,
                    media_type: img.media_type,
                })
                .collect::<Vec<_>>()
        });
        let input = crate::background_session::IncomingMessage::with_images(
            message.source_session_id,
            message.role_name,
            message.message,
            bridge_images,
        )?;
        session.receive_incoming_message(input)
    }

    fn get_debug_enabled(&self, session_id: &SessionId) -> bool {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .map(|s| s.get_debug_enabled())
            .unwrap_or(false)
    }

    fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool) {
        let uuid = uuid_from(session_id);
        if let Ok(session) = self.get_session(&uuid.to_string()) {
            session.set_debug_enabled(enabled);
        }
    }

    fn toggle_debug(&self, session_id: &SessionId, debug_dir: &str) -> Result<String, String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                // RPC-055: port of NAPI's session_toggle_debug
                // (codelet/napi/src/session_bindings.rs:2645). We:
                //   1. Compute per-session debug directory
                //      `{debug_dir}/debug/{session_id}/`.
                //   2. Lock the session's DebugCaptureManager and call
                //      start_capture or stop_capture depending on the
                //      current enabled flag.
                //   3. Persist the new enabled flag on the
                //      BackgroundSession atomic.
                //   4. Emit `StreamChunk::DebugStateChange` for the TUI.
                let session_debug_dir = std::path::PathBuf::from(debug_dir)
                    .join("debug")
                    .join(session.id.to_string());
                let was_enabled = session.get_debug_enabled();
                let result = {
                    let mut manager = session.debug_capture.lock().map_err(|_| {
                        "Failed to acquire lock on per-session debug capture manager".to_string()
                    })?;
                    manager.set_debug_directory_raw(session_debug_dir);
                    if was_enabled {
                        manager.stop_capture().map_err(|e| e.to_string())?
                    } else {
                        manager.start_capture().map_err(|e| e.to_string())?
                    }
                };
                let new_enabled = !was_enabled;
                session.set_debug_enabled(new_enabled);
                let _ = SessionManager::chunks_tx(self).send((
                    session_id.clone(),
                    StreamChunk::debug_state_change(new_enabled),
                ));
                Ok(result)
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn set_debug_directory(&self, path: PathBuf) -> Result<(), String> {
        // RPC-055: pre-session global toggle path. Delegates to the
        // codelet-common global DebugCaptureManager singleton. This is
        // ONLY used before any session exists — once a session is
        // created, `toggle_debug(session_id, debug_dir)` is the
        // preferred entry point (it uses the per-session manager on
        // BackgroundSession).
        match codelet_common::debug_capture::get_debug_capture_manager() {
            Ok(arc) => {
                let mut manager = arc.lock().map_err(|_| {
                    "Failed to acquire lock on global debug capture manager".to_string()
                })?;
                manager.set_debug_directory(path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to access debug capture manager: {e}")),
        }
    }

    fn pause_resume(&self, session_id: &SessionId) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.send_pause_response(codelet_tools::tool_pause::PauseResponse::Resumed);
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn pause_confirm(&self, session_id: &SessionId, accept: bool) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.send_pause_response(confirm_accept_to_pause_response(accept));
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn pause_triple(&self, session_id: &SessionId, choice: ApprovalChoice) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.send_pause_response(approval_choice_to_pause_response(choice));
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn send_hitl_response(
        &self,
        session_id: &SessionId,
        response: HitlResponse,
    ) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                // RPC-410: direct pass-through mapping — the wire
                // payload {cancelled, answers} is authoritative; no
                // option-label inference, no reading of the pending
                // request to classify answers.
                session.send_hitl_response(crate::hitl_mapping::wire_response_to_internal(
                    response.cancelled,
                    response.answers,
                ));
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn get_pause_state(&self, session_id: &SessionId) -> Option<PauseState> {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .ok()
            .and_then(|s| s.get_pause_state())
            .map(pause_state_to_rpc)
    }

    fn get_hitl_request(&self, session_id: &SessionId) -> Option<HitlRequest> {
        let uuid = uuid_from(session_id);
        let internal = self
            .get_session(&uuid.to_string())
            .ok()
            .and_then(|s| s.get_hitl_request())?;
        // RPC-410: full pass-through — every question surfaces on the
        // wire in order (options: None → empty vec).
        Some(crate::hitl_mapping::internal_request_to_wire(internal))
    }

    fn send_fspec_result(&self, session_id: &SessionId, result: FspecResult) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.send_fspec_result(result);
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    /// Create an isolated (worktree-backed) session.
    ///
    /// **Runtime requirement (RPC-070):** bridges sync→async via
    /// `tokio::task::block_in_place(|| Handle::current().block_on(...))`.
    /// MUST be invoked from a multi-thread tokio runtime.
    fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String> {
        let id = Uuid::new_v4();
        let project = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();
        let model = self.get_default_model().ok_or_else(|| {
            "create_isolated_session declined: no default model set \
             (PROV-101: no anthropic fallback)"
                .to_string()
        })?;
        let name = format!("isolated-{}", &id.to_string()[..8]);
        let info = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.create_isolated_session_with_id(&id.to_string(), &model, &project, &name)
                    .await
            })
        })?;
        if let Some(role_str) = role {
            if !role_str.is_empty() {
                if let Ok(session) = self.get_session(&id.to_string()) {
                    session.set_role(role_str);
                }
            }
        }
        Ok(info)
    }

    fn set_thinking_level_default(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        // TUI-002: mirror `set_default_model`'s entry/success logging so the
        // default-thinking write path is observable in the combined log.
        tracing::info!(
            level = level as u8,
            session_id = %session_id.value.as_str(),
            "set_thinking_level_default: persisting + applying default thinking level"
        );
        // TUI-002: persist the chosen default ALWAYS (user-level setting,
        // mirrors TS `saveDefaultThinkingLevel`). Best-effort: a persistence
        // failure is non-fatal and must not block the in-memory apply.
        if let Err(e) =
            crate::default_thinking_level_persistence::save_default_thinking_level(level)
        {
            tracing::warn!("Failed to persist default thinking level: {e}");
        }
        // Apply in-memory when the session exists (idle render reflects it).
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.set_base_thinking_level(level as u8);
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn destroy_session(&self, session_id: &SessionId) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        SessionManager::destroy_session(self, &uuid.to_string()).map_err(|e| {
            if e.contains("Session not found") {
                e
            } else {
                format!("Session not found: {}: {}", session_id.value.as_str(), e)
            }
        })
    }

    fn get_model_info(&self, session_id: &SessionId) -> codelet_rpc_types::ModelInfo {
        use std::sync::atomic::Ordering;
        let uuid = uuid_from(session_id);
        // TUI-001: build the models.dev registry once so `resolve_model_info`
        // can promote the raw slug to the friendly catalog name + capability
        // flags. `None` outside a multi-thread runtime / on cache failure →
        // graceful fallback to the raw slug (mirrors the TS fallback path).
        let registry = build_cloud_registry();
        self.get_session(&uuid.to_string())
            .map(|s| {
                let provider_id = s
                    .provider_id
                    .read()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();
                let model_id = s
                    .model_id
                    .read()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();
                crate::cloud_models::resolve_model_info(
                    registry.as_ref(),
                    &provider_id,
                    &model_id,
                    s.cached_context_window.load(Ordering::Acquire),
                    s.cached_compaction_threshold.load(Ordering::Acquire),
                )
            })
            .unwrap_or_default()
    }

    fn get_thinking_level(&self, session_id: &SessionId) -> ThinkingLevel {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .map(|s| match s.get_base_thinking_level() {
                0 => ThinkingLevel::Off,
                1 => ThinkingLevel::Low,
                2 => ThinkingLevel::Medium,
                _ => ThinkingLevel::High,
            })
            .unwrap_or(ThinkingLevel::Off)
    }

    fn list_providers(&self) -> Vec<codelet_rpc_types::ProviderInfo> {
        // RPC-073: wire the providers registry into the /model picker.
        // Mirrors the TS Ink path (NAPI `list_providers` →
        // `codelet_providers::custom::list_providers_info`) so the
        // Rust ratatui ModelSelectorDialog sees the same provider tree
        // as the TS Ink ModelSelectorView.
        //
        // The tarpc wire format `codelet_rpc_types::ProviderInfo` is
        // lossy vs. the 9-field `codelet_providers::custom::ProviderInfo`
        // (no facade / base_url / api_key_env_var / api_style /
        // is_custom / available — the latter is exposed via the
        // sibling `list_provider_credentials` for the /provider view).
        // We map field-by-field here:
        //   * `name` → `key` (canonical provider slug used by
        //     `set_session_model`).
        //   * `display_name.unwrap_or(name)` → `display_name` (matches
        //     the existing precedent at handle_impl.rs:803 for
        //     `list_provider_credentials`).
        //   * Each `ProviderModelInfo` → `ModelEntry` with
        //     `supports_thinking` → `supports_reasoning`, `is_custom`
        //     propagated from parent, `usize` → `u32` saturating.
        // Built-in providers carry empty `models`; custom providers
        // surface their declared models.
        //
        // On Err the underlying helper failed (e.g. corrupt
        // `~/.fspec/providers/*.json`). We log via `tracing::error`
        // and return `Vec::new()` so the model dialog opens empty
        // rather than panicking (matches the
        // `list_provider_credentials` graceful-degradation pattern).
        match codelet_providers::custom::list_providers_info() {
            Ok(list) => {
                // RPC-073 (reopened): source the models.dev catalog ONCE so
                // built-in/cloud providers expose their tool-capable models
                // (credential-gated) instead of empty rows. Mirrors TS
                // `loadCloudModels` + `buildCloudSections`. Graceful: any load
                // failure (cold cache offline, corrupt file) → None → the
                // built-ins keep their empty model lists (prior behaviour).
                let cloud_registry = build_cloud_registry();
                // PROV-130: partition the mapped sections into the cloud
                // (built-in / canonical) bucket and the custom-provider bucket
                // by the SOURCE `p.is_custom` flag. The wire `ProviderInfo`
                // carries no section-level `is_custom`, so the split must happen
                // here at the map, before it is erased. Mirrors the TS
                // `cloudSections` vs `customSections` distinction consumed by
                // `modelInitializationService.ts`.
                let mut cloud_sections: Vec<codelet_rpc_types::ProviderInfo> = Vec::new();
                let mut custom_sections: Vec<codelet_rpc_types::ProviderInfo> = Vec::new();
                for p in list {
                    let is_custom = p.is_custom;
                    let display_name = p.display_name.unwrap_or_else(|| p.name.clone());
                    let mut models: Vec<codelet_rpc_types::ModelEntry> = p
                        .models
                        .into_iter()
                        .map(|m| codelet_rpc_types::ModelEntry {
                            id: m.id.clone(),
                            display_name: m.id,
                            context_window: u32::try_from(m.context_window).unwrap_or(u32::MAX),
                            supports_reasoning: m.supports_thinking,
                            supports_vision: m.supports_vision,
                            is_custom,
                        })
                        .collect();
                    // RPC-073: built-in (non-custom) providers carry no
                    // declared models — fill them from the models.dev
                    // registry, gated on configured credentials.
                    if !is_custom && models.is_empty() {
                        if let Some(registry) = cloud_registry.as_ref() {
                            let has_creds = crate::cloud_models::provider_has_credentials(&p.name);
                            models = crate::cloud_models::cloud_model_entries(
                                registry, &p.name, has_creds,
                            );
                        }
                    }
                    // RPC-338: cloud / custom providers are never profile
                    // sections and are always treated as reachable.
                    let info = crate::profile_sections::cloud_provider_info(
                        &p.name,
                        &display_name,
                        models,
                    );
                    if is_custom {
                        custom_sections.push(info);
                    } else {
                        cloud_sections.push(info);
                    }
                }
                // PROV-129: synthesize the "Codex (ChatGPT)" section by
                // re-parenting the OpenAI cloud models when Codex credentials
                // (OAuth or Codex API key) are present. Mirrors TS
                // `cloudSectionBuilder.ts` `extractCodexSection` (:191-237) +
                // the `openai.hasCredentials = hasCodexCredentials` override
                // (:117-119): the OpenAI catalog is allowlist-filtered under a
                // synthetic Codex identity, the standalone OpenAI section is
                // removed, and (PROV-130) the Codex section LEADS the cloud
                // group. Applied to the CLOUD bucket only (customs are never
                // re-parented) and BEFORE the PROV-127 drop-empty filter so the
                // now-populated codex header survives and the now-empty openai
                // header is dropped.
                let cloud_sections = if let Some(registry) = cloud_registry.as_ref() {
                    crate::cloud_models::synthesize_codex_section(cloud_sections, registry)
                } else {
                    cloud_sections
                };
                // PROV-127: drop cloud/custom sections whose model list is empty
                // so uncredentialed/known-absent providers do not render as dead
                // "Provider (0 models)" headers. Mirrors TS
                // `cloudSectionBuilder.ts` (`filter(s => s.hasCredentials)`) +
                // `modelInitializationService.ts` (`filter(s => s.models.length
                // > 0)`). Local profile sections (appended below) may legitimately
                // have zero models when unreachable (RPC-338/MODEL-004) and are
                // never subject to this filter.
                let cloud_sections =
                    crate::profile_sections::retain_populated_cloud_sections(cloud_sections);
                let custom_sections =
                    crate::profile_sections::retain_populated_cloud_sections(custom_sections);
                // PROV-130: assemble the final DISPLAY order to match TS
                // `modelInitializationService.ts:196-200`
                // `[...profileSections, ...customSections, ...cloudSections]` —
                // local-server profiles FIRST, then custom providers, then the
                // cloud group. Because the first section-with-models is the
                // auto-default, this makes the Rust default match TS.
                //
                // RPC-338: local-server profile sections probe each profile's
                // `/v1/models` endpoint to compute reachability (MODEL-004: a
                // profile with custom models is never marked unreachable).
                // Mirrors TS `loadProfileSections`.
                let mut providers = crate::profile_sections::build_local_profile_sections();
                providers.extend(custom_sections);
                providers.extend(cloud_sections);
                providers
            }
            Err(e) => {
                tracing::error!(
                    target: "handle_impl",
                    error = %e,
                    "list_providers: codelet_providers::custom::list_providers_info failed",
                );
                Vec::new()
            }
        }
    }

    /// PROV-118: delegate to the inherent `SessionManager::set_default_model`,
    /// which ignores empty strings (PROV-101 no-fallback policy preserved).
    fn set_default_model(&self, model: &str) {
        SessionManager::set_default_model(self, model);
    }

    fn set_model(
        &self,
        session_id: &SessionId,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        let session = match self.get_session(&uuid.to_string()) {
            Ok(session) => session,
            Err(_) => return Err(format!("Session not found: {}", session_id.value.as_str())),
        };

        // RPC-343: re-resolve the new model server-side instead of only
        // swapping the cosmetic label strings. Mirror creation-time credential
        // resolution for the (possibly new) provider, apply the selection to
        // the inner request-issuing provider manager via the shared resolver,
        // then recompute and cache the model limits. On any failure we return
        // Err BEFORE mutating session state, so a bad model leaves the previous
        // model and its cached limits intact.
        let model = format!("{provider_id}/{model_id}");
        let project_path = std::path::PathBuf::from(&session.project);
        if let Err(e) =
            crate::credentials::resolve_and_set_env_var(provider_id, Some(project_path.as_path()))
        {
            tracing::error!(
                "set_model: failed to resolve credentials for provider {}: {}",
                provider_id,
                e
            );
        }

        // session.inner is a tokio::sync::Mutex. This trait method is sync and
        // is invoked from the (idle) UI dispatch path, so take the lock without
        // blocking: try_lock succeeds for an idle session and avoids both a
        // sync→async bridge and the risk of deadlocking a streaming turn. If
        // the session is mid-stream the switch is declined (state unchanged).
        let resolved = {
            let mut inner = session
                .inner
                .try_lock()
                .map_err(|_| "Session is busy; cannot switch model right now".to_string())?;
            crate::model_resolution::apply_model_selection(inner.provider_manager_mut(), &model)?
        };

        let compaction_threshold = codelet_cli::compaction_threshold::resolve_compaction_threshold(
            resolved.context_window as u64,
            resolved.max_output_tokens as u64,
            Some(model_id),
            None,
        ) as u32;

        session.set_model(Some(provider_id.to_string()), Some(model_id.to_string()));
        session.set_model_limits(
            resolved.context_window,
            resolved.max_output_tokens,
            compaction_threshold,
        );

        // PROV-123: keep the global default in sync so a NEW session created in
        // this same process inherits the just-selected model (TS-parity with
        // modelSelectionService "keeps store in sync for new sessions").
        // `set_default_model` updates the in-memory `default_model` RwLock that
        // `create_session` / `create_isolated_session` read AND persists
        // `default-model.json` + `tui.lastUsedModel` (PROV-119/122), so it
        // SUPERSEDES the standalone PROV-122 `save_persisted_model_string` call.
        // `model` is the composite `provider_id/model_id` — for profile
        // selections `provider_id` is the profile-qualified key (e.g.
        // `openai:qwen`), so the stored default round-trips to a form a new
        // session resolves identically. Empty/whitespace is ignored inside
        // `set_default_model` (PROV-101 invariant preserved).
        SessionManager::set_default_model(self, &model);
        Ok(())
    }

    // RPC-347: custom-model write surface. These delegate to the RPC-346
    // persistence functions in `profile_sections`, converting the
    // transport-portable `CustomModelDefinition` into the on-disk
    // `CustomModelDef` via `conversions::custom_model_def_from_wire`. The
    // openai-only guard and the missing-profile / empty-array no-op semantics
    // live in `profile_sections`, so these overrides are thin pass-throughs.
    fn add_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        definition: &codelet_rpc_types::CustomModelDefinition,
    ) -> Result<(), String> {
        // TS parity (profile-management.ts saveProfile): profiles are only
        // supported for the OpenAI API provider. Surface an error for a
        // non-openai add rather than silently no-op'ing.
        if provider_id != "openai" {
            return Err(format!(
                "Profiles are only supported for the OpenAI API provider (got '{provider_id}')"
            ));
        }
        let def = crate::conversions::custom_model_def_from_wire(definition);
        crate::profile_sections::save_custom_model(provider_id, profile_name, &def, None)
            .map_err(|e| e.to_string())
    }

    fn update_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        original_model_id: &str,
        definition: &codelet_rpc_types::CustomModelDefinition,
    ) -> Result<(), String> {
        if provider_id != "openai" {
            return Err(format!(
                "Profiles are only supported for the OpenAI API provider (got '{provider_id}')"
            ));
        }
        let def = crate::conversions::custom_model_def_from_wire(definition);
        crate::profile_sections::save_custom_model(
            provider_id,
            profile_name,
            &def,
            Some(original_model_id),
        )
        .map_err(|e| e.to_string())
    }

    fn delete_custom_model(
        &self,
        provider_id: &str,
        profile_name: &str,
        model_id: &str,
    ) -> Result<(), String> {
        crate::profile_sections::delete_custom_model(provider_id, profile_name, model_id)
            .map_err(|e| e.to_string())
    }

    // PROV-108: profile write surface. Delegates to the `profile_persistence`
    // read-modify-write (preserving customModels + sibling keys), converting
    // the transport-portable `ProfileDefinition` via
    // `conversions::profile_def_from_wire`.
    fn save_profile(
        &self,
        provider_id: &str,
        profile_name: &str,
        definition: &codelet_rpc_types::ProfileDefinition,
    ) -> Result<(), String> {
        // TS parity (profile-management.ts saveProfile): profiles are only
        // supported for the OpenAI API provider. The guard predicate and the
        // user-facing message are single-sourced in `profile_persistence` so
        // they cannot drift from the NAPI binding.
        if !crate::profile_persistence::profiles_supported(provider_id) {
            return Err(crate::profile_persistence::profiles_unsupported_error(
                provider_id,
            ));
        }
        let def = crate::conversions::profile_def_from_wire(definition);
        crate::profile_persistence::save_profile(provider_id, profile_name, &def)
            .map_err(|e| e.to_string())
    }

    fn delete_profile(&self, provider_id: &str, profile_name: &str) -> Result<(), String> {
        crate::profile_persistence::delete_profile(provider_id, profile_name)
            .map_err(|e| e.to_string())
    }

    // PROV-136: rename (or in-place update) a profile. Guards on the openai-only
    // predicate (single-sourced in `profile_persistence`) then delegates to the
    // persistence-layer single read-modify-write which preserves customModels
    // and rejects a collision with an existing different profile.
    fn rename_profile(
        &self,
        provider_id: &str,
        old_name: &str,
        new_name: &str,
        definition: &codelet_rpc_types::ProfileDefinition,
    ) -> Result<(), String> {
        if !crate::profile_persistence::profiles_supported(provider_id) {
            return Err(crate::profile_persistence::profiles_unsupported_error(
                provider_id,
            ));
        }
        let def = crate::conversions::profile_def_from_wire(definition);
        crate::profile_persistence::rename_profile(provider_id, old_name, new_name, &def)
            .map_err(|e| e.to_string())
    }

    fn set_thinking_level(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.set_base_thinking_level(level as u8);
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    // CONT-002: auto-continue state — delegates to the BackgroundSession's
    // atomic chrome fields; the agent loop syncs them into the inner
    // session before each dispatched user message.
    fn set_continue_state(
        &self,
        session_id: &SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.set_continue_state(enabled, budget);
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn get_continue_state(&self, session_id: &SessionId) -> (bool, u32) {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .map(|session| session.get_continue_state())
            .unwrap_or((false, 10))
    }

    // CONT-003: goal chrome state — delegates to the BackgroundSession's
    // goal Mutex; the agent loop syncs it into the inner session before
    // each dispatched user message.
    fn set_goal_state(
        &self,
        session_id: &SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                session.set_goal_state(goal);
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    fn get_goal_state(&self, session_id: &SessionId) -> Option<(String, Option<String>)> {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .ok()
            .and_then(|session| session.get_goal_state())
    }

    fn get_role(&self, session_id: &SessionId) -> Option<String> {
        let uuid = uuid_from(session_id);
        self.get_session(&uuid.to_string())
            .ok()
            .and_then(|s| s.get_role())
    }

    fn set_role(&self, session_id: &SessionId, role: Option<String>) -> Result<(), String> {
        let uuid = uuid_from(session_id);
        match self.get_session(&uuid.to_string()) {
            Ok(session) => {
                match role {
                    Some(r) => session.set_role(r),
                    None => session.clear_role(),
                }
                Ok(())
            }
            Err(_) => Err(format!("Session not found: {}", session_id.value.as_str())),
        }
    }

    // ========================================================================
    // RPC-054: Provider credentials surface — delegate to `codelet-providers`.
    // The provider helpers there are already NAPI-free so we can call them
    // directly without dragging any NAPI dependency into codelet-sessions.
    //
    // Scope per the RPC-054 attachment:
    //   * `list_provider_credentials` / `get_provider_credential` reflect
    //     `ProviderCredentials::detect()` + `custom::list_providers_info()`
    //     into the wire-portable `ProviderCredentialInfo` shape.
    //   * `set_provider_credentials` (api_key kind) PERSISTS to
    //     `<data_dir>/credentials/credentials.json` via
    //     `credentials::save_credential` and refreshes the in-memory store
    //     (RPC-054 reopened — mirrors TS saveCredential). oauth / custom
    //     kinds are validated but remain non-persistent (follow-up).
    //   * `delete_provider_credentials` removes the provider entry via
    //     `credentials::delete_credential` (mirrors TS deleteCredential);
    //     absent provider / missing file is a no-op success.
    //   * `test_provider_connection` delegates to
    //     `codelet_providers::custom::test_provider_connection` for custom
    //     providers and falls back to `ProviderManager::with_provider`
    //     (CONFIG-004 pattern) for built-ins.
    //   * `refresh_models_cache` delegates to the same custom-provider
    //     helper because the built-ins have static model lists.
    // ========================================================================

    fn list_provider_credentials(&self) -> Vec<ProviderCredentialInfo> {
        match codelet_providers::custom::list_providers_info() {
            Ok(list) => list
                .into_iter()
                .map(|p| ProviderCredentialInfo {
                    provider_id: p.name.clone(),
                    display_name: p.display_name.unwrap_or_else(|| p.name.clone()),
                    configured: p.available,
                    credential_type: if p.is_custom {
                        "custom".to_string()
                    } else {
                        // PROV-053 / RPC-107: codex, github-copilot, and
                        // anthropic use OAuth flows; everything else uses
                        // an env-var API key. The "anthropic" slug
                        // replaces the legacy Rust-internal "claude" slug
                        // as part of the RPC-107 canonical-catalog port.
                        match p.name.as_str() {
                            "anthropic" | "codex" | "github-copilot" => "oauth".to_string(),
                            _ => "api_key".to_string(),
                        }
                    },
                    model_count: p.models.len() as u32,
                    // RPC-108: propagate masked credential + source
                    // verbatim from codelet-providers. Masking has
                    // already happened server-side via
                    // `credentials::mask_api_key` before this mapper
                    // runs, so raw key bytes never traverse the wire.
                    masked_key: p.masked_key,
                    source: p.source,
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "list_provider_credentials: codelet_providers::custom::list_providers_info failed");
                Vec::new()
            }
        }
    }

    fn get_provider_credential(&self, provider_id: &str) -> Option<ProviderCredentialInfo> {
        self.list_provider_credentials()
            .into_iter()
            .find(|p| p.provider_id == provider_id)
    }

    fn set_provider_credentials(
        &self,
        provider_id: &str,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        // RPC-054 (reopened): the api_key write path is no longer a stub —
        // it persists to <data_dir>/credentials/credentials.json and refreshes
        // the in-memory store, mirroring TS saveCredential. oauth / custom
        // kinds remain non-persistent here (OAuth write path is a follow-up).
        match creds.kind.as_str() {
            "api_key" => {
                let api_key = creds.api_key.as_deref().unwrap_or("");
                if api_key.is_empty() {
                    return Err("api_key input requires a non-empty api_key".to_string());
                }
                crate::credentials::save_credential(provider_id, api_key)?;
                tracing::info!(
                    provider = provider_id,
                    kind = "api_key",
                    "set_provider_credentials: api key persisted to credentials.json"
                );
                Ok(())
            }
            "oauth" => {
                if creds.oauth_token.as_deref().unwrap_or("").is_empty() {
                    return Err("oauth input requires a non-empty oauth_token".to_string());
                }
                tracing::info!(
                    provider = provider_id,
                    kind = "oauth",
                    "set_provider_credentials: oauth persistence is a follow-up; input accepted"
                );
                Ok(())
            }
            "custom" => {
                if creds.custom_endpoint.as_deref().unwrap_or("").is_empty() {
                    return Err("custom input requires a non-empty custom_endpoint".to_string());
                }
                tracing::info!(
                    provider = provider_id,
                    kind = "custom",
                    "set_provider_credentials: custom persistence is a follow-up; input accepted"
                );
                Ok(())
            }
            other => Err(format!("unknown credential kind: {other}")),
        }
    }

    fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String> {
        // PROV-133: pressing 'd' in provider settings must actually remove the
        // credential across EVERY source the availability projection reads.
        // `ProviderCredentials::detect()` derives `configured` from ENV VARS +
        // OAuth AUTH FILES (never credentials.json), so an authoritative delete
        // (Option A) must clear all three sources for the target provider.
        //
        // 1. credentials.json entry (absent provider / missing file = no-op).
        crate::credentials::delete_credential(provider_id)?;

        // 2. Process env vars for ONLY this provider (env source).
        crate::credentials::remove_provider_env_vars(provider_id);

        // 3. OAuth auth file for the three OAuth providers (auth-file source).
        //    Sync file removals so this sync bridge never needs a tokio runtime;
        //    a missing file stays a no-op success. Copilot uses the sync twin
        //    `delete_copilot_auth_sync` for the same reason.
        match provider_id {
            "anthropic" => {
                codelet_providers::claude_auth::delete_claude_auth()
                    .map_err(|e| format!("failed to delete claude auth file: {e}"))?;
            }
            "codex" => {
                codelet_providers::codex::codex_auth::delete_codex_auth()
                    .map_err(|e| format!("failed to delete codex auth file: {e}"))?;
            }
            "github-copilot" => {
                codelet_providers::copilot::delete_copilot_auth_sync()
                    .map_err(|e| format!("failed to delete copilot auth file: {e}"))?;
            }
            _ => {}
        }

        tracing::info!(
            provider = provider_id,
            "delete_provider_credentials: credential removed from credentials.json, env vars, and auth file"
        );
        Ok(())
    }

    fn test_provider_connection(&self, provider_id: &str) -> Result<TestConnectionResult, String> {
        let start = std::time::Instant::now();
        // Try the custom-provider HTTP probe first — it's the only path
        // that returns rich `ProviderTestResult` metadata. If the
        // provider id is unknown to the custom-provider helper, fall
        // back to `ProviderManager::with_provider` which validates
        // credentials for the built-ins.
        //
        // RPC-070: pre-fix this method built its own `tokio::runtime`
        // via `Handle::try_current()` + `runtime.block_on(...)`, which
        // panics under a nested runtime context (the live tarpc
        // dispatcher). Replace with the canonical
        // `block_in_place(|| Handle::current().block_on(...))` pattern
        // so this bridge is safe from inside any multi-thread tokio
        // worker.
        let provider_id_owned = provider_id.to_string();
        let custom_result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                codelet_providers::custom::test_provider_connection(&provider_id_owned).await
            })
        });
        match custom_result {
            Ok(probe) => Ok(TestConnectionResult {
                success: probe.reachable,
                error: if probe.reachable {
                    None
                } else {
                    Some(format!("unreachable (status {:?})", probe.status_code))
                },
                latency_ms: start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
            }),
            Err(_) => {
                // Fall back to credential validation for built-in providers.
                match codelet_providers::ProviderManager::with_provider(provider_id) {
                    Ok(_) => Ok(TestConnectionResult {
                        success: true,
                        error: None,
                        latency_ms: start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
                    }),
                    Err(e) => Ok(TestConnectionResult {
                        success: false,
                        error: Some(e.to_string()),
                        latency_ms: start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
                    }),
                }
            }
        }
    }

    fn refresh_models_cache(&self, provider_id: &str) -> Result<Vec<ModelEntry>, String> {
        // Built-in providers don't publish a dynamic model catalog; we
        // re-list the providers and return the matching model entries.
        // Custom providers DO have dynamic catalogs but lifting the
        // OpenAI-compatible `/models` round-trip into a typed
        // `ModelEntry` list is a follow-up — for now return the
        // statically-declared list.
        let provider_info = codelet_providers::custom::list_providers_info()
            .map_err(|e| format!("list_providers_info failed: {e}"))?
            .into_iter()
            .find(|p| p.name == provider_id);
        Ok(provider_info
            .map(|p| {
                p.models
                    .into_iter()
                    .map(|m| ModelEntry {
                        id: m.id,
                        display_name: String::new(),
                        context_window: m.context_window as u32,
                        supports_reasoning: m.supports_thinking,
                        supports_vision: m.supports_vision,
                        is_custom: true,
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    // ========================================================================
    // RPC-056: Blocklist surface.
    //
    // Re-loads the system + project blocklist configs SEPARATELY (rather
    // than via `load_blocklist_config`, which merges and loses the
    // per-rule provenance) so the wire snapshot can stamp each rule with
    // its `source` tag ("system" | "project"). The project root is
    // derived from `std::env::current_dir()` — matching the
    // `create_session` pattern in this file.
    //
    // System and project configs each fall back to an empty
    // `BlocklistConfig` when the file is absent or fails to parse, so
    // missing configs surface as an empty list rather than an error.
    // The TS frontend takes the same fall-back path via
    // `blocklistLoad(cwd)`.
    // ========================================================================

    fn blocklist_list(&self) -> Vec<BlocklistRuleInfo> {
        let mut out: Vec<BlocklistRuleInfo> = Vec::new();

        // System config (~/.fspec/blocklist.json)
        if let Some(sys_path) = codelet_tools::blocklist::system_config_path() {
            if sys_path.exists() {
                match codelet_tools::blocklist::BlocklistConfig::load_from_file(&sys_path) {
                    Ok(cfg) => {
                        for rule in cfg.rules {
                            out.push(blocklist_rule_to_wire(rule, "system"));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            path = ?sys_path,
                            "blocklist_list: failed to load system blocklist config"
                        );
                    }
                }
            }
        }

        // Project config (<cwd>/.fspec/blocklist.json)
        let project_root = std::env::current_dir().ok();
        if let Some(root) = project_root {
            let project_path = codelet_tools::blocklist::project_config_path(&root);
            if project_path.exists() {
                match codelet_tools::blocklist::BlocklistConfig::load_from_file(&project_path) {
                    Ok(cfg) => {
                        for rule in cfg.rules {
                            out.push(blocklist_rule_to_wire(rule, "project"));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            path = ?project_path,
                            "blocklist_list: failed to load project blocklist config"
                        );
                    }
                }
            }
        }

        out
    }

    // ========================================================================
    // RPC-057: Merge/worktree surface — delegates to `codelet-git`.
    // The repo_path is resolved at call time via std::env::current_dir(),
    // matching the blocklist_list pattern above. MergeStrategy is accepted
    // on the trait surface for future evolution but the underlying
    // codelet-git layer uses a single fast-forward-style algorithm
    // (parity with the current TS sessionMergeChanges).
    // ========================================================================

    fn merge_session_worktree(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        // The strategy is reserved for future evolution — the codelet-git
        // layer currently has only one merge algorithm.
        let _ = strategy;
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        match codelet_git::merge_session(&repo_path, &session_id.value) {
            Ok(result) => {
                let total_changed = result.files_modified.len()
                    + result.files_added.len()
                    + result.files_deleted.len();
                let status = if total_changed == 0 {
                    MergeStatus::NoChanges
                } else {
                    MergeStatus::Success
                };
                Ok(MergeOutcome {
                    status,
                    conflicts: Vec::new(),
                    // codelet_git::merge_session does not surface the
                    // resulting commit SHA today — None until it does.
                    merge_commit: None,
                })
            }
            Err(codelet_git::GitError::ConflictError { files }) => Ok(MergeOutcome {
                status: MergeStatus::Conflict,
                conflicts: files,
                merge_commit: None,
            }),
            Err(e) => Err(format!("{e}")),
        }
    }

    fn discard_session_worktree(&self, session_id: &SessionId) -> Result<(), String> {
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        codelet_git::discard_session(&repo_path, &session_id.value)
            .map(|_| ())
            .map_err(|e| format!("{e}"))
    }

    fn prune_orphaned_worktrees(&self) -> Result<Vec<String>, String> {
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        // The active-session set is sourced from the live SessionManager.
        let active: std::collections::HashSet<String> =
            self.list_sessions().into_iter().map(|s| s.id).collect();
        codelet_git::prune_orphaned(&repo_path, &active)
            .map(|r| r.pruned)
            .map_err(|e| format!("{e}"))
    }

    fn list_session_worktrees(&self) -> Vec<SessionWorktreeInfo> {
        let repo_path = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let worktrees = match codelet_git::list_worktrees(&repo_path) {
            Ok(w) => w,
            Err(_) => return Vec::new(),
        };
        worktrees
            .into_iter()
            .map(|w| {
                // Dirty heuristic: a non-empty session diff means uncommitted
                // changes are present in the worktree.
                let dirty = codelet_git::get_session_diff(&repo_path, &w.session_id)
                    .map(|r| {
                        !r.files_changed.is_empty()
                            || !r.files_added.is_empty()
                            || !r.files_deleted.is_empty()
                    })
                    .unwrap_or(false);
                // base_commit is the session's base; falls back to the
                // worktree HEAD when the session_result is unavailable.
                let base_commit = codelet_git::get_session_diff(&repo_path, &w.session_id)
                    .map(|r| r.base_commit)
                    .unwrap_or_else(|_| w.head_commit.clone());
                SessionWorktreeInfo {
                    session_id: SessionId::new(w.session_id),
                    worktree_path: w.path.to_string_lossy().to_string(),
                    base_commit,
                    head_commit: w.head_commit,
                    dirty,
                }
            })
            .collect()
    }

    fn inspect_session_changes(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionChangesSummary, String> {
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        let result = codelet_git::inspect_session(&repo_path, &session_id.value)
            .map_err(|e| format!("{e}"))?;
        let files_changed = (result.files_changed.len()
            + result.files_added.len()
            + result.files_deleted.len()) as u32;
        // Naively derive insertions/deletions from the unified diff —
        // counts `^+`/`^-` lines while skipping the `+++`/`---` headers.
        let mut insertions: u32 = 0;
        let mut deletions: u32 = 0;
        for line in result.diff.lines() {
            if line.starts_with("+++") || line.starts_with("---") {
                continue;
            }
            if line.starts_with('+') {
                insertions = insertions.saturating_add(1);
            } else if line.starts_with('-') {
                deletions = deletions.saturating_add(1);
            }
        }
        Ok(SessionChangesSummary {
            files_changed,
            insertions,
            deletions,
            // codelet-git does not yet surface a session commit log;
            // leave empty until it does.
            commits: Vec::new(),
        })
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule. Each method resolves the repo_path at call
    // time via `std::env::current_dir()` (matching the blocklist_list
    // pattern in RPC-056) and delegates to
    // `codelet_core::scheduler::crud` — the lifted, NAPI-free
    // CRUD helpers introduced by RPC-058.
    // ─────────────────────────────────────────────────────────────────

    fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob, String> {
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        codelet_core::scheduler::crud::schedule_add(&repo_path.to_string_lossy(), job)
    }

    fn schedule_list(&self) -> Vec<ScheduledJob> {
        let Ok(repo_path) = std::env::current_dir() else {
            return Vec::new();
        };
        codelet_core::scheduler::crud::schedule_list(&repo_path.to_string_lossy())
            .unwrap_or_default()
    }

    fn schedule_pause(&self, name: &str) -> Result<ScheduledJob, String> {
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        codelet_core::scheduler::crud::schedule_pause(&repo_path.to_string_lossy(), name)
    }

    fn schedule_resume(&self, name: &str) -> Result<ScheduledJob, String> {
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        codelet_core::scheduler::crud::schedule_resume(&repo_path.to_string_lossy(), name)
    }

    fn schedule_remove(&self, name: &str) -> Result<(), String> {
        let repo_path = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
        codelet_core::scheduler::crud::schedule_remove(&repo_path.to_string_lossy(), name)
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop. Each method delegates to the shared
    // `codelet_core::loops::LoopStore` singleton via a sync→async
    // bridge so callers MUST be on a thread with an active tokio
    // runtime.
    // ─────────────────────────────────────────────────────────────────

    fn loop_add(
        &self,
        session_id: &SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        let session = self
            .get_session(session_id.value.as_str())
            .map_err(|e| format!("Session not found: {e}"))?;
        let session_uuid = session.id;

        // Construct the entry with a fresh 8-char hex id derived from a
        // UUID v4 (avoids pulling in `rand` since `uuid` is already in
        // the workspace dependency tree).
        let id = generate_loop_id();
        let created_at = Utc::now();
        let expires_at = created_at + Duration::days(3);
        let entry = codelet_core::loops::LoopEntry {
            id,
            session_id: session_uuid,
            prompt,
            interval_seconds,
            created_at,
            expires_at,
            last_run_at: None,
        };

        // Capture the BackgroundSession by Arc for both callbacks.
        let session_for_fire = Arc::clone(&session);
        let on_fire: Arc<dyn Fn(String) + Send + Sync + 'static> =
            Arc::new(move |prompt_text: String| {
                let _ = session_for_fire.send_input(prompt_text, None);
            });

        let session_for_idle = Arc::clone(&session);
        let idle_check: codelet_core::loops::IdleCheckFn = Arc::new(move |_session_id: Uuid| {
            let session = Arc::clone(&session_for_idle);
            Box::pin(async move { session.get_status() == SessionStatus::Idle })
        });

        let entry_for_store = entry.clone();
        loop_block_on(async move {
            codelet_core::loops::LoopStore::instance()
                .try_register_with_task_and_idle_check(entry_for_store, on_fire, idle_check)
                .await
        })?;

        Ok(entry_to_wire(&entry))
    }

    fn loop_cancel(&self, id: &str) -> Result<bool, String> {
        let id_owned = id.to_string();
        let removed = loop_block_on(async move {
            codelet_core::loops::LoopStore::instance()
                .cancel(&id_owned)
                .await
        });
        Ok(removed)
    }

    fn loop_list(&self, session_id: &SessionId) -> Vec<RegisteredLoop> {
        let Ok(session) = self.get_session(session_id.value.as_str()) else {
            return Vec::new();
        };
        let session_uuid = session.id;
        let entries = loop_block_on(async move {
            codelet_core::loops::LoopStore::instance()
                .list_for_session(session_uuid)
                .await
        });
        entries.iter().map(entry_to_wire).collect()
    }
}

/// RPC-059: shared sync→async bridge for every `/loop` handle method.
/// Centralising the `Handle::current().block_on(...)` call here keeps
/// the impl block's bridge-count small even though three trait methods
/// have to cross the boundary.
///
/// **RPC-070:** the inner `block_on` is wrapped in
/// `tokio::task::block_in_place(...)` so this bridge is legal from
/// inside a tokio worker that is currently driving an async future
/// (e.g. the live tarpc handler at
/// `codelet/rpc/src/lib.rs::FspecServiceImpl::create_session`). Without
/// the wrapper, calling `block_on` from inside an executing future
/// panics with "Cannot start a runtime from within a runtime".
/// `block_in_place` temporarily detaches the worker from the
/// multi-thread scheduler — which is why this helper requires a
/// multi-thread runtime (asserted below).
fn loop_block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    debug_assert_eq!(
        tokio::runtime::Handle::current().runtime_flavor(),
        tokio::runtime::RuntimeFlavor::MultiThread,
        "loop_block_on requires a multi-thread tokio runtime — \
         tokio::task::block_in_place panics on a single-thread runtime",
    );
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

/// RPC-059: convert an internal `LoopEntry` (chrono-typed) into the
/// wire-portable `RegisteredLoop` (RFC-3339 String timestamps).
fn entry_to_wire(entry: &codelet_core::loops::LoopEntry) -> RegisteredLoop {
    RegisteredLoop {
        id: entry.id.clone(),
        session_id: SessionId::new(entry.session_id.to_string()),
        prompt: entry.prompt.clone(),
        interval_seconds: entry.interval_seconds,
        created_at: entry.created_at.to_rfc3339(),
        expires_at: entry.expires_at.to_rfc3339(),
        last_run_at: entry.last_run_at.map(|t| t.to_rfc3339()),
    }
}

/// RPC-059: generate a fresh 8-char lowercase-hex loop id by taking the
/// first 8 chars of a UUID v4's simple representation. Avoids pulling
/// in the `rand` crate since `uuid` is already in the workspace
/// dependency tree.
fn generate_loop_id() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

/// Convert a `codelet_tools::blocklist::BlocklistRule` to the wire-portable
/// `BlocklistRuleInfo`, stamping the supplied `source` tag.
fn blocklist_rule_to_wire(
    rule: codelet_tools::blocklist::BlocklistRule,
    source: &str,
) -> BlocklistRuleInfo {
    let action = match rule.action {
        codelet_tools::blocklist::BlocklistAction::Block => "block",
        codelet_tools::blocklist::BlocklistAction::Allow => "allow",
        codelet_tools::blocklist::BlocklistAction::Prompt => "prompt",
    };
    BlocklistRuleInfo {
        id: rule.id,
        pattern: rule.pattern,
        action: action.to_string(),
        reason: rule.reason,
        guidance: rule.guidance,
        source: source.to_string(),
    }
}

/// Build the models.dev `ModelRegistry` once for `list_providers` cloud-model
/// population (RPC-073). Returns `None` on any failure (cold cache offline,
/// corrupt file) so the model selector degrades to empty built-in model lists
/// rather than erroring. Bridges the async registry load from the sync trait
/// method via `block_in_place` (the same pattern `build_local_profile_sections`
/// and `test_provider_connection` use), so it is only safe inside a tokio
/// multi-thread runtime — which `list_providers` always runs within.
fn build_cloud_registry() -> Option<codelet_providers::models::ModelRegistry> {
    // RPC-073: bridge the async registry load from the sync trait method.
    // `block_in_place` + `Handle::block_on` requires a live multi-thread
    // tokio runtime. Outside one (e.g. plain `#[test]` callers exercising
    // the no-panic graceful-degradation contract), `Handle::try_current`
    // returns Err — degrade to `None` rather than panicking.
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return None;
    };
    tokio::task::block_in_place(|| {
        handle.block_on(async {
            match codelet_providers::models::ModelCache::new() {
                Ok(cache) => codelet_providers::models::ModelRegistry::new(&cache)
                    .await
                    .ok(),
                Err(_) => None,
            }
        })
    })
}
