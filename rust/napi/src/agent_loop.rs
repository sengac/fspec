//! Background-session agent loop (RPC-043).
//!
//! Extracted from `rust/napi/src/session_manager.rs` lines 1216-2917
//! by RPC-043 as part of the codelet-napi thin-adapter split. This
//! module owns:
//!
//! - the `run_with_provider!` dispatch macro,
//! - `agent_loop_dispatch_supports_provider` predicate + dispatch tests,
//! - the `InputWithImages` helper struct,
//! - `pub(crate) async fn agent_loop` (the napi-side per-session task),
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

use crate::persistence::AssistantContent;
use crate::types::{NotificationSeverity, StreamChunk};

// HOOK-013: Agent lifecycle hooks — engine functions live napi-side
// because they bridge the agent_loop's per-turn state into the
// codelet-core hook runner.
use codelet_core::lifecycle_hooks::{
    run_post_tool, run_session_end, run_session_start, run_user_prompt, HookMessageLevel,
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
use crate::persist::{
    persist_assistant_message_internal, persist_pending_annotations, persist_token_state,
    persist_tool_result_internal, persist_user_message,
};

// RPC-043: bridge wiring + handler registration helpers live in
// `crate::bridges`. The agent_loop body calls
// `register_deep_search_handler` / `register_agent_manager_handler` on
// every session creation so the per-session DeepSearch / AgentManager
// closures capture the latest provider/model values.
use crate::bridges::{register_agent_manager_handler, register_deep_search_handler};

// RPC-043: `is_global_chunk_callback_registered` lives next to its
// owning static (`CHUNK_FANOUT_TSFN`) in `crate::session_bindings`.
use crate::session_bindings::is_global_chunk_callback_registered;

/// Macro to reduce duplication in provider handling.
/// Each provider returns a different concrete type, so we must match and call
/// run_agent_stream in each branch. This macro eliminates the boilerplate.
///
/// TOOL-012: Now passes session.id to create_rig_agent() so tools know which
/// session's handler to use at call time.
/// BRIDGE-007: Updated to use run_agent_stream_with_images for multimodal support.
macro_rules! run_with_provider {
    ($inner:expr, $getter:ident, $input:expr, $images:expr, $session:expr, $output:expr, $thinking:expr) => {
        match $inner.provider_manager_mut().$getter() {
            Ok(provider) => {
                // PROV-009-DEBUG: Log provider creation
                tracing::debug!(
                    "[run_with_provider] Creating agent - session={}, getter={}",
                    $session.id,
                    stringify!($getter)
                );

                // MCP-001: Gather MCP tool wrappers for this turn.
                // Connected MCP server tools appear as mcp__<server>__<tool>.
                // Uses try_read (non-blocking) — if lock is held, tools appear next turn.
                let mcp_wrappers = codelet_tools::gather_mcp_tool_wrappers($session.id);

                // BUG-120: Read session role and pass as preamble so it becomes
                // part of the system prompt. All providers handle preamble via
                // SystemPromptFacade — the role text is prepended to fspec guidance.
                let role_preamble = $session.get_role();
                // TOOL-012: Pass session.id as first parameter so tools store it at construction
                let agent = provider.create_rig_agent(
                    $session.id,
                    role_preamble.as_deref(),
                    $thinking.clone(),
                );

                // MCP-001: Add dynamic MCP tools to the built agent.
                // Uses ToolServerHandle.add_tool() to register wrappers post-build.
                if !mcp_wrappers.is_empty() {
                    tracing::info!(
                        "[MCP] Adding {} MCP tool wrappers to agent for session {}",
                        mcp_wrappers.len(),
                        $session.id,
                    );
                    for wrapper in mcp_wrappers {
                        if let Err(e) = agent.tool_server_handle.add_tool(wrapper).await {
                            tracing::warn!("[MCP] Failed to add MCP tool: {}", e);
                        }
                    }
                }

                // MCP-002: Store the ToolServerHandle in per-session MCP state so
                // ConnectMcpTool can register newly discovered tools mid-turn.
                codelet_tools::set_mcp_tool_server_handle(
                    $session.id,
                    agent.tool_server_handle.clone(),
                );

                let agent = codelet_core::RigAgent::with_default_depth(agent);
                // BRIDGE-007: Use run_agent_stream_with_images for multimodal support
                codelet_cli::interactive::run_agent_stream_with_images(
                    agent,
                    $input,
                    $images,
                    $inner,
                    $session.is_interrupted.clone(),
                    $session.compaction_in_progress.clone(),
                    $session.interrupt_notify.clone(),
                    $output,
                    $session.id,
                )
                .await
            }
            Err(e) => {
                tracing::warn!("[run_with_provider] Failed to get provider: {}", e);
                Err(anyhow::anyhow!("Failed to get provider: {}", e))
            }
        }
    };
}

/// PROV-057 Layer 3 — Pure predicate that returns `true` for every
/// provider name handled by an explicit arm in the
/// [`run_with_provider!`] match inside [`agent_loop`].
///
/// This function is kept in lock-step with the match arms so tests can
/// assert structural support for a provider without having to spin up a
/// full session. If you add an arm to the match, add the same provider
/// name here. If you remove an arm, remove it here.
///
/// Feature: spec/features/github-copilot-end-to-end-integration.feature
#[must_use]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn agent_loop_dispatch_supports_provider(provider_name: &str) -> bool {
    matches!(
        provider_name,
        "claude" | "openai" | "gemini" | "zai" | "codex" | "github-copilot" | "copilot"
    )
}

/// Input with optional images for multimodal support (BRIDGE-007)
struct InputWithImages {
    /// The text prompt
    text: String,
    /// Optional thinking config JSON
    thinking_config: Option<String>,
    /// Optional images from bridge (BRIDGE-007)
    images: Option<Vec<BridgeImageData>>,
}

/// Agent loop that runs in background tokio task
/// WATCH-019: Modified to also process supervisor injections via supervisor_input_rx
/// REFAC-007: Persists messages to Rust persistence layer
/// BRIDGE-007: Now supports multimodal input with images from bridge
/// MCP-001: Processes MCP server-initiated messages (notifications + sampling)
pub(crate) async fn agent_loop(
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
        let input_to_process: Option<InputWithImages> = if let Some(retry_text) =
            compaction_retry_input.take()
        {
            tracing::info!("[AGENT-LOOP] Compaction watchdog: retrying with escalation input");
            Some(InputWithImages {
                text: retry_text,
                thinking_config: None,
                images: None,
            })
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
                                let supervisor_images: Vec<crate::types::IncomingMessageImage> = images.iter()
                                    .map(|img| crate::types::IncomingMessageImage {
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

            input_to_process_inner
        }; // end CMPCT-020 if/else

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
            let fspec_handler: codelet_tools::FspecHandler = std::sync::Arc::new(
                move |request: codelet_tools::FspecHandlerRequest| {
                    // RPC-041: Check the napi-side TSFN registration via the
                    // helper that consults CHUNK_FANOUT_TSFN. The user-visible
                    // error string is preserved verbatim for back-compat.
                    if !is_global_chunk_callback_registered() {
                        return codelet_tools::FspecHandlerResult {
                        success: false,
                        data: String::new(),
                        error: Some("Global chunk callback not registered - cannot execute fspec command".to_string()),
                        system_reminder: None,
                    };
                    }

                    // Generate a unique tool call ID for correlation
                    let tool_call_id = uuid::Uuid::new_v4().to_string();

                    // Emit FspecCommandRequest chunk for TypeScript to process
                    let fspec_request = crate::types::FspecRequest {
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
                },
            );

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

            // TOOL-022 P2: Register the exec-stdin request callback so the
            // deterministic quiet detector (tools crate, spawned with the
            // reaper on `run`) can push an ExecStdinRequest onto this
            // agent session's BackgroundSession. Pure overlay — NO
            // status flip, NO response channel (unlike the HITL handler).
            // BUG-171: the payload is Option — the detector also pushes
            // clears (child exit / store removal / output resumption);
            // the setter is the sole emission point for the push
            // StreamChunks, so passing the Option through routes both
            // transitions into the chunk stream.
            let session_for_exec_stdin = session.clone();
            let exec_stdin_callback: codelet_tools::unified_exec::ExecStdinRequestCallback =
                std::sync::Arc::new(move |request: Option<codelet_tools::unified_exec::ExecStdinRequest>| {
                    session_for_exec_stdin.set_exec_stdin_request(request);
                });
            codelet_tools::unified_exec::set_exec_stdin_request_callback(
                session.id,
                Some(exec_stdin_callback),
            );

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
            register_agent_manager_handler(session.id, &inner_session, session.project.clone());

            // AMGR-015: Register async handler for await_idle action
            {
                let async_handler = crate::agent_manager_handler::create_async_handler();
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
                                        crate::types::stream_chunk_to_json_value(&chunk);
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

                    let fspec_request = crate::types::FspecRequest {
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
                                    let llm_response = crate::graph::call_learnings_extraction_llm(
                                        &learnings_provider,
                                        learnings_model.as_deref(),
                                        &dag_text,
                                    )
                                    .await;
                                    crate::graph::extract_learnings_from_dag(
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
            codelet_tools::unified_exec::set_exec_stdin_request_callback(
                session.id,
                None,
            ); // TOOL-022 P2: Cleanup exec-stdin callback
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
        }
    }
}

/// Output handler for background sessions that implements StreamOutput
///
/// REFAC-007: This now accumulates assistant content blocks during streaming
/// and persists the complete assistant message on Done.
struct BackgroundOutput {
    session: Arc<BackgroundSession>,
    /// REFAC-007: Accumulated assistant content blocks for current turn
    assistant_content: std::sync::Mutex<Vec<AssistantContent>>,
    /// REFAC-007: Current provider name for message envelope
    provider: String,
    /// HOOK-013: Track last tool call name+args for post_tool_use hooks
    last_tool_call: std::sync::Mutex<Option<(String, serde_json::Value)>>,
}

impl BackgroundOutput {
    fn with_provider(session: Arc<BackgroundSession>, provider: String) -> Self {
        Self {
            session,
            assistant_content: std::sync::Mutex::new(Vec::new()),
            provider,
            last_tool_call: std::sync::Mutex::new(None),
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
        use crate::types::{
            ContextFillInfo, SessionState, StreamChunk, TokenTracker, ToolCallInfo,
            ToolProgressInfo, ToolResultInfo,
        };
        use codelet_cli::interactive::StreamEvent;

        let chunk = match event {
            StreamEvent::Text(ref text) => {
                // REFAC-007: Accumulate text for later persistence
                self.add_assistant_content(AssistantContent::Text { text: text.clone() });
                StreamChunk::text(text.clone())
            }
            StreamEvent::Thinking(ref thinking) => {
                // REFAC-007: Accumulate thinking for later persistence
                self.add_assistant_content(AssistantContent::Thinking {
                    thinking: thinking.clone(),
                    signature: None,
                });
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
                // Without this, the assistant message with tool_use would be combined with the final response.
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
                        // Get the tool name from the last_tool_call cache (set during ToolCall event)
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

                            // Spawn async task for post_tool_use hooks
                            tokio::spawn(async move {
                                let outcome = run_post_tool(
                                    &hooks_clone,
                                    &ctx,
                                    &tool_name,
                                    &tool_input,
                                    &tool_response,
                                )
                                .await;
                                // Inject additional context as notifications
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
                // CODE-009: FspecTool now uses fspec_handler (like pause_handler)
                // The handler executes before the tool returns, so tool results
                // contain actual command output, not __fspec_request__ markers.
                // No special handling needed here anymore.
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
                if let Err(e) = persist_token_state(
                    &self.session.id,
                    input_tokens,
                    output_tokens,
                    reasoning_tokens.unwrap_or(0),
                ) {
                    tracing::error!("REFAC-007: Failed to persist token state: {}", e);
                }

                // (Old KGRAPH entity pipeline flush was here — removed in KGRAPH-024 dual-graph migration)

                // Do NOT set Idle when compaction or pending DAG is active.
                if crate::inject_summary_handler::should_idle_on_done(
                    &self.session.compaction_in_progress,
                    &self.session.pending_dag_content,
                ) {
                    // NAPI-009-FIX: Set status to Idle BEFORE emitting Done chunk
                    // This prevents a race condition where JavaScript receives the Done callback
                    // and calls sessionGetStatus() before Rust has set the status to Idle.
                    // The NonBlocking callback mode means JS could process Done at any time,
                    // so we must ensure status is Idle before the chunk is sent.
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
                self.session.set_compaction_progress(None); // Clear progress on completion
                                                            // Emit state change first
                self.session
                    .handle_output(StreamChunk::session_state_change(SessionState::Idle));
                // UX-002: Send STRUCTURED CompactionComplete - no string parsing needed!
                StreamChunk::compaction_complete(crate::types::CompactionResult {
                    original_tokens: info.original_tokens,
                    compacted_tokens: info.compacted_tokens,
                    compression_ratio: info.compression_ratio * 100.0, // Convert to percentage
                    turns_summarized: 0, // Not available from CompactionCompleteInfo
                    turns_kept: 0,       // Not available from CompactionCompleteInfo
                })
            }
            StreamEvent::CompactionFailed { reason } => {
                self.session.set_status(SessionStatus::Idle);
                self.session.set_compaction_progress(None); // Clear progress on failure
                                                            // Emit state change first, then notification
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
struct BackgroundProgressEmitter {
    session: Arc<BackgroundSession>,
}

impl codelet_cli::interactive::StreamOutput for BackgroundProgressEmitter {
    fn emit(&self, event: codelet_cli::interactive::StreamEvent) {
        // Only handle ToolProgress events
        if let codelet_cli::interactive::StreamEvent::ToolProgress(tp) = event {
            let chunk = crate::types::StreamChunk::tool_progress(crate::types::ToolProgressInfo {
                tool_call_id: tp.tool_call_id,
                tool_name: tp.tool_name,
                output_chunk: tp.output_chunk,
                is_stderr: tp.is_stderr,
            });
            self.session.handle_output(chunk);
        }
    }
}

#[cfg(test)]
mod agent_loop_dispatch_tests {
    //! Feature: spec/features/github-copilot-end-to-end-integration.feature
    //!
    //! PROV-057 Layer 3 — Agent loop dispatch arm for github-copilot.
    //!
    //! These tests assert the structural contract of the
    //! [`run_with_provider!`] dispatch in [`agent_loop`]:
    //!
    //!   * `"github-copilot"` and `"copilot"` are recognised provider
    //!     names that route to a real `run_with_provider!` arm (NOT the
    //!     `_ => Unsupported provider` fallthrough).
    //!   * The arm calls
    //!     [`ProviderManager::get_github_copilot`](codelet_providers::ProviderManager::get_github_copilot)
    //!     and then
    //!     [`CopilotProvider::create_rig_agent`](codelet_providers::CopilotProvider::create_rig_agent)
    //!     just like every other provider arm — there is no longer a
    //!     "deferred / pending" error path.
    //!
    //! Note: The actual call to `create_rig_agent` happens inside the
    //! `run_with_provider!` macro expansion, so the dispatch can only be
    //! exercised by the cargo build (which proves the method exists with
    //! the right signature) plus the predicate-based assertions below
    //! that prove the match arm itself is present.

    use super::agent_loop_dispatch_supports_provider;

    // =========================================================================
    // Scenario: Agent loop dispatches github-copilot to CopilotProvider
    // =========================================================================

    #[test]
    fn agent_loop_dispatch_supports_github_copilot_arm() {
        // @step Given a session has selected a "github-copilot/gpt-4o" model
        // @step And valid Copilot credentials exist on disk
        // @step When the agent loop processes a chat message
        // @step Then the run_with_provider macro matches the "github-copilot" arm
        assert!(
            agent_loop_dispatch_supports_provider("github-copilot"),
            "run_with_provider match must have a 'github-copilot' arm"
        );

        // @step And it constructs a CopilotProvider via provider_manager.get_github_copilot()
        // (covered by the call to get_github_copilot() in the actual arm — see
        //  session_manager.rs run_with_provider! match site; this predicate is
        //  a structural proof the arm exists at all)

        // @step And the response stream completes without an "Unsupported provider" error
        // The predicate must distinguish the supported arm from the fall-through
        // `_ => Err("Unsupported provider: …")` branch.
        assert!(
            !agent_loop_dispatch_supports_provider("does-not-exist"),
            "unknown providers must NOT match the dispatch predicate"
        );
    }

    #[test]
    fn agent_loop_dispatch_supports_copilot_short_alias() {
        // Some call sites use the short 'copilot' alias; the dispatch arm
        // must accept both forms so neither falls through to Unsupported.
        assert!(
            agent_loop_dispatch_supports_provider("copilot"),
            "run_with_provider match must accept the 'copilot' short alias"
        );
    }

    #[test]
    fn agent_loop_dispatch_still_supports_existing_providers() {
        // Regression: adding the github-copilot arm must not break any
        // previously supported provider.
        for provider in ["claude", "openai", "gemini", "zai", "codex"] {
            assert!(
                agent_loop_dispatch_supports_provider(provider),
                "{provider} dispatch support regressed"
            );
        }
    }

    /// PROV-057 Layer 3 upgrade smoke test:
    ///
    /// This test imports [`CopilotProvider`] and references
    /// `CopilotProvider::create_rig_agent` as a function pointer with the
    /// EXACT signature the [`run_with_provider!`] macro expects:
    ///
    /// ```ignore
    /// fn(&CopilotProvider, uuid::Uuid, Option<&str>, Option<serde_json::Value>)
    ///     -> rig::agent::Agent<…>
    /// ```
    ///
    /// If Layer 2 ever regresses (the method is removed, renamed, or its
    /// signature changes), this test stops compiling — which is exactly
    /// the failure mode we want, because the macro arm in
    /// [`agent_loop`] would silently break in the same way.
    ///
    /// We do NOT execute the function (that would require live OAuth
    /// credentials and a real HTTP client); the type-level reference is
    /// sufficient to pin the contract at the dispatch boundary.
    #[test]
    fn copilot_create_rig_agent_signature_matches_dispatch_macro_contract() {
        use codelet_providers::copilot::CopilotProvider;

        // Bind the method as a function item so the compiler enforces the
        // exact argument and return types expected by the dispatch macro.
        // The cast to a fn pointer would over-constrain the generic
        // CompletionModel parameter, so we use a closure that captures the
        // method instead.
        let _create_rig_agent_ref =
            |provider: &CopilotProvider,
             session_id: uuid::Uuid,
             preamble: Option<&str>,
             thinking: Option<serde_json::Value>| {
                // Returning the agent value here is what proves the signature.
                // The closure is never called, so the agent itself is never
                // built — but the typechecker still has to validate this body.
                provider.create_rig_agent(session_id, preamble, thinking)
            };

        // Reaching this assertion means the closure above typechecked,
        // which means `CopilotProvider::create_rig_agent` exists with the
        // contract the dispatch macro depends on.
        assert!(
            agent_loop_dispatch_supports_provider("github-copilot"),
            "github-copilot dispatch arm must be present whenever \
             CopilotProvider::create_rig_agent is wired up"
        );
    }
}
