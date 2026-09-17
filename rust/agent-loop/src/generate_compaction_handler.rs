//! Compactor sub-agent spawner (CMPCT-044).
//!
//! DeepSearch-clone that builds a compaction DAG for a target session in a
//! clean ephemeral sub-agent. The sub-agent:
//!
//! 1. Runs under a fresh `Uuid::new_v4()` session (never shared with the
//!    target or the caller — no persisted session record, no worktree).
//! 2. Inherits the CALLING session's provider, model, context window, and
//!    max output tokens (BUG-102 pattern — `ProviderManager::with_provider_and_model`).
//! 3. Surveys the target session EXCLUSIVELY via SessionSearch over the
//!    target UUID (the target's `compaction_in_progress` flag gates
//!    Layer-0 trimming on those reads).
//! 4. Has exactly the 7 read-only tools (`SUB_AGENT_TOOL_COUNT`) — no
//!    `inject_summary`, no Write/Edit, no GenerateCompaction (no
//!    recursion into compaction).
//! 5. Is bounded by the shared AMGR-016 wall-clock timeout
//!    (`deep_search_wall_clock_timeout`, default 600s) and a fixed
//!    sub-agent tool-depth.
//! 6. Returns the DAG text as its final response (the prompt instructs
//!    it to output the DAG instead of calling inject_summary). On
//!    timeout / failure the caller falls back to the free force-inject
//!    fallback DAG (CMPCT-044 Rule [4] / CMPCT-020 Level-3 shape).
//!
//! The handler-side pin (clear to reminders + push wrapped DAG +
//! recalculate the token tracker) is performed by
//! `codelet_cli::compactor_sub_agent::pin_dag_to_session` — the
//! sub-agent itself cannot mis-target or loop.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use codelet_core::RigAgent;
use codelet_providers::custom::CustomProvider;
use codelet_providers::custom_provider_registered;
use codelet_providers::LlmProvider;
use codelet_tools::{
    set_session_search_handler, AstGrepTool, BashTool, GlobTool, GrepTool, LsTool, ReadTool,
    SessionSearchTool, SUB_AGENT_TOOL_COUNT,
};
use rig::client::CompletionClient;
use uuid::Uuid;

use crate::deep_search_provider_config::request_config_for_provider;
use crate::deep_search_handler::provider_uses_streaming_execution;
use crate::session_search_handler;

/// Drop guard that ensures the ephemeral SessionSearch handler is always
/// cleaned up, even if the sub-agent run panics (guaranteed even on panic).
struct CompactorSessionSearchCleanup(Uuid);

impl Drop for CompactorSessionSearchCleanup {
    fn drop(&mut self) {
        set_session_search_handler(self.0, None);
    }
}

/// Fixed tool-turn depth for the compactor sub-agent.
///
/// ~6–8 SessionSearch calls + DAG composition is comfortably inside 20
/// tool-turns; beyond that the sub-agent is stuck in a search loop and
/// the wall-clock timeout would waste the budget.
pub const COMPACTION_SUB_AGENT_MAX_DEPTH: usize = 20;

/// Execute the compactor sub-agent for a target session.
///
/// Returns the DAG text on success. On timeout, spawn/build failure, or
/// LLM error, returns `Err(reason)` — the caller decides whether to fall
/// back to the free force-inject fallback DAG (production callers always
/// do — the session must ALWAYS be reduced, never left oversized).
#[allow(clippy::too_many_arguments)]
pub async fn execute_compaction_subagent(
    project_path: &Path,
    target_session_id: Uuid,
    existing_dag: Option<(String, usize)>,
    compaction_in_progress: &Arc<AtomicBool>,
    provider_name: &str,
    model_id: Option<&str>,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
) -> Result<String, String> {
    // 1. Create ephemeral session_id — NOT shared with the target or caller.
    let ephemeral_session_id = Uuid::new_v4();
    tracing::debug!(
        "[compaction-subagent] execute_compaction_subagent ENTER provider='{}' model={:?} \
         target={} ephemeral={} project={}",
        provider_name,
        model_id,
        target_session_id,
        ephemeral_session_id,
        project_path.display(),
    );

    // 2. Register the SessionSearch handler for the ephemeral session.
    //    The target's compaction_in_progress flag is passed as the
    //    compaction-trimming gate so Layer-0 trimming applies to the
    //    sub-agent's reads of the target (CMPCT-044 Rule [5]). The flag
    //    is set for the whole run and cleared after it completes.
    compaction_in_progress.store(true, Ordering::SeqCst);
    let session_search_handler = session_search_handler::create_handler(
        project_path.to_path_buf(),
        compaction_in_progress.clone(),
    );
    set_session_search_handler(ephemeral_session_id, Some(session_search_handler));

    // Drop guard ensures cleanup even if the run below panics.
    let _ss_cleanup = CompactorSessionSearchCleanup(ephemeral_session_id);

    // 3. Build the compactor prompt (FRESH or INCREMENTAL — the existing
    //    DAG must have been captured from the target BEFORE any clear).
    let prompt =
        codelet_cli::compaction_dag::build_generate_compaction_prompt(target_session_id, existing_dag);

    // 4. AMGR-016: bound the entire sub-agent execution with the shared
    //    wall-clock timeout so a stalled sub-agent cannot block the parent
    //    forever.
    let wall_clock_timeout = codelet_cli::interactive::deep_search_wall_clock_timeout();
    match tokio::time::timeout(
        wall_clock_timeout,
        build_and_run_compactor_agent(
            ephemeral_session_id,
            project_path,
            &prompt,
            target_session_id,
            provider_name,
            model_id,
            context_window,
            max_output_tokens,
        ),
    )
    .await
    {
        Ok(result) => {
            tracing::debug!(
                "[compaction-subagent] EXIT target={target_session_id} result_is_ok={}",
                result.is_ok(),
            );
            result
        }
        Err(_elapsed) => {
            let timeout_msg =
                codelet_cli::interactive::build_deep_search_timeout_message(
                    wall_clock_timeout.as_secs(),
                );
            tracing::warn!(
                "AMGR-016: compactor sub-agent timed out for target {target_session_id}: {timeout_msg}"
            );
            // The caller (production path) converts this into the free
            // force-inject fallback DAG — the session is still reduced.
            Err(timeout_msg)
        }
    }
}

/// Build the compactor sub-agent (7 read-only tools — compile-time
/// asserted against `SUB_AGENT_TOOL_COUNT`) and run it to completion.
///
/// BUG-102: provider-agnostic — `ProviderManager::with_provider_and_model`
/// inherits the calling session's provider/model envelope. Custom
/// (Rhai-scripted) providers dispatch through
/// `CustomProvider::create_rig_agent` exactly the way
/// `deep_search_handler::build_and_run_agent` does (PROV-104).
#[allow(clippy::too_many_arguments)]
async fn build_and_run_compactor_agent(
    ephemeral_session_id: Uuid,
    project_path: &Path,
    prompt: &str,
    target_session_id: Uuid,
    provider_name: &str,
    model_id: Option<&str>,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
) -> Result<String, String> {
    // Compile-time assertion: the base tool count must be 7. The compactor
    // sub-agent NEVER gets an 8th tool (no DeepSearch recursion, no
    // GraphSearch, no GenerateCompaction — no recursion into compaction).
    const _: () = assert!(
        SUB_AGENT_TOOL_COUNT == 7,
        "base tool count changed — update build_and_run_compactor_agent tool list"
    );

    // The sub-agent's ONLY context source for the target session is
    // SessionSearch over the target UUID. The prompt hard-codes the target
    // id into every SessionSearch call (its own session is its ephemeral
    // one) — it never reads the target's in-memory message list.
    let task = format!(
        "Target session: {target_session_id}\n\n{prompt}"
    );

    if custom_provider_registered(provider_name) {
        // PROV-104: Rhai-scripted custom providers dispatch through
        // CustomProvider::create_rig_agent (same path as the parent
        // agent loop — DeepSearch clone contract).
        let model_alias = model_id.unwrap_or("default");
        let handle = CustomProvider::create_rig_agent(
            project_path,
            provider_name,
            model_alias,
            ephemeral_session_id,
            Some(prompt),
            None, // thinking_config is handled by the parent; sub-agent runs vanilla
        )
        .map_err(|e| {
            tracing::warn!(
                "[compaction-subagent] CustomProvider::create_rig_agent failed for '{provider_name}': {e}"
            );
            format!(
                "Failed to build custom-provider compactor sub-agent for '{provider_name}': {e}"
            )
        })?;
        let agent = handle.into_inner();
        let rig_agent = RigAgent::new(agent, COMPACTION_SUB_AGENT_MAX_DEPTH);
        return rig_agent
            .prompt(&task)
            .await
            .map_err(|e| format!("Compactor sub-agent failed: {e}"));
    }

    // BUG-102: inherit the calling session's provider/model envelope.
    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        provider_name,
        model_id,
        context_window,
        max_output_tokens,
    )
    .map_err(|e| format!("Failed to create ProviderManager: {e}"))?;

    // Each provider returns a different generic `Agent<T>` type, so we
    // use a macro to build and run the agent for each provider (same
    // pattern as `deep_search_handler::build_and_run!`).
    macro_rules! build_and_run_compactor {
        ($provider:expr, $request_config:expr) => {{
            let request_config = $request_config;
            let mut agent_builder = $provider
                .client()
                .agent($provider.model())
                .preamble(&request_config.preamble);

            if let Some(max_tokens) = request_config.max_tokens {
                agent_builder = agent_builder.max_tokens(max_tokens);
            }

            if let Some(additional_params) = request_config.additional_params.clone() {
                agent_builder = agent_builder.additional_params(additional_params);
            }

            // Exactly the 7 read-only tools — NO inject_summary, NO
            // GenerateCompaction, NO Write/Edit (CMPCT-044 Rule [3]).
            let agent = agent_builder
                .tool(ReadTool::new(ephemeral_session_id))
                .tool(GrepTool::new(ephemeral_session_id))
                .tool(AstGrepTool::new(ephemeral_session_id))
                .tool(GlobTool::new(ephemeral_session_id))
                .tool(LsTool::new(ephemeral_session_id))
                .tool(BashTool::new(ephemeral_session_id))
                .tool(SessionSearchTool::new(ephemeral_session_id))
                .build();

            let rig_agent = RigAgent::new(agent, COMPACTION_SUB_AGENT_MAX_DEPTH);
            if provider_uses_streaming_execution(provider_name) {
                let stream = rig_agent.prompt_streaming(&task).await;
                crate::deep_search_handler::collect_final_response_from_stream(stream).await
            } else {
                rig_agent
                    .prompt(&task)
                    .await
                    .map_err(|e| format!("Compactor sub-agent failed: {e}"))
            }
        }};
    }

    match provider_name {
        "claude" => {
            let provider = manager
                .get_claude()
                .map_err(|e| format!("Failed to get Claude provider: {e}"))?;
            let request_config = request_config_for_provider(
                provider_name,
                provider.model(),
                prompt,
                provider.is_oauth_mode(),
            )
            .map_err(|e| format!("Failed to build Claude compactor config: {e}"))?;
            build_and_run_compactor!(provider, request_config)
        }
        "openai" => {
            let provider = manager
                .get_openai(ephemeral_session_id)
                .map_err(|e| format!("Failed to get OpenAI provider: {e}"))?;
            let request_config =
                request_config_for_provider(provider_name, provider.model(), prompt, false)
                    .map_err(|e| format!("Failed to build OpenAI compactor config: {e}"))?;
            build_and_run_compactor!(provider, request_config)
        }
        "gemini" => {
            let provider = manager
                .get_gemini()
                .map_err(|e| format!("Failed to get Gemini provider: {e}"))?;
            let request_config =
                request_config_for_provider(provider_name, provider.model(), prompt, false)
                    .map_err(|e| format!("Failed to build Gemini compactor config: {e}"))?;
            build_and_run_compactor!(provider, request_config)
        }
        "codex" => {
            let provider = manager
                .get_codex()
                .map_err(|e| format!("Failed to get Codex provider: {e}"))?;
            let request_config =
                request_config_for_provider(provider_name, provider.model(), prompt, false)
                    .map_err(|e| format!("Failed to build Codex compactor config: {e}"))?;
            build_and_run_compactor!(provider, request_config)
        }
        "zai" => {
            let provider = manager
                .get_zai()
                .map_err(|e| format!("Failed to get Z.AI provider: {e}"))?;
            let request_config =
                request_config_for_provider(provider_name, provider.model(), prompt, false)
                    .map_err(|e| format!("Failed to build Z.AI compactor config: {e}"))?;
            build_and_run_compactor!(provider, request_config)
        }
        _ => Err(format!(
            "Unsupported provider for compactor sub-agent: {provider_name}. \
                 Supported: claude, openai, gemini, codex, zai"
        )),
    }
}

/// Register a `CompactorSubAgentHandler` for the given session.
///
/// Mirrors `register_deep_search_handler` (bridges.rs): captures provider,
/// model, context_window, max_output from the inner session's
/// ProviderManager at call time. Called at session creation and after
/// `/model` / `/provider` changes so the captured values stay in sync.
///
/// `compaction_in_progress` is the target session's flag — it gates
/// Layer-0 trimming on the sub-agent's SessionSearch reads of the target
/// (CMPCT-044 Rule [5]) and is set/cleared by the trigger site for the
/// whole sub-agent run.
pub fn register_compactor_sub_agent_handler(
    session_id: Uuid,
    inner_session: &codelet_cli::session::Session,
    project_path: std::path::PathBuf,
    compaction_in_progress: Arc<AtomicBool>,
) {
    let compactor_provider = inner_session
        .provider_manager()
        .facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| inner_session.current_provider_name().to_string());
    let compactor_model = inner_session.current_model_id().map(|s| s.to_string());
    let compactor_context_window = inner_session.provider_manager().raw_model_context_window();
    let compactor_max_output = inner_session
        .provider_manager()
        .raw_model_max_output_tokens();

    let handler: codelet_cli::compactor_sub_agent::CompactorSubAgentHandler = Arc::new(
        move |target_session_id, existing_dag| {
            let path = project_path.clone();
            let provider = compactor_provider.clone();
            let model = compactor_model.clone();
            let flag = compaction_in_progress.clone();
            Box::pin(async move {
                execute_compaction_subagent(
                    &path,
                    target_session_id,
                    existing_dag,
                    &flag,
                    &provider,
                    model.as_deref(),
                    compactor_context_window,
                    compactor_max_output,
                )
                .await
            })
        },
    );

    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(session_id, Some(handler));
}

/// CMPCT-045: Register the `GenerateCompaction` tool handler for a session.
///
/// Mirrors `register_deep_search_handler` (bridges.rs): captures provider,
/// model, context_window, max_output from the inner session's
/// ProviderManager at call time, plus the caller's `BackgroundSession`
/// (status / progress / `pending_dag_content` for the calling-session pin)
/// and the owning `SessionManager` (needed to reach other live sessions'
/// `inner`). Called at session creation and after `/model` / `/provider`
/// changes so the captured values stay in sync; cleared in the end-of-turn
/// cleanup block.
///
/// The tools-crate `GenerateCompactionTool` dispatches here per CALLING
/// session (the tool's construction session id). The target session is
/// resolved by the tool (explicit `session_id` arg or the calling session)
/// and passed to this closure:
///
/// - **target == caller**: the agent loop holds the caller's `inner` lock
///   for the whole turn, so the handler CANNOT lock it (deadlock). It sets
///   `compaction_in_progress`, runs the sub-agent, and stashes the wrapped
///   DAG in `pending_dag_content` — the existing end-of-turn
///   `apply_pending_dag_and_emit` path performs the pin +
///   `CompactionComplete` emit (CMPCT-045 Rule [11]).
/// - **target != caller**: the handler locks the target's `inner` (a
///   different session — no deadlock), pins immediately, emits
///   `CompactionComplete` on the target, and clears the flag before
///   returning.
/// - **unknown target**: rejected — pinning requires a live session's
///   in-memory message list (CMPCT-045 Rule [12]).
pub fn register_generate_compaction_handler(
    session_id: Uuid,
    inner_session: &codelet_cli::session::Session,
    project_path: std::path::PathBuf,
    session: std::sync::Arc<codelet_sessions::background_session::BackgroundSession>,
    owning_manager: Option<std::sync::Arc<codelet_sessions::session_manager::SessionManager>>,
) {
    let gc_provider = inner_session
        .provider_manager()
        .facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| inner_session.current_provider_name().to_string());
    let gc_model = inner_session.current_model_id().map(|s| s.to_string());
    let gc_context_window = inner_session.provider_manager().raw_model_context_window();
    let gc_max_output = inner_session.provider_manager().raw_model_max_output_tokens();

    let handler: codelet_tools::GenerateCompactionHandler =
        std::sync::Arc::new(move |target_session_id| {
            let path = project_path.clone();
            let provider = gc_provider.clone();
            let model = gc_model.clone();
            let session = session.clone();
            let manager = owning_manager.clone();
            Box::pin(async move {
                execute_generate_compaction(
                    session_id,
                    target_session_id,
                    &path,
                    &session,
                    manager.as_deref(),
                    &provider,
                    model.as_deref(),
                    gc_context_window,
                    gc_max_output,
                )
                .await
            })
        });

    codelet_tools::set_generate_compaction_handler(session_id, Some(handler));
}

/// CMPCT-045: run the compaction for `target` and pin the DAG handler-side.
///
/// Returns the pinned DAG text (success or fallback — the tool result is
/// always `Ok` once a handler exists; `Err` is reserved for unknown-target
/// rejections, which the tools-crate tool maps to `ToolError::Execution`).
#[allow(clippy::too_many_arguments)]
async fn execute_generate_compaction(
    caller_session_id: Uuid,
    target_session_id: Uuid,
    project_path: &Path,
    caller_session: &std::sync::Arc<codelet_sessions::background_session::BackgroundSession>,
    owning_manager: Option<&codelet_sessions::session_manager::SessionManager>,
    provider_name: &str,
    model_id: Option<&str>,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
) -> Result<String, String> {
    let is_callee = target_session_id == caller_session_id;

    // Resolve the target session.
    let target_bg = if is_callee {
        caller_session.clone()
    } else {
        match owning_manager {
            Some(manager) => manager
                .get_session(&target_session_id.to_string())
                .map_err(|_| {
                    format!(
                        "target session {target_session_id} is not a live session — \
                         GenerateCompaction pins a live session's in-memory context"
                    )
                })?,
            None => {
                return Err(format!(
                    "target session {target_session_id} is not a live session — \
                     GenerateCompaction pins a live session's in-memory context"
                ))
            }
        }
    };

    // Capture the target's existing DAG BEFORE any clear (CMPCT-019 /
    // CMPCT-045 Rule [5]): Some((content, max_turn_end)) ⇒ INCREMENTAL,
    // None ⇒ FRESH. The pre-compaction token basis (CMPCT-038) is the
    // TARGET's tracker total under the same lock — the handler holds no
    // shared token state, so the tracker is the honest pre-run basis.
    let (existing_dag, original_tokens) = {
        let inner = target_bg.inner.lock().await;
        (
            codelet_cli::compaction_dag::detect_existing_dag(&inner.messages),
            u32::try_from(inner.token_tracker.input_tokens).unwrap_or(u32::MAX),
        )
    };

    // Also mirror the basis into the shared pre-compaction store so the
    // end-of-turn path (calling-session targets) sees it.
    target_bg.store_pre_compaction_tokens(original_tokens);
    target_bg.set_status(codelet_rpc_types::SessionStatus::Compacting);
    let total_turns = (target_bg.inner.lock().await.messages.len() as u32).max(1);
    target_bg.update_compaction_progress(
        "Compaction sub-agent working".to_string(),
        0,
        total_turns,
    );

    // CMPCT-044 Rule [5]: the target's flag gates Layer-0 trimming on the
    // sub-agent's SessionSearch reads of the target for the whole run.
    target_bg
        .compaction_in_progress
        .store(true, std::sync::atomic::Ordering::SeqCst);

    let run_result = execute_compaction_subagent(
        project_path,
        target_session_id,
        existing_dag,
        &target_bg.compaction_in_progress,
        provider_name,
        model_id,
        context_window,
        max_output_tokens,
    )
    .await;

    // Convergence guarantee (CMPCT-044 Rule [4]): on timeout / failure /
    // unparseable output, assemble a fallback DAG (partial nodes, else the
    // generic auto-recovered node) and pin it with zero further LLM cost.
    let (dag_text, fallback_reason) = match run_result {
        Ok(text) => {
            if codelet_core::compaction::parse_dag_nodes(&text, None).is_empty() {
                let reason = "sub-agent returned no parseable <dag-node> blocks".to_string();
                tracing::warn!(
                    "[generate-compaction] {reason} for target {target_session_id} — \
                     falling back to the recovered DAG"
                );
                (build_fallback_dag(&text, total_turns), Some(reason))
            } else {
                (text, None)
            }
        }
        Err(reason) => {
            tracing::warn!(
                "[generate-compaction] sub-agent failed for target {target_session_id}: \
                 {reason} — pinning the free fallback DAG"
            );
            (build_fallback_dag(&reason, total_turns), Some(reason))
        }
    };

    if is_callee {
        // Deadlock rule (CMPCT-045 Rule [11]): the agent loop holds the
        // caller's `inner` lock for the whole turn — stash the wrapped DAG
        // in `pending_dag_content` and let the existing end-of-turn
        // `apply_pending_dag_and_emit` path perform the pin, the
        // CompactionComplete emit (honest post-pin basis), the flag
        // clear, and the status transition.
        target_bg
            .compaction_in_progress
            .store(false, std::sync::atomic::Ordering::SeqCst);
        target_bg.set_compaction_progress(None);
        let wrapped = codelet_core::compaction::wrap_dag_content(&dag_text);
        if let Ok(mut guard) = target_bg.pending_dag_content.lock() {
            *guard = Some(wrapped);
        } else {
            return Err("failed to acquire pending_dag_content lock for the calling session".to_string());
        }
        tracing::debug!(
            "[generate-compaction] caller-session target — DAG stashed in \
             pending_dag_content; end-of-turn path will pin it (fallback={fallback_reason:?})"
        );
        return Ok(with_fallback_note(&dag_text, fallback_reason.as_deref()));
    }

    // Cross-session pin: lock the target's inner (a different session —
    // no deadlock), clear-to-reminders + push the wrapped DAG +
    // recalculate the tracker, emit the lifecycle events on the target,
    // and clear the flag.
    let mut inner = target_bg.inner.lock().await;
    let pre_pin_tokens = inner.token_tracker.input_tokens;

    let _counts = codelet_cli::interactive_helpers::reset_session_to_reminders(&mut inner);
    let wrapped = codelet_core::compaction::wrap_dag_content(&dag_text);
    inner.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text(&wrapped)),
    });
    codelet_cli::interactive_helpers::recalculate_token_tracker(&mut inner);
    inner.token_tracker.reset_after_compaction();
    let post_pin_tokens = inner.token_tracker.input_tokens;

    let ratio =
        (codelet_cli::interactive_helpers::compression_ratio(
            u64::from(original_tokens),
            post_pin_tokens as u64,
        )
            * 100.0)
            .max(0.0);
    target_bg.handle_output(
        codelet_rpc_types::StreamChunk::session_state_change(
            codelet_rpc_types::SessionState::Running,
        ),
    );
    target_bg.handle_output(codelet_rpc_types::StreamChunk::compaction_complete(
        codelet_rpc_types::CompactionResult {
            original_tokens,
            compacted_tokens: u32::try_from(post_pin_tokens).unwrap_or(u32::MAX),
            compression_ratio: ratio,
            turns_summarized: 0,
            turns_kept: 0,
        },
    ));

    target_bg
        .compaction_in_progress
        .store(false, std::sync::atomic::Ordering::SeqCst);
    target_bg.set_compaction_progress(None);
    target_bg.set_status(codelet_rpc_types::SessionStatus::Running);

    tracing::debug!(
        "[generate-compaction] pinned DAG to target {target_session_id} — \
         tokens={pre_pin_tokens}->{post_pin_tokens}, fallback={fallback_reason:?}"
    );
    Ok(with_fallback_note(&dag_text, fallback_reason.as_deref()))
}

/// CMPCT-045 Rule [15]: the tool result for a fallback pin carries a
/// structured note that it is a free force-inject fallback and why — the
/// DAG text itself stays first so the caller sees exactly what was pinned.
fn with_fallback_note(dag_text: &str, fallback_reason: Option<&str>) -> String {
    match fallback_reason {
        None => dag_text.to_string(),
        Some(reason) => format!(
            "[free force-inject fallback — the compactor sub-agent failed or timed out: {reason}]\n{dag_text}"
        ),
    }
}

/// Assemble the fallback DAG for a failed/timed-out sub-agent: recover
/// any complete `<dag-node>` blocks from its output, else emit the generic
/// `Auto-recovered: compaction timeout` D1 node (CMPCT-020 Level-3 shape).
fn build_fallback_dag(sub_agent_output: &str, total_turns: u32) -> String {
    let partial = codelet_cli::compaction_dag::extract_partial_dag_nodes_from_text(
        sub_agent_output,
    );
    if !partial.is_empty() {
        partial.join("\n\n")
    } else {
        format!(
            r#"<dag-node depth="D1" turns="0-{}" label="Auto-recovered: compaction timeout">
Session was auto-compacted due to a compaction-sub-agent timeout.
Use SessionSearch to recover context.
</dag-node>"#,
            total_turns.saturating_sub(1)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario: Compactor sub-agent has only the read-only tool surface
    /// @step Given a compactor sub-agent is constructed
    /// @step When its tool list is built
    /// @step Then the sub-agent has exactly the seven read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
    #[test]
    fn compactor_sub_agent_keeps_the_seven_read_only_tools() {
        // @step And the sub-agent does NOT have the inject_summary tool
        // @step And the sub-agent does NOT have the GenerateCompaction tool
        // @step And the sub-agent does NOT have any Write or Edit tool
        assert_eq!(
            SUB_AGENT_TOOL_COUNT, 7,
            "the compactor sub-agent must keep exactly the 7 read-only DeepSearch tools"
        );
        assert_eq!(
            codelet_tools::SUB_AGENT_TOOL_NAMES,
            ["Read", "Grep", "AstGrep", "Glob", "Ls", "Bash", "SessionSearch"],
            "the compactor sub-agent's tool surface must be exactly the 7 \
             read-only tools"
        );
        assert!(
            !codelet_tools::SUB_AGENT_TOOL_NAMES
                .iter()
                .any(|t| t.to_lowercase().contains("inject_summary")),
            "the compactor sub-agent must NOT have inject_summary (the pin is handler-side)"
        );
        assert!(
            !codelet_tools::SUB_AGENT_TOOL_NAMES
                .iter()
                .any(|t| *t == "GenerateCompaction"),
            "the compactor sub-agent must NOT have GenerateCompaction (no recursion into compaction)"
        );
        assert!(
            !codelet_tools::SUB_AGENT_TOOL_NAMES
                .iter()
                .any(|t| *t == "Write" || *t == "Edit"),
            "the compactor sub-agent must NOT have Write or Edit tools"
        );
    }

    /// Scenario: Compactor sub-agent is bounded by the fixed tool-depth
    #[test]
    fn compactor_sub_agent_depth_is_bounded() {
        assert!(
            (1..=25).contains(&COMPACTION_SUB_AGENT_MAX_DEPTH),
            "the compactor sub-agent tool-depth must be a bounded fixed constant"
        );
    }

    /// Scenario: Compactor spawner reuses the shared AMGR-016 wall-clock timeout
    #[test]
    fn compactor_sub_agent_shares_the_deep_search_wall_clock_timeout() {
        use codelet_cli::interactive::{
            deep_search_wall_clock_timeout, DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS,
        };
        assert_eq!(
            deep_search_wall_clock_timeout().as_secs(),
            DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS,
            "the compactor sub-agent must reuse the shared AMGR-016 wall-clock timeout"
        );
    }

    /// Scenario: Compactor spawner uses the compaction prompt for the target session
    #[test]
    fn compactor_task_names_the_target_session() {
        let target = Uuid::new_v4();
        let prompt = codelet_cli::compaction_dag::build_generate_compaction_prompt(target, None);
        assert!(
            prompt.contains(&target.to_string()),
            "the compactor prompt must name the target session id"
        );
    }
}
