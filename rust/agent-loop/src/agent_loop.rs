//! Background-session agent loop (RPC-043).
//!
//! Extracted from `rust/napi/src/session_manager.rs` lines 1216-2917
//! by RPC-043 as part of the codelet-napi thin-adapter split. This
//! module owns:
//!
//! - the `run_with_provider!` dispatch macro,
//! - `agent_loop_dispatch_supports_provider` predicate + dispatch tests,
//! - the `InputWithImages` helper struct,
//! - `pub async fn agent_loop` (the napi-side per-session task),
//! - `BackgroundOutput` + `BackgroundProgressEmitter` (the two
//!   [`codelet_cli::interactive::StreamOutput`] sinks that bridge the
//!   `codelet-cli` streaming pipeline into the napi-side persistence and
//!   chunk-fanout machinery).
//!
//! Callers:
//! - [`crate::session_hooks::NapiSessionManagerHooks::spawn_agent_loop`]
//!   tokio-spawns the `agent_loop` future every time the
//!   `codelet-sessions` `SessionManager` creates a session.
//! - Inside napi only — agent_loop is `pub(crate)` and never leaves the
//!   crate.

#![allow(clippy::too_many_arguments)]

use codelet_rpc_types::{NotificationSeverity, StreamChunk};

// HOOK-013: Agent lifecycle hooks — engine functions live napi-side
// because they bridge the agent_loop's per-turn state into the
// codelet-core hook runner.
use codelet_core::lifecycle_hooks::{
    run_session_end, run_session_start, run_user_prompt, HookMessageLevel,
};
use codelet_tools::tool_pause::{
    set_pause_handler, PauseHandler, PauseRequest, PauseResponse, PauseState,
};
use codelet_tools::McpInjection;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::mpsc;

// RPC-039 / RPC-040: BackgroundSession + companion types live in the
// NAPI-free `codelet-sessions` crate. We import them directly here.
use codelet_rpc_types::SessionStatus;
use codelet_sessions::background_session::{
    format_incoming_message, BackgroundSession, BridgeImageData, IncomingMessage, PromptInput,
};

// RPC-043: persistence helpers live in `crate::persist`.
use crate::persist::{persist_pending_annotations, persist_user_message};

// RPC-043: bridge wiring + handler registration helpers live in
// `crate::bridges`. The agent_loop body calls
// `register_deep_search_handler` / `register_agent_manager_handler` on
// every session creation so the per-session DeepSearch / AgentManager
// closures capture the latest provider/model values.
use crate::bridges::{register_agent_manager_handler, register_deep_search_handler};

// RPC-043: `is_global_chunk_callback_registered` lives next to its
// owning static (`CHUNK_FANOUT_TSFN`) in `crate::session_bindings`.
use crate::is_global_chunk_callback_registered;

// RPC-072: lifted helpers + types live next door in the agent-loop crate.
use crate::background_output::BackgroundOutput;
use crate::dispatch::InputWithImages;
use crate::run_with_provider;

/// Agent loop that runs in background tokio task
/// WATCH-019: Modified to also process supervisor injections via supervisor_input_rx
/// REFAC-007: Persists messages to Rust persistence layer
/// BRIDGE-007: Now supports multimodal input with images from bridge
/// MCP-001: Processes MCP server-initiated messages (notifications + sampling)
pub async fn agent_loop(
    session: Arc<BackgroundSession>,
    mut input_rx: mpsc::Receiver<PromptInput>,
    mut mcp_injection_rx: mpsc::Receiver<McpInjection>,
) {
    // MCP-001-FIX: Track whether the MCP injection channel is still open.
    // Once it returns None (sender dropped by cleanup_mcp_session), we must stop
    // polling it. Without this guard, the closed channel returns None immediately
    // every iteration, causing tokio::select! to resolve instantly → CPU busy-loop.
    let mut mcp_channel_open = true;

    // CMPCT-020: Compaction convergence watchdog state
    let mut compaction_watchdog_attempts: usize = 0;
    let mut compaction_retry_input: Option<String> = None;

    // RIG-014: Loop-abort auto-continue retry counter (prevents infinite
    // cycling if the model keeps degenerating into loops on retry).
    let mut loop_abort_retry_count: usize = 0;
    const RIG014_MAX_LOOP_ABORT_RETRIES: usize = 10;

    // HOOK-013: Fire session_start hooks
    if let Some(ref hooks) = session.lifecycle_hooks {
        let ctx = session.hook_context();
        let outcome = run_session_start(hooks, &ctx, "startup").await;
        // Inject additional context as system-reminder messages
        if !outcome.additional_context.is_empty() {
            let mut inner = session.inner.lock().await;
            let combined_context = outcome.additional_context.join("\n");
            inner.add_system_reminder(
                codelet_cli::session::SystemReminderType::FspecWorkflow,
                &combined_context,
            );
            drop(inner);
        }
        for msg in &outcome.messages {
            if msg.level == HookMessageLevel::Warning || msg.level == HookMessageLevel::Error {
                tracing::warn!("[HOOK-013] session_start hook: {}", msg.content);
                session.handle_output(StreamChunk::user_notification(
                    format!("Hook: {}", msg.content),
                    NotificationSeverity::Warning,
                ));
            }
        }
    }

    loop {
        // CMPCT-020: Check for compaction watchdog retry input before waiting for user input
        // RIG-014: `is_retry_input` distinguishes synthetic retry inputs (compaction
        // watchdog / loop-abort auto-continue) from real user input so the
        // loop-abort retry counter is only reset on genuine user turns.
        let (input_to_process, is_retry_input): (Option<InputWithImages>, bool) =
            if let Some(retry_text) = compaction_retry_input.take() {
                tracing::info!("[AGENT-LOOP] Compaction watchdog: retrying with escalation input");
                (
                    Some(InputWithImages {
                        text: retry_text,
                        thinking_config: None,
                        images: None,
                    }),
                    true,
                )
            } else {
            // WATCH-019: Use tokio::select! to wait on both user input and supervisor input
            // Lock the supervisor_input_rx to use in select
            let mut supervisor_rx = session.incoming_message_rx.lock().await;

            // Use biased to prefer user input over supervisor/MCP input
            // BRIDGE-007: Changed to InputWithImages to support multimodal content
            let input_to_process_inner: Option<InputWithImages> = tokio::select! {
                biased;

                // User input takes priority
                result = input_rx.recv() => {
                    match result {
                        Some(prompt_input) => Some(InputWithImages {
                            text: prompt_input.input,
                            thinking_config: prompt_input.thinking_config,
                            images: None, // Regular user input doesn't have images (yet)
                        }),
                        None => {
                            // Channel closed, exit loop
                            drop(supervisor_rx);
                            // HOOK-013: Fire session_end hooks before exiting
                            if let Some(ref hooks) = session.lifecycle_hooks {
                                let ctx = session.hook_context();
                                let _outcome = run_session_end(hooks, &ctx, "exit").await;
                            }
                            break;
                        }
                    }
                }

                // WATCH-019: Supervisor injection input
                result = supervisor_rx.recv() => {
                    match result {
                        Some(supervisor_input) => {
                            // FIX-6: Decrement pending counter when message is consumed
                            session.incoming_message_pending.fetch_sub(1, Ordering::Release);
                            tracing::debug!("agent_loop received supervisor input from {}: {}", supervisor_input.role_name, supervisor_input.message.chars().take(50).collect::<String>());
                            // Format supervisor input as a user message with structured prefix
                            let formatted = format_incoming_message(&supervisor_input);

                            // BRIDGE-007: Emit the supervisor input chunk with images if present
                            if let Some(ref images) = supervisor_input.images {
                                let supervisor_images: Vec<codelet_rpc_types::IncomingMessageImage> = images.iter()
                                    .map(|img| codelet_rpc_types::IncomingMessageImage {
                                        data: img.data.clone(),
                                        media_type: img.media_type.clone(),
                                    })
                                    .collect();
                                session.handle_output(StreamChunk::incoming_message_with_images(formatted.clone(), supervisor_images));
                            } else {
                                session.handle_output(StreamChunk::incoming_message(formatted.clone()));
                            }

                            // BRIDGE-007: Pass images to LLM as multimodal input
                            Some(InputWithImages {
                                text: formatted,
                                thinking_config: None,
                                images: supervisor_input.images,
                            })
                        }
                        None => {
                            // Supervisor channel closed, continue with user input only
                            None
                        }
                    }
                }

                // MCP-001: Server-initiated MCP messages (notifications, sampling requests)
                // MCP-001-FIX: Only poll when channel is open to prevent busy-loop spin
                result = mcp_injection_rx.recv(), if mcp_channel_open => {
                    match result {
                        Some(McpInjection::Notification(text)) => {
                            tracing::info!("[MCP] agent_loop received notification: {}", text.chars().take(80).collect::<String>());
                            // Emit as supervisor input chunk so the UI shows it
                            session.handle_output(StreamChunk::incoming_message(text.clone()));
                            // Process as LLM input so the agent can react to the notification
                            Some(InputWithImages {
                                text,
                                thinking_config: None,
                                images: None,
                            })
                        }
                        Some(McpInjection::SamplingRequest { params, response_tx }) => {
                            tracing::info!(
                                "[MCP] agent_loop received sampling/createMessage request ({} messages, maxTokens={})",
                                params.messages.len(),
                                params.max_tokens,
                            );
                            // Format sampling messages as a prompt for the LLM.
                            // The agent processes the prompt normally, and we capture its
                            // response text from the output handler to send back via response_tx.
                            //
                            // For V1: We cannot easily capture the full response text from
                            // run_agent_stream because it streams through BackgroundOutput.
                            // Instead, we return an error to the MCP server. The server will
                            // receive a structured error and can retry or fall back.
                            //
                            // TODO(MCP-001 V2): To support sampling properly:
                            //   1. Run a dedicated LLM call with the sampling messages
                            //   2. Capture the full response text
                            //   3. Send CreateMessageResult through response_tx
                            let _ = response_tx.send(Err(
                                "sampling/createMessage not yet supported — V2 feature".to_string(),
                            ));
                            tracing::debug!("[MCP] sampling/createMessage rejected (V2 feature)");
                            None // Don't process as agent input
                        }
                        None => {
                            // MCP-001-FIX: Channel closed (sender dropped by cleanup_mcp_session).
                            // Disable this select! branch to prevent busy-loop. The closed receiver
                            // would return None immediately on every poll, causing the select! to
                            // resolve instantly and spin the CPU.
                            tracing::info!("[MCP] injection channel closed for session {}", session.id);
                            mcp_channel_open = false;
                            None
                        }
                    }
                }
            };

            // Drop the lock before processing to avoid holding it during agent execution
            drop(supervisor_rx);

            (input_to_process_inner, false)
        }; // end CMPCT-020 if/else

        // RIG-014: Reset the loop-abort retry counter on real user input
        // (not on synthetic retry inputs from the watchdog or loop-abort).
        if !is_retry_input {
            loop_abort_retry_count = 0;
        }

        // If we got input to process, run the agent
        // BRIDGE-007: Changed to InputWithImages to support multimodal content
        if let Some(input_with_images) = input_to_process {
            let input = &input_with_images.text;

            tracing::debug!(
                "Session {} processing input: {}",
                session.id,
                input.chars().take(50).collect::<String>()
            );

            // BRIDGE-007: Log if images are present
            if let Some(ref images) = input_with_images.images {
                tracing::debug!(
                    "Session {} has {} image(s) attached",
                    session.id,
                    images.len()
                );
            }

            // HOOK-013: Run user_prompt_submit hooks (can block the prompt)
            if let Some(ref hooks) = session.lifecycle_hooks {
                if !hooks.user_prompt_submit.is_empty() {
                    let ctx = session.hook_context();
                    let outcome = run_user_prompt(hooks, &ctx, input).await;
                    // Surface hook warnings/errors
                    for msg in &outcome.messages {
                        if msg.level == HookMessageLevel::Warning
                            || msg.level == HookMessageLevel::Error
                        {
                            tracing::warn!("[HOOK-013] user_prompt_submit hook: {}", msg.content);
                        }
                    }
                    if !outcome.allow_prompt {
                        let reason = outcome
                            .block_reason
                            .unwrap_or_else(|| "Blocked by hook".to_string());
                        tracing::warn!("[HOOK-013] Prompt blocked: {}", reason);
                        session.handle_output(StreamChunk::user_notification(
                            format!("Prompt blocked: {}", reason),
                            NotificationSeverity::Warning,
                        ));
                        session.set_status(SessionStatus::Idle);
                        session.handle_output(StreamChunk::done());
                        continue; // Skip this prompt, go back to waiting for input
                    }
                    // Inject additional context from the hook
                    if !outcome.additional_context.is_empty() {
                        let mut inner_session = session.inner.lock().await;
                        let combined_context = outcome.additional_context.join("\n");
                        inner_session.add_system_reminder(
                            codelet_cli::session::SystemReminderType::FspecWorkflow,
                            &combined_context,
                        );
                        drop(inner_session);
                    }
                }
            }

            // REFAC-007: Persist user message to Rust persistence layer
            // This replaces TypeScript's persistenceStoreMessageEnvelope call
            if let Err(e) = persist_user_message(&session.id, input) {
                tracing::error!(
                    "Failed to persist user message for session {}: {}",
                    session.id,
                    e
                );
                // Continue processing even if persistence fails - don't block agent execution
            }

            // Set status to running
            session.set_status(SessionStatus::Running);
            session.reset_interrupt();

            // Get provider name and model ID early (needed for thinking config)
            // Lock briefly, then release before the heavy processing
            // PROV-005: We need both provider AND model to correctly determine thinking config.
            // Adaptive thinking models (claude-opus-4-6, claude-sonnet-4-6) need the model name,
            // not just the provider name, to trigger adaptive thinking in get_thinking_config().
            let (current_provider, current_model, thinking_model_key) =
                {
                    let inner = session.inner.lock().await;
                    // MODEL-004: Check facade_override first — if set, dispatch to that
                    // provider instead of the current_provider. This allows custom models
                    // to route API calls through a different provider backend.
                    let provider = inner
                        .provider_manager()
                        .facade_override()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| inner.current_provider_name().to_string());
                    let model = inner.current_model_id().map(|s| s.to_string());

                    // PROV-100: Compute a parallel "thinking model key" that
                    // resolves a custom provider's model alias (e.g.
                    // `"opus-4.7"`, the HashMap key in `ProviderConfig.models`)
                    // to its upstream model identifier (e.g.
                    // `"claude-opus-4-7"`, the `ModelDef.id` field).
                    // `is_adaptive_thinking_model` and
                    // `get_thinking_config` key on the upstream id, so
                    // without this the custom provider never sees a
                    // non-empty thinking config and Opus 4.7 can't enter
                    // adaptive thinking. We intentionally keep
                    // `current_model` as the raw alias because the
                    // CustomProvider dispatch further down uses it as the
                    // `model_alias` argument and would fail the config
                    // lookup if we substituted the resolved id.
                    let real_provider_name = inner.current_provider_name().to_string();
                    let thinking_key = if matches!(
                        inner.provider_manager().current_provider_type(),
                        codelet_providers::ProviderType::Custom(_)
                    ) {
                        model.as_deref().and_then(|alias| {
                            codelet_providers::custom::resolve_custom_model_id(
                                &real_provider_name,
                                alias,
                            )
                        })
                    } else {
                        None
                    };

                    tracing::debug!(
                    "[AGENT-LOOP] current_provider={}, current_model={:?}, thinking_model_key={:?}",
                    provider, model, thinking_key
                );
                    (provider, model, thinking_key)
                };

            // BRIDGE-006: Unified thinking level detection
            // Single source of truth - same logic for TUI, Bridge, and Supervisor input.
            // This replaces the old approach where TypeScript passed thinking_config
            // only for TUI input (supervisor/bridge was hardcoded to None).
            //
            // Priority (PROV-005 fix):
            // 1. ALWAYS use model-aware config for adaptive thinking models (Opus 4.6, Sonnet 4.6)
            //    This overrides any TypeScript-provided config to prevent budget_tokens errors
            // 2. Otherwise, if TypeScript passed an explicit thinking_config, use it (backwards compat)
            // 3. Otherwise, detect from message text + session base level
            let thinking_config_value: Option<serde_json::Value> = {
                use crate::thinking_config::{get_thinking_config, JsThinkingLevel};
                use crate::thinking_level_detection::{
                    compute_effective_thinking_level, detect_thinking_level, has_disable_keywords,
                    thinking_level_from_u8,
                };
                use codelet_tools::facade::is_adaptive_thinking_model;

                // PROV-100: Prefer the resolved "thinking model key"
                // for custom providers (e.g. `"claude-opus-4-7"` rather
                // than the config alias `"opus-4.7"`). This is the key
                // that `is_adaptive_thinking_model` / `get_thinking_config`
                // recognise. For built-in providers
                // `thinking_model_key` is None and we fall back to
                // `current_model` verbatim, so no behaviour changes for
                // claude/openai/gemini/zai/codex/copilot.
                let routing_model = thinking_model_key.as_deref().or(current_model.as_deref());

                // PROV-005 FIX: For adaptive thinking models, ALWAYS use model-aware config
                // regardless of what TypeScript passed. This prevents the bug where TypeScript
                // calls getThinkingConfig('claude', level) and gets budgeted thinking, which
                // Opus 4.6 rejects with "max_tokens must be greater than thinking.budget_tokens".
                let is_adaptive_model = routing_model
                    .map(is_adaptive_thinking_model)
                    .unwrap_or(false);

                tracing::debug!(
                    "[AGENT-LOOP] thinking routing: routing_model={:?}, is_adaptive={}, base_thinking_level={}, has_ts_config={}",
                    routing_model,
                    is_adaptive_model,
                    session.get_base_thinking_level(),
                    input_with_images.thinking_config.is_some(),
                );

                if is_adaptive_model {
                    // Adaptive models: detect level and use model-aware config
                    let detected_level = detect_thinking_level(input);
                    let force_off = has_disable_keywords(input);
                    let base_level = thinking_level_from_u8(session.get_base_thinking_level());
                    let effective_level =
                        compute_effective_thinking_level(base_level, detected_level, force_off);
                    tracing::debug!(
                        "[AGENT-LOOP] adaptive path: base={:?}, detected={:?}, force_off={}, effective={:?}",
                        base_level, detected_level, force_off, effective_level
                    );

                    if effective_level == JsThinkingLevel::Off {
                        None
                    } else {
                        // Use the actual model name for adaptive config
                        // Safety: is_adaptive_model is true only when routing_model is Some
                        let config_key = routing_model
                            .expect("routing_model must be Some when is_adaptive_model is true");
                        match get_thinking_config(config_key.to_string(), effective_level) {
                            Ok(config_str) => {
                                tracing::info!(
                                    "[AGENT-LOOP] adaptive thinking selected: config_key={}, effective_level={:?}, base={:?}, detected={:?}, force_off={}, config_str={}",
                                    config_key, effective_level, base_level, detected_level, force_off, config_str
                                );
                                serde_json::from_str(&config_str).ok()
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to get thinking config for adaptive model: {}",
                                    e
                                );
                                None
                            }
                        }
                    }
                } else if let Some(config_str) = input_with_images.thinking_config.as_deref() {
                    // Non-adaptive: use TypeScript-provided config (for backwards compatibility)
                    serde_json::from_str(config_str).ok()
                } else {
                    // Unified detection: detect level from message text
                    let detected_level = detect_thinking_level(input);
                    let force_off = has_disable_keywords(input);
                    let base_level = thinking_level_from_u8(session.get_base_thinking_level());
                    let effective_level =
                        compute_effective_thinking_level(base_level, detected_level, force_off);

                    if effective_level == JsThinkingLevel::Off {
                        None
                    } else {
                        // PROV-005: Get thinking config using model name (if available) for model-aware config.
                        // For Claude 4.6 models, this triggers adaptive thinking instead of budgeted.
                        // Falls back to provider name for providers that don't have model-specific configs.
                        let config_key = routing_model.unwrap_or(&current_provider);
                        match get_thinking_config(config_key.to_string(), effective_level) {
                            Ok(config_str) => {
                                tracing::info!("Thinking level detected: {:?} (base={:?}, detected={:?}, force_off={}, config_key={})",
                                    effective_level, base_level, detected_level, force_off, config_key);
                                serde_json::from_str(&config_str).ok()
                            }
                            Err(e) => {
                                tracing::warn!("Failed to get thinking config: {}", e);
                                None
                            }
                        }
                    }
                }
            };

            // Re-acquire lock for the rest of processing
            let mut inner_session = session.inner.lock().await;

            // CONT-002/CONT-003 (CONT-009): this is a real dispatched user
            // message — run the shared completion-contract sync (inner
            // continue/goal sync, per-turn nudge reset, done() registry
            // arm + goal) immediately before create_rig_agent so done() is
            // registered only while armed. The block lives on
            // BackgroundSession so both agent-loop twins share one copy.
            session.sync_completion_contract_for_user_turn(&mut inner_session);

            // REFAC-007: Create output handler with provider for message persistence
            let session_for_output = session.clone();
            let output =
                BackgroundOutput::with_provider(session_for_output, current_provider.clone());

            // RIG-014: Reset the per-turn loop detectors (defensive — the
            // BackgroundOutput is fresh per turn, but this keeps the
            // contract explicit if the instance is ever reused).
            output.reset_turn_loop_detectors();

            // RIG-014: If the previous turn was aborted by the streaming
            // loop detector, inject the corrective note into the turn
            // context so the model continues with a fresh approach instead
            // of repeating its earlier reasoning.
            if let Some(loop_abort_note) = session.take_pending_loop_abort_note() {
                tracing::info!(
                    "[RIG-014] injecting loop-abort corrective note into turn context (session={})",
                    session.id
                );
                inner_session.messages.push(rig::message::Message::User {
                    content: rig::OneOrMany::one(rig::message::UserContent::text(
                        loop_abort_note,
                    )),
                });
            }

            let session_for_pause = session.clone();
            let pause_handler: PauseHandler = Arc::new(move |request: PauseRequest| {
                let state = PauseState {
                    kind: request.kind,
                    tool_name: request.tool_name.clone(),
                    message: request.message.clone(),
                    details: request.details.clone(),
                };
                session_for_pause.set_pause_state(Some(state));
                session_for_pause.set_status(SessionStatus::Paused);

                let response = session_for_pause.wait_for_pause_response();

                session_for_pause.set_status(SessionStatus::Running);

                response
            });

            set_pause_handler(session.id, Some(pause_handler));

            // CODE-009: Set fspec handler for TypeScript command execution
            // Similar to pause handler - blocks until TypeScript executes and responds
            let session_for_fspec = session.clone();
            let fspec_handler: codelet_tools::FspecHandler =
                std::sync::Arc::new(move |request: codelet_tools::FspecHandlerRequest| {
                    // RPC-041: Check the napi-side TSFN registration via the
                    // helper that consults CHUNK_FANOUT_TSFN.
                    //
                    // TOOL-019 / RPC-327 follow-up: when the chunk callback is
                    // NOT registered (i.e. the standalone fspec Rust binary has
                    // no TypeScript shell), fall through to the in-process Rust
                    // dispatcher in `codelet_fspec_core`. Phase 1 stubs return a
                    // structured `NotYetPorted` / `UnknownCommand` error per
                    // command so the agent loop completes the turn instead of
                    // hanging on a non-existent JS callback.
                    //
                    // We still emit `FspecCommandRequest` + `FspecCommandResult`
                    // chunks here so the TUI / WS bridge subscribers can render
                    // the tool call visually — this matters because the
                    // standalone Rust binary is precisely the host where the
                    // shim returns `false`, yet the TUI is the primary surface
                    // that needs to see "Fspec(list-work-units) → <result>".
                    if !is_global_chunk_callback_registered() {
                        let tool_call_id = uuid::Uuid::new_v4().to_string();
                        let fspec_request = codelet_rpc_types::FspecRequest {
                            command: request.command.clone(),
                            args_json: request.args_json.clone(),
                            project_root: request.project_root.clone(),
                            tool_call_id: tool_call_id.clone(),
                        };
                        session_for_fspec
                            .handle_output(StreamChunk::fspec_command_request(fspec_request));

                        let dispatch_req = codelet_fspec_core::DispatchRequest {
                            command: request.command.clone(),
                            args_json: request.args_json.clone(),
                            project_root: std::path::PathBuf::from(&request.project_root),
                        };
                        let dispatch_result = codelet_fspec_core::dispatch_command(dispatch_req);

                        let fspec_result_chunk = codelet_rpc_types::FspecResult {
                            success: dispatch_result.success,
                            data: dispatch_result.data.clone(),
                            error: dispatch_result.error.clone(),
                            system_reminder: dispatch_result.system_reminder.clone(),
                            tool_call_id,
                        };
                        session_for_fspec
                            .handle_output(StreamChunk::fspec_command_result(fspec_result_chunk));

                        return codelet_tools::FspecHandlerResult {
                            success: dispatch_result.success,
                            data: dispatch_result.data,
                            error: dispatch_result.error,
                            system_reminder: dispatch_result.system_reminder,
                        };
                    }

                    // Generate a unique tool call ID for correlation
                    let tool_call_id = uuid::Uuid::new_v4().to_string();

                    // Emit FspecCommandRequest chunk for TypeScript to process
                    let fspec_request = codelet_rpc_types::FspecRequest {
                        command: request.command.clone(),
                        args_json: request.args_json.clone(),
                        project_root: request.project_root.clone(),
                        tool_call_id: tool_call_id.clone(),
                    };

                    session_for_fspec
                        .handle_output(StreamChunk::fspec_command_request(fspec_request));

                    // Block until TypeScript executes and calls sessionSendFspecResult
                    let fspec_result = session_for_fspec.wait_for_fspec_response();

                    // Emit FspecCommandResult chunk for UI display
                    session_for_fspec
                        .handle_output(StreamChunk::fspec_command_result(fspec_result.clone()));

                    // Convert NAPI FspecResult to tools FspecHandlerResult
                    codelet_tools::FspecHandlerResult {
                        success: fspec_result.success,
                        data: fspec_result.data,
                        error: fspec_result.error,
                        system_reminder: fspec_result.system_reminder,
                    }
                });

            // REFAC-008-FIX: Use per-session handler storage to prevent race conditions
            // when multiple sessions run concurrently.
            codelet_tools::set_fspec_handler_for_session(session.id, Some(fspec_handler));

            // BUG-117: Register HITL handler for request_user_input tool
            // Follows the PAUSE pattern: store request state, set status Paused, block, clear on response
            let session_for_hitl = session.clone();
            let hitl_handler: codelet_tools::request_user_input::HitlHandler = std::sync::Arc::new(
                move |_session_id, request: codelet_tools::request_user_input::HitlRequest| {
                    // Store HITL request in session state for TypeScript to poll
                    session_for_hitl.set_hitl_request(Some(request));

                    // Set session status to Paused (triggers React re-render via SessionStateChange)
                    session_for_hitl.set_status(SessionStatus::Paused);

                    // Block until TypeScript sends response via session_send_hitl_response
                    let response = session_for_hitl.wait_for_hitl_response();

                    // Clear HITL request state and restore Running status
                    session_for_hitl.set_hitl_request(None);
                    session_for_hitl.set_status(SessionStatus::Running);

                    Ok(response)
                },
            );
            codelet_tools::set_hitl_handler(session.id, Some(hitl_handler));

            // AMGR-001: Register SessionSearch handler for this session
            // The handler accesses the persistence layer directly (MessageStore, SessionStore, BlobStore)
            let session_search_handler = crate::session_search_handler::create_handler(
                std::path::PathBuf::from(&session.project),
                session.compaction_in_progress.clone(),
            );
            codelet_tools::set_session_search_handler(session.id, Some(session_search_handler));

            // KGRAPH-024: Register GraphSearch handler (dual-graph architecture — no provider context needed)
            let graph_search_handler = crate::graph_search_handler::create_handler();
            codelet_tools::set_graph_search_handler(session.id, Some(graph_search_handler));

            // RLM-001: Register DeepSearch handler for this session
            // BUG-102: Capture provider and model from current session so the
            // sub-agent inherits the same LLM configuration.
            // MODEL-005: Capture context window and max output from parent session
            // so DeepSearch sub-agents use per-model limits instead of provider constants.
            // Returns a Future (not sync) because the sub-agent makes async LLM API calls
            // BUG-132: Extracted into register_deep_search_handler() so it can be
            // called again after model changes.
            register_deep_search_handler(
                session.id,
                &inner_session,
                std::path::PathBuf::from(&session.project),
            );

            // AMGR-009: Register AgentManager handler for this session
            // The handler accesses SessionManager for spawn/list/get_status/close
            // AMGR-013: Use selected_model_string() which preserves the original
            // "provider/model" registry format (e.g. "anthropic/claude-opus-4-6")
            // instead of current_provider_name() which returns the internal name ("claude").
            // MODEL-005: Capture per-model context window and max output tokens from parent
            // session so subordinate agents inherit per-model limits.
            // BUG-132: Extracted into register_agent_manager_handler() so it can be
            // called again after model changes.
            register_agent_manager_handler(
                session.id,
                &inner_session,
                session.project.clone(),
                session.owning_manager(),
            );

            // AMGR-015: Register async handler for await_idle action
            {
                let async_handler =
                    crate::agent_manager_handler::create_async_handler(session.owning_manager());
                codelet_tools::set_agent_manager_async_handler(session.id, Some(async_handler));
            }

            // Register inject_summary handler — stores DAG in pending_dag_content
            // and fires on_injected to clear the compaction progress spinner.
            // CMPCT-038: on_injected does NOT emit CompactionComplete — only the
            // summary size is known here. The honest chunk (compacted_tokens =
            // recalculated post-injection tracker total) is emitted after the
            // stream by apply_pending_dag_and_emit below.
            {
                let context_window = inner_session.provider_manager().context_window() as u64;
                let session_for_inject = session.clone();
                let on_injected: crate::inject_summary_handler::OnInjectedCallback =
                    Arc::new(move |injected_tokens: u32| {
                        session_for_inject.set_compaction_progress(None);
                        tracing::debug!(
                            "[AGENT-LOOP] inject_summary stored DAG ({} summary tokens) — \
                             CompactionComplete deferred to apply_pending_dag_and_emit",
                            injected_tokens
                        );
                    });
                let inject_handler = crate::inject_summary_handler::create_handler(
                    session.pending_dag_content.clone(),
                    context_window,
                    session.compaction_in_progress.clone(),
                    Some(on_injected),
                );
                codelet_tools::set_inject_summary_handler(session.id, Some(inject_handler));
            }

            // SCHED-009: Register schedule handler for AI-callable Schedule tool
            {
                let schedule_handler =
                    crate::schedule_handler::create_handler(session.project.clone());
                codelet_tools::set_schedule_handler(session.id, Some(schedule_handler));
            }

            // BRIDGE-001: Set up bridge handler and session context for WebSocket relay
            // The bridge handler needs to call async handle_bridge_action, so we use
            // the tokio runtime handle to block_on the async function from the sync handler.
            let session_for_bridge = session.clone();
            let session_id_for_bridge = session.id;
            let runtime_handle = tokio::runtime::Handle::current();

            // Create the broadcast receiver factory that converts StreamChunk to JSON
            // This is the Adapter Pattern - adapts StreamChunk broadcast to JSON broadcast
            let supervisor_broadcast_sender = session_for_bridge.supervisor_broadcast.clone();
            let broadcast_rx_factory: codelet_tools::BroadcastReceiverFactory =
                Arc::new(move || {
                    // Subscribe to the supervisor broadcast
                    let mut stream_rx = supervisor_broadcast_sender.subscribe();

                    // Create a new JSON broadcast channel for this bridge connection
                    let (json_tx, json_rx) =
                        tokio::sync::broadcast::channel::<serde_json::Value>(256);

                    // Spawn an adapter task that converts StreamChunk to JSON
                    let json_tx_clone = json_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            match stream_rx.recv().await {
                                Ok(chunk) => {
                                    // Convert StreamChunk to JSON using stream_chunk_to_json_value()
                                    let json_value =
                                        crate::stream_chunk_json::stream_chunk_to_json_value(
                                            &chunk,
                                        );
                                    // Send to the JSON broadcast channel
                                    // Ignore send errors (no receivers)
                                    let _ = json_tx_clone.send(json_value);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                    tracing::warn!("Bridge adapter lagged {} messages", n);
                                    // Continue receiving
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    tracing::debug!("Bridge adapter: source broadcast closed");
                                    break;
                                }
                            }
                        }
                    });

                    json_rx
                });

            // Create input injector that sends messages to the session's supervisor input channel
            // BRIDGE-007: Updated to accept InjectedInput with optional images
            // FIX-6b: Use receive_incoming_message() instead of raw sender to centralize
            // counter logic (incoming_message_pending AtomicUsize)
            let session_for_injector = session_for_bridge.clone();
            let input_injector: codelet_tools::InputInjector =
                Arc::new(move |input: codelet_tools::InjectedInput| {
                    // Convert InjectedInput images to BridgeImageData
                    let bridge_images = input.images.map(|imgs| {
                        imgs.into_iter()
                            .map(|img| BridgeImageData {
                                data: img.data,
                                media_type: img.media_type,
                            })
                            .collect()
                    });

                    // Create a IncomingMessage message for injection from bridge
                    // Note: For bridge, we allow empty message if images are present
                    let supervisor_input = if input.message.is_empty() && bridge_images.is_some() {
                        IncomingMessage {
                            source_session_id: "bridge".to_string(),
                            role_name: "bridge".to_string(),
                            message: String::new(),
                            images: bridge_images,
                        }
                    } else {
                        IncomingMessage {
                            source_session_id: "bridge".to_string(),
                            role_name: "bridge".to_string(),
                            message: input.message.clone(),
                            images: bridge_images,
                        }
                    };

                    // FIX-6b: Route through receive_incoming_message() to track pending count
                    match session_for_injector.receive_incoming_message(supervisor_input) {
                        Ok(()) => {
                            tracing::debug!(
                                "Bridge input injected successfully: {}",
                                input.message.chars().take(50).collect::<String>()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to inject bridge input: {}", e);
                        }
                    }
                });

            // BRIDGE-008: Create control handler for interrupt/clear actions
            // BRIDGE-014: Also handles pause_response actions
            let session_for_control = session.clone();
            let control_handler: codelet_tools::ControlHandler =
                Arc::new(move |action: &str, response: Option<&str>| {
                    match action {
                        "interrupt" => {
                            tracing::info!("Bridge control: interrupting session");
                            session_for_control.interrupt();
                        }
                        "clear" => {
                            tracing::info!("Bridge control: clearing session");
                            // TUI-065: Use block_in_place because this closure is called from async context
                            // (handle_inbound_message is async). blocking_lock() panics if called directly
                            // from within a tokio runtime without this wrapper.
                            tokio::task::block_in_place(|| {
                                // DRY: Use the shared clear_history method
                                session_for_control.clear_history();
                            });
                        }
                        "pause_response" => {
                            // BRIDGE-014: Handle pause response from Telegram
                            if let Some(resp) = response {
                                tracing::info!("Bridge control: pause_response = {}", resp);
                                let pause_resp = match resp {
                                    "allow_once" => PauseResponse::AllowOnce,
                                    "allow_session" => PauseResponse::AllowSession,
                                    "deny" => PauseResponse::Denied,
                                    _ => {
                                        tracing::warn!(
                                            "Unknown pause response: {}, defaulting to deny",
                                            resp
                                        );
                                        PauseResponse::Denied
                                    }
                                };
                                session_for_control.send_pause_response(pause_resp);
                            } else {
                                tracing::warn!(
                                    "pause_response action received without response value"
                                );
                            }
                        }
                        _ => {
                            tracing::warn!("Bridge control: unknown action '{}'", action);
                        }
                    }
                });

            // Set the session context for bridge relay tasks
            // BRIDGE-017: Create command emitter for fspec command execution via bridge
            let session_for_command = session.clone();
            let command_emitter: codelet_tools::CommandEmitter =
                Arc::new(move |command, args_json, project_root, tool_call_id| {
                    // RPC-041: gate via the napi-side TSFN registration helper.
                    if !is_global_chunk_callback_registered() {
                        tracing::warn!(
                            "Cannot emit FspecCommandRequest - no global chunk callback"
                        );
                        return;
                    }

                    let fspec_request = codelet_rpc_types::FspecRequest {
                        command,
                        args_json,
                        project_root,
                        tool_call_id,
                    };

                    // Fire-and-forget: emit into the session's broadcast channel
                    session_for_command
                        .handle_output(StreamChunk::fspec_command_request(fspec_request));
                });

            codelet_tools::set_bridge_session_context(
                session_id_for_bridge,
                broadcast_rx_factory,
                input_injector,
                Some(control_handler),
                Some(command_emitter),
            );

            // Set the bridge handler that calls handle_bridge_action
            let bridge_handler: codelet_tools::BridgeHandler =
                Arc::new(move |request: codelet_tools::BridgeRequest| {
                    // Use block_in_place to run async code from sync context
                    // This is safe because we're in a multi-threaded tokio runtime
                    tokio::task::block_in_place(|| {
                        runtime_handle.block_on(async {
                            match codelet_tools::handle_bridge_action(
                                request.session_id,
                                request.action,
                            )
                            .await
                            {
                                Ok(result) => result,
                                Err(e) => codelet_tools::BridgeResult {
                                    success: false,
                                    message: format!("Bridge action failed: {}", e),
                                    connections: None,
                                },
                            }
                        })
                    })
                });

            codelet_tools::set_bridge_handler(session.id, Some(bridge_handler));

            // BRIDGE-007: Convert BridgeImageData to BridgeImage for run_agent_stream_with_images
            let bridge_images: Option<Vec<codelet_cli::interactive::BridgeImage>> =
                input_with_images.images.map(|imgs| {
                    imgs.into_iter()
                        .map(|img| codelet_cli::interactive::BridgeImage {
                            data: img.data,
                            media_type: img.media_type,
                        })
                        .collect()
                });

            // AMGR-016: Drop guard ensures set_status(Idle) executes even if the stream
            // loop panics (Rule [7]). The guard is armed when status is set to Running and
            // disarmed when the normal cleanup code runs. If a panic occurs, Drop fires
            // and transitions the session back to Idle so await_idle callers don't hang.
            struct IdleOnDropGuard {
                session: Arc<BackgroundSession>,
                armed: bool,
            }

            impl Drop for IdleOnDropGuard {
                fn drop(&mut self) {
                    if self.armed {
                        tracing::warn!(
                            "AMGR-016: IdleOnDropGuard fired (panic or early exit) — forcing session {} to Idle",
                            self.session.id
                        );
                        self.session.set_status(SessionStatus::Idle);
                        self.session.handle_output(StreamChunk::done());
                    }
                }
            }

            let mut idle_guard = IdleOnDropGuard {
                session: session.clone(),
                armed: true,
            };

            let result = match current_provider.as_str() {
                "claude" => run_with_provider!(
                    &mut inner_session,
                    get_claude,
                    input,
                    bridge_images.clone(),
                    session,
                    &output,
                    thinking_config_value
                ),
                "openai" => {
                    // PROV-051: get_openai requires session_id for cache optimization headers
                    match inner_session.provider_manager_mut().get_openai(session.id) {
                        Ok(provider) => {
                            tracing::debug!(
                                "[run_with_provider] Creating agent - session={}, getter=get_openai",
                                session.id,
                            );
                            let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers(session.id);
                            let role_preamble = session.get_role();
                            let agent = provider.create_rig_agent(
                                session.id,
                                role_preamble.as_deref(),
                                thinking_config_value.clone(),
                            );
                            if !mcp_wrappers.is_empty() {
                                tracing::info!(
                                    "[MCP] Adding {} MCP tool wrappers to agent for session {}",
                                    mcp_wrappers.len(),
                                    session.id,
                                );
                                for wrapper in mcp_wrappers {
                                    if let Err(e) = agent.tool_server_handle.add_tool(wrapper).await
                                    {
                                        tracing::warn!("[MCP] Failed to add MCP tool: {}", e);
                                    }
                                }
                            }
                            codelet_tools::set_mcp_tool_server_handle(
                                session.id,
                                agent.tool_server_handle.clone(),
                            );
                            let agent = codelet_core::RigAgent::with_default_depth(agent);
                            codelet_cli::interactive::run_agent_stream_with_images(
                                agent,
                                input,
                                bridge_images.clone(),
                                &mut inner_session,
                                session.is_interrupted.clone(),
                                session.compaction_in_progress.clone(),
                                session.interrupt_notify.clone(),
                                &output,
                                session.id,
                            )
                            .await
                        }
                        Err(e) => {
                            tracing::warn!("[run_with_provider] Failed to get provider: {}", e);
                            Err(anyhow::anyhow!("Failed to get provider: {}", e))
                        }
                    }
                }
                "gemini" => run_with_provider!(
                    &mut inner_session,
                    get_gemini,
                    input,
                    bridge_images.clone(),
                    session,
                    &output,
                    thinking_config_value
                ),
                "zai" => run_with_provider!(
                    &mut inner_session,
                    get_zai,
                    input,
                    bridge_images,
                    session,
                    &output,
                    thinking_config_value
                ),
                "codex" => run_with_provider!(
                    &mut inner_session,
                    get_codex,
                    input,
                    bridge_images.clone(),
                    session,
                    &output,
                    thinking_config_value
                ),
                // PROV-057 Layer 3 — Dispatch arm for GitHub Copilot.
                //
                // Now that Layer 2 has landed
                // [`CopilotProvider::create_rig_agent`] (see
                // `rust/providers/src/copilot/rig_agent.rs`) with the
                // same signature as every other provider in the macro, we
                // can dispatch through `run_with_provider!` directly. The
                // macro:
                //   1. Calls `provider_manager.get_github_copilot()` to
                //      build a `CopilotProvider` (Layer 2 OAuth/token
                //      handling lives there).
                //   2. Calls `provider.create_rig_agent(session.id,
                //      role_preamble, thinking_config)` to build a fully
                //      tooled rig agent wired through `CopilotHttpClient`
                //      so every request carries a refreshed Bearer token.
                //   3. Streams the agent through
                //      `run_agent_stream_with_images`, identical to the
                //      claude / gemini / zai / codex arms.
                //
                // Both spellings are accepted because the model selector
                // historically emitted both `github-copilot` (the canonical
                // provider name in `provider_manager`) and the shorter
                // `copilot` alias used by some TUI call sites.
                //
                // Scenario: Agent loop dispatches github-copilot to CopilotProvider
                // Feature: spec/features/github-copilot-end-to-end-integration.feature
                "github-copilot" | "copilot" => run_with_provider!(
                    &mut inner_session,
                    get_github_copilot,
                    input,
                    bridge_images.clone(),
                    session,
                    &output,
                    thinking_config_value
                ),
                // RPC-069: Custom("stub") dispatch — route to the
                // in-memory stub provider registered by
                // `register_stub_provider()` in
                // `rust/fspec/src/common.rs::build_service` under
                // the `test-stub-provider` feature. The stub yields
                // the canned [Text("hi back"), Done] stream via its
                // `StubModel` impl of `rig::completion::CompletionModel`
                // with no network egress.
                //
                // Gated by the `test-support` feature so production
                // builds (release `codelet-fspec` without
                // `--features test-stub-provider`) compile this arm
                // out entirely — same gate that controls whether the
                // `codelet_providers::stub_provider` and
                // `codelet_providers::stub_model` modules are even
                // available.
                //
                // Lock-step contract: this arm MUST stay paired with
                // the `"stub"` branch in
                // `agent_loop_dispatch_supports_provider`
                // (`dispatch.rs:118`). See RPC-069 rule [1].
                //
                // Feature: spec/features/stub-provider-rig-dispatch.feature
                #[cfg(feature = "test-support")]
                "stub" => {
                    if codelet_providers::stub_provider::is_stub_registered(&current_provider) {
                        tracing::debug!(
                            "[run_with_provider] Creating stub agent - session={}, provider={}",
                            session.id,
                            current_provider,
                        );
                        let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers(session.id);
                        let role_preamble = session.get_role();
                        // StubProvider is a ZST — every instance is
                        // identical. We construct a fresh one (rather
                        // than down-casting the `Arc<dyn LlmProvider>`
                        // from the registry) so we can call the
                        // inherent `create_rig_agent` method directly,
                        // mirroring the pattern every other provider
                        // arm uses.
                        let stub = codelet_providers::stub_provider::StubProvider::new();
                        let agent = stub.create_rig_agent(
                            session.id,
                            role_preamble.as_deref(),
                            thinking_config_value.clone(),
                        );
                        if !mcp_wrappers.is_empty() {
                            for wrapper in mcp_wrappers {
                                if let Err(e) = agent.tool_server_handle.add_tool(wrapper).await {
                                    tracing::warn!("[MCP] Failed to add MCP tool: {}", e);
                                }
                            }
                        }
                        codelet_tools::set_mcp_tool_server_handle(
                            session.id,
                            agent.tool_server_handle.clone(),
                        );
                        let agent = codelet_core::RigAgent::with_default_depth(agent);
                        codelet_cli::interactive::run_agent_stream_with_images(
                            agent,
                            input,
                            bridge_images.clone(),
                            &mut inner_session,
                            session.is_interrupted.clone(),
                            session.compaction_in_progress.clone(),
                            session.interrupt_notify.clone(),
                            &output,
                            session.id,
                        )
                        .await
                    } else {
                        tracing::error!(
                            "Stub provider '{}' not in in-memory registry — was register_stub_provider() called?",
                            current_provider,
                        );
                        Err(anyhow::anyhow!(
                            "Stub provider '{}' not registered",
                            current_provider
                        ))
                    }
                }
                _ => {
                    // PROV-092: Custom-provider dispatch. If the
                    // current_provider name corresponds to a registered
                    // Rhai shadow/custom provider, route through
                    // `CustomProvider::create_rig_agent` which returns a
                    // real `rig::agent::Agent<RhaiCustomProviderModel>`.
                    // Otherwise fall through to the existing "unsupported"
                    // error path so misspelled provider names still surface.
                    let project_root = std::path::PathBuf::from(&session.project);
                    let model_alias = current_model
                        .clone()
                        .unwrap_or_else(|| "default".to_string());
                    let role_preamble = session.get_role();
                    let agent_result = codelet_providers::custom::CustomProvider::create_rig_agent(
                        &project_root,
                        &current_provider,
                        &model_alias,
                        session.id,
                        role_preamble.as_deref(),
                        thinking_config_value.clone(),
                    );
                    match agent_result {
                        Ok(handle) => {
                            tracing::debug!(
                                "[run_with_provider] Creating custom-provider agent - session={}, provider={}",
                                session.id,
                                current_provider,
                            );
                            let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers(session.id);
                            let agent = handle.into_inner();
                            if !mcp_wrappers.is_empty() {
                                for wrapper in mcp_wrappers {
                                    if let Err(e) = agent.tool_server_handle.add_tool(wrapper).await
                                    {
                                        tracing::warn!("[MCP] Failed to add MCP tool: {}", e);
                                    }
                                }
                            }
                            codelet_tools::set_mcp_tool_server_handle(
                                session.id,
                                agent.tool_server_handle.clone(),
                            );
                            let agent = codelet_core::RigAgent::with_default_depth(agent);
                            codelet_cli::interactive::run_agent_stream_with_images(
                                agent,
                                input,
                                bridge_images.clone(),
                                &mut inner_session,
                                session.is_interrupted.clone(),
                                session.compaction_in_progress.clone(),
                                session.interrupt_notify.clone(),
                                &output,
                                session.id,
                            )
                            .await
                        }
                        Err(e) => {
                            tracing::error!(
                                "Unsupported provider '{}' (not a built-in and custom-provider dispatch failed: {})",
                                current_provider,
                                e,
                            );
                            Err(anyhow::anyhow!(
                                "Unsupported provider: {}",
                                current_provider
                            ))
                        }
                    }
                }
            };

            persist_pending_annotations(&session.id, &mut inner_session);

            // Apply pending DAG content from inject_summary (deferred because handler can't lock session.inner).
            // CMPCT-038: this is also the CompactionComplete emit site — the tracker is
            // recalculated by the apply, so compacted_tokens reflects the real
            // post-injection context (reminders + summary), not the summary alone.
            let pre_compaction_tokens = session.pre_compaction_tokens.load(Ordering::Acquire);
            if let Some(dag_nodes) = crate::inject_summary_handler::apply_pending_dag_and_emit(
                &mut inner_session,
                &session.pending_dag_content,
                pre_compaction_tokens,
                &|chunk| session.handle_output(chunk),
            ) {
                tracing::info!(
                    "[AGENT-LOOP] Applied pending DAG for session {} — messages_len={}, tokens={}, dag_nodes={}",
                    session.id,
                    inner_session.messages.len(),
                    inner_session.token_tracker.input_tokens,
                    dag_nodes.len(),
                );

                // KGRAPH-021: Extract learnings from the DAG summary at compaction boundary.
                // This is a session boundary event — the right time to extract knowledge.
                // Uses the Residue methodology: sends the DAG text to the current LLM with
                // LEARNINGS_EXTRACTION_PROMPT, then passes the response to the extraction pipeline.
                // Fire-and-forget on a background thread to not block the agent loop.
                // Errors are logged via tracing (not silently swallowed).
                {
                    let dag_text: String = dag_nodes
                        .iter()
                        .map(|n| n.label.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    let learnings_provider = current_provider.clone();
                    let learnings_model = current_model.clone();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build();
                        match rt {
                            Ok(rt) => {
                                rt.block_on(async {
                                    // Make LLM call using the session's provider/model
                                    let llm_response =
                                        codelet_graph::call_learnings_extraction_llm(
                                            &learnings_provider,
                                            learnings_model.as_deref(),
                                            &dag_text,
                                        )
                                        .await;
                                    codelet_graph::extract_learnings_from_dag(
                                        &dag_text,
                                        llm_response.as_deref(),
                                    )
                                    .await;
                                });
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "[KGRAPH] Failed to create runtime for learnings extraction: {e}"
                                );
                            }
                        }
                    });
                }

                // CompactionComplete was emitted by apply_pending_dag_and_emit above
                // (CMPCT-038: Running → CompactionComplete with the recalculated
                // basis). We only need to transition to Idle now that the DAG has
                // been applied and the agent loop is finishing.
                session.set_status(SessionStatus::Idle);
                session.set_compaction_progress(None);
            }

            // CMPCT-020: Compaction convergence watchdog
            // Check if compaction was in progress but agent failed to call inject_summary.
            // The flag is still true if inject_summary was never called during the stream.
            let was_compacting = session.compaction_in_progress.load(Ordering::Acquire);
            let has_pending_dag = session
                .pending_dag_content
                .lock()
                .map(|guard| guard.is_some())
                .unwrap_or(false);

            if was_compacting && !has_pending_dag {
                // Agent failed to produce a DAG. Apply convergence escalation.
                // compaction_watchdog_attempts tracks how many times we've tried.
                compaction_watchdog_attempts += 1;
                tracing::warn!(
                    "[AGENT-LOOP] Compaction watchdog: agent failed to call inject_summary (attempt {})",
                    compaction_watchdog_attempts
                );

                if compaction_watchdog_attempts == 1 {
                    // Level 2: Inject escalation message and retry
                    tracing::warn!(
                        "[AGENT-LOOP] Compaction watchdog: Level 2 — injecting escalation message"
                    );
                    inner_session.messages.push(rig::message::Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(
                            codelet_cli::compaction_dag::COMPACTION_ESCALATION_MESSAGE,
                        )),
                    });

                    // Set up retry: inject a synthetic input so the loop runs another stream
                    compaction_retry_input = Some("Continue".to_string());

                    // Don't clear the flag — we're still in compaction mode
                    // Skip the safety net below and go back to loop top
                } else {
                    // Level 3: Force-inject fallback DAG
                    tracing::warn!(
                        "[AGENT-LOOP] Compaction watchdog: Level 3 — force-injecting fallback DAG"
                    );

                    // Extract any partial dag-nodes from recent messages
                    let partial_nodes = codelet_cli::compaction_dag::extract_partial_dag_nodes(
                        &inner_session.messages,
                    );

                    let fallback_dag = if !partial_nodes.is_empty() {
                        tracing::info!(
                            "[AGENT-LOOP] Found {} partial dag-node blocks, assembling",
                            partial_nodes.len()
                        );
                        partial_nodes.join("\n\n")
                    } else {
                        let last_turn = inner_session.messages.len().saturating_sub(1);
                        tracing::info!(
                            "[AGENT-LOOP] No partial dag-nodes found, creating minimal fallback (turns 0-{})",
                            last_turn
                        );
                        format!(
                            r#"<dag-node depth="D1" turns="0-{}" label="Auto-recovered: compaction timeout">
Session was auto-compacted due to convergence timeout.
Use SessionSearch to recover context.
</dag-node>"#,
                            last_turn
                        )
                    };

                    codelet_cli::compaction_dag::force_inject_fallback_dag(
                        &mut inner_session,
                        &session.compaction_in_progress,
                        &fallback_dag,
                    );

                    compaction_watchdog_attempts = 0;
                    compaction_retry_input = None;

                    // Emit CompactionComplete for the force-inject
                    session.set_status(SessionStatus::Idle);
                    session.set_compaction_progress(None);
                }
            } else {
                // Normal path: either not compacting, or inject_summary succeeded
                if was_compacting || has_pending_dag {
                    // Reset watchdog on success
                    compaction_watchdog_attempts = 0;
                }
            }

            // Unconditionally clear compaction_in_progress (safety net for agent failures)
            // CMPCT-020: Skip if watchdog retry is pending (we need the flag to stay true)
            if compaction_retry_input.is_none() {
                let was_compacting = session.compaction_in_progress.swap(false, Ordering::SeqCst);

                if was_compacting {
                    session.set_compaction_progress(None);
                    if session.get_status() != SessionStatus::Idle {
                        session.set_status(SessionStatus::Idle);
                    }
                }
            }

            set_pause_handler(session.id, None);
            // Clean up per-session handlers
            codelet_tools::set_fspec_handler_for_session(session.id, None);
            codelet_tools::set_session_search_handler(session.id, None);
            codelet_tools::set_graph_search_handler(session.id, None); // KGRAPH-003: Cleanup
            codelet_tools::set_inject_summary_handler(session.id, None);
            codelet_tools::set_deep_search_handler(session.id, None); // RLM-001: Cleanup
            codelet_tools::set_agent_manager_handler(session.id, None); // AMGR-009: Cleanup
            codelet_tools::set_agent_manager_async_handler(session.id, None); // AMGR-015: Cleanup
            codelet_tools::set_schedule_handler(session.id, None); // SCHED-009: Cleanup
            codelet_tools::set_hitl_handler(session.id, None); // BUG-117: Cleanup HITL handler
            codelet_tools::set_bridge_handler(session.id, None);
            codelet_tools::remove_bridge_session_context(session.id);

            // Handle result
            // Note: run_agent_stream emits StreamEvent::Done on successful completion,
            // so we only emit Done here on error (to ensure the turn is properly terminated)

            // AMGR-016: Disarm the drop guard — normal cleanup is handling the transition.
            // If we don't reach this line (panic), the guard's Drop fires instead.
            idle_guard.armed = false;

            if let Err(e) = result {
                // PROV-009-DEBUG: Log full error with chain at warn level
                tracing::warn!(
                    "[AGENT-LOOP] ERROR received - session={}, error={}, error_chain={:?}",
                    session.id,
                    e,
                    e.chain().map(|c| c.to_string()).collect::<Vec<_>>()
                );
                tracing::error!("Agent stream error for session {}: {}", session.id, e);
                session.handle_output(StreamChunk::error(e.to_string()));
                // NAPI-009-FIX: Set status to Idle BEFORE emitting Done chunk
                // This prevents race condition where JS receives Done before status is Idle
                session.set_status(SessionStatus::Idle);
                session.handle_output(StreamChunk::done());
            } else {
                // Success case: BackgroundOutput::emit already set status to Idle when Done was emitted
                // Setting it again here is idempotent and ensures consistency
                session.set_status(SessionStatus::Idle);
            }

            // RIG-014: If the streaming loop detector aborted this turn,
            // auto-continue with a synthetic "Continue" input so the
            // corrective note (staged on the session) gets injected into
            // the next turn's context and the model gets a fresh chance
            // to complete the task. Reuses the same retry-input mechanism
            // as the compaction watchdog (CMPCT-020). Bounded by
            // RIG014_MAX_LOOP_ABORT_RETRIES so a model that keeps
            // degenerating into loops doesn't cycle forever.
            if session.has_pending_loop_abort_note() {
                if loop_abort_retry_count < RIG014_MAX_LOOP_ABORT_RETRIES {
                    loop_abort_retry_count += 1;
                    tracing::info!(
                        "[RIG-014] Loop abort auto-continue: injecting synthetic Continue input (session={}, retry {}/{})",
                        session.id,
                        loop_abort_retry_count,
                        RIG014_MAX_LOOP_ABORT_RETRIES
                    );
                    compaction_retry_input = Some("Continue".to_string());
                } else {
                    tracing::warn!(
                        "[RIG-014] Loop abort retry limit reached (session={}) — giving up, waiting for user input",
                        session.id
                    );
                    // Discard the staged note so it doesn't linger into a
                    // future user turn (it was already consumed by the
                    // retries that just failed).
                    let _ = session.take_pending_loop_abort_note();
                }
            }
        }
    }
}
