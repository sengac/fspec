//! Session creation helper — RPC-425.
//!
//! Extracts the shared session creation logic from `create_session_with_id`
//! and `create_session_from_manifest` into a common helper function.
//!
//! ## Feature
//! `spec/features/extract-shared-session-creation.feature`

use std::sync::Arc;

use codelet_core::lifecycle_hooks::{load_lifecycle_hooks, run_pre_tool};
use codelet_tools::pre_tool_hook::{
    register_pre_tool_hook, PreToolHookDecision, PreToolHookHandler,
};
use codelet_tools::McpInjection;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::background_session::{BackgroundSession, PromptInput};

/// Result of creating a background session.
///
/// Contains the session and channels needed for agent loop spawning.
pub struct SessionCreationResult {
    /// The fully constructed BackgroundSession
    pub session: Arc<BackgroundSession>,
    /// Receiver for prompt inputs (caller passes to agent loop)
    pub input_rx: mpsc::Receiver<PromptInput>,
    /// Receiver for MCP injections (caller passes to agent loop)
    pub mcp_injection_rx: mpsc::Receiver<McpInjection>,
}

/// Parsed model information, extracted by the caller before calling the helper.
///
/// This avoids redundant model parsing inside the helper since the caller
/// already parsed the model string via `parse_model_string`.
pub struct ParsedModelInfo<'a> {
    /// Model string (e.g., "anthropic/claude-opus-4-5")
    pub model: &'a str,
    /// Registry provider (e.g., "anthropic")
    pub registry_provider: &'a str,
    /// Whether this is a profile model
    pub is_profile_model: bool,
    /// Whether this is a codex model
    pub is_codex_model: bool,
    /// Whether this is a custom model
    pub is_custom_model: bool,
}

/// Parameters for creating a background session.
///
/// All values are pre-parsed by the caller. The helper focuses on session
/// construction, configuration, and initialization — not parsing or credentials.
pub struct SessionCreationParams<'a> {
    /// Session UUID
    pub uuid: Uuid,
    /// Session name
    pub name: &'a str,
    /// Project path string
    pub project: &'a str,
    /// Project path buffer
    pub project_path: &'a std::path::Path,
    /// Parsed model information (from `parse_model_string`)
    pub parsed_model: ParsedModelInfo<'a>,
    /// Provider ID (e.g., "anthropic")
    pub provider_id: Option<String>,
    /// Model ID (e.g., "claude-opus-4-5")
    pub model_id: Option<String>,
    /// Worktree path (for isolated sessions)
    pub worktree_path: Option<std::path::PathBuf>,
    /// Base commit (for isolated sessions)
    pub base_commit: Option<String>,
    /// Isolation context (for isolated sessions)
    pub isolation: Option<codelet_cli::session::context_gathering::IsolationContext>,
    /// Chunks broadcast sender
    pub chunks_tx:
        broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)>,
    /// Status changes broadcast sender
    pub status_changes_tx: broadcast::Sender<(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::SessionStatus,
    )>,
}

/// Create a BackgroundSession with all shared initialization logic.
///
/// This helper extracts the ~200 lines of duplicated code from
/// `create_session_with_id` and `create_session_from_manifest`.
///
/// # Parameters
/// - `params`: Pre-parsed session creation parameters (model already parsed by caller)
/// - `provider_manager`: Already-configured provider manager
///
/// # Returns
/// A SessionCreationResult containing the BackgroundSession and channels.
///
/// # Note
/// Credentials resolution and model parsing are done by the CALLER, not this helper.
/// This helper focuses on:
/// - Model type logging
/// - Model selection via `apply_model_selection`
/// - Session construction from provider manager
/// - Context reminder injection
/// - Lifecycle hooks loading
/// - BackgroundSession construction
/// - Thinking level application
/// - Model limits setting
/// - Pre-tool hook registration
/// - MCP session initialization
pub async fn create_background_session_inner(
    params: SessionCreationParams<'_>,
    provider_manager: codelet_providers::ProviderManager,
) -> Result<SessionCreationResult, String> {
    let SessionCreationParams {
        uuid,
        name,
        project,
        project_path,
        parsed_model,
        provider_id,
        model_id,
        worktree_path,
        base_commit,
        isolation,
        chunks_tx,
        status_changes_tx,
    } = params;

    let (input_tx, input_rx) = mpsc::channel::<PromptInput>(32);

    let ParsedModelInfo {
        model,
        registry_provider,
        is_profile_model,
        is_codex_model,
        is_custom_model,
    } = parsed_model;

    tracing::info!(
        session_id = %uuid,
        provider_id = ?provider_id,
        model_id = ?model_id,
        "create_background_session_inner: resolved provider and model"
    );

    if is_profile_model {
        tracing::info!(
            "PROV-007: Profile model detected, using set_model_direct for {}",
            model
        );
    } else if is_codex_model {
        tracing::info!(
            "PROV-018: Codex model detected, using set_model_direct for {}",
            model
        );
    } else if is_custom_model {
        tracing::info!(
            "PROV-096: Custom provider '{}' detected, using set_model_direct for {}",
            registry_provider,
            model
        );
    }

    // RPC-343: apply the selection via the shared resolver so creation and
    // the mid-session set_model path can never drift.
    let mut provider_manager = provider_manager;
    let resolved = crate::model_resolution::apply_model_selection(&mut provider_manager, model)?;

    // BUG-168: store the resolved vision capability in the tool-layer registry
    // so the Read tool can default non-vision sessions to text mode.
    codelet_tools::model_capabilities::set_session_model_vision(
        uuid,
        crate::model_resolution::resolve_model_vision(&provider_manager),
    );
    // PROV-144: store the resolved per-profile image budget (absent => None so
    // the Read tool applies its default of 4) in the tool-layer registry
    // alongside the vision entry, sourced from the shared resolver so the
    // create path cannot drift.
    codelet_tools::model_capabilities::set_session_model_max_images(
        uuid,
        crate::model_resolution::resolve_profile_max_images(&provider_manager),
    );

    let initial_context_window = resolved.context_window;
    let initial_max_output_tokens = resolved.max_output_tokens;

    let mut inner = codelet_cli::session::Session::from_provider_manager(provider_manager);

    // PROV-143: seed the session's preserve-thinking flag from the profile's
    // stored default. Profile (OpenAI local-server) sessions default to
    // DISABLED — old thinking blocks are stripped from the outgoing chat
    // history so the model is not confused by stale reasoning. Non-profile
    // sessions keep the historical preserve behavior (required for
    // Anthropic/Gemini signed thinking blocks).
    if is_profile_model {
        let (colon_idx, slash_idx) = match (model.find(':'), model.find('/')) {
            (Some(colon), Some(slash)) if colon < slash => (colon, slash),
            _ => (0, 0),
        };
        let profile_name = &model[colon_idx + 1..slash_idx];
        let preserve = crate::profile_sections::load_local_server_profiles()
            .into_iter()
            .find(|p| p.name == profile_name)
            .and_then(|p| p.preserve_thinking)
            .unwrap_or(false);
        inner.preserve_thinking_enabled = preserve;
        tracing::info!(
            session_id = %uuid,
            profile = %profile_name,
            preserve,
            "PROV-143: seeded preserve-thinking flag from profile"
        );
    }

    // Inject context reminders (with isolation context for isolated sessions)
    if let Some(isolation_ctx) = &isolation {
        inner.inject_context_reminders_with_isolation(Some(isolation_ctx));
    } else {
        inner.inject_context_reminders();
    }

    let lifecycle_hooks =
        match load_lifecycle_hooks(Some(project_path), dirs::home_dir().as_deref()) {
            Ok(Some(compiled)) => Some(Arc::new(compiled)),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(
                    "[HOOK-013] Failed to load lifecycle hooks: {} - continuing without",
                    e
                );
                None
            }
        };

    let session = Arc::new(BackgroundSession::new(
        uuid,
        name.to_string(),
        project.to_string(),
        provider_id,
        model_id,
        inner,
        input_tx,
        worktree_path,
        base_commit,
        lifecycle_hooks.clone(),
        chunks_tx,
        status_changes_tx,
    ));

    // TUI-002: re-apply the persisted default thinking level
    let restored_thinking_level =
        crate::default_thinking_level_persistence::load_default_thinking_level();
    tracing::debug!(
        level = restored_thinking_level as u8,
        "create_background_session_inner: applying persisted default thinking level"
    );
    session.set_base_thinking_level(restored_thinking_level as u8);

    // PROV-142: seed the session's auto-continue state from the profile's
    // stored default (if the model is a profile model and the profile carries
    // the field). The seed happens BEFORE the first user message is
    // dispatched; runtime `/continue` still overrides it for the session's
    // lifetime (the profile default is a seed, not a lock).
    if is_profile_model {
        let (colon_idx, slash_idx) = match (model.find(':'), model.find('/')) {
            (Some(colon), Some(slash)) if colon < slash => (colon, slash),
            _ => (0, 0),
        };
        let profile_name = &model[colon_idx + 1..slash_idx];
        if let Some(profile) = crate::profile_sections::load_local_server_profiles()
            .into_iter()
            .find(|p| p.name == profile_name)
        {
            let budget = profile.auto_continue.unwrap_or(0);
            let enabled = budget >= 1;
            // CONT-002: DEFAULT_CONTINUE_BUDGET (10) when off — the same
            // default BackgroundSession::new applies.
            let seed_budget = if enabled { budget } else { 10 };
            session.set_continue_state(enabled, seed_budget);
            tracing::info!(
                session_id = %uuid,
                profile = %profile_name,
                enabled,
                budget = seed_budget,
                "PROV-142: seeded auto-continue state from profile"
            );
        }
    }

    let initial_model_id = session
        .model_id
        .read()
        .map_err(|e| format!("model_id lock poisoned: {e}"))?
        .clone();
    let initial_compaction_threshold =
        codelet_cli::compaction_threshold::resolve_compaction_threshold(
            initial_context_window as u64,
            initial_max_output_tokens as u64,
            initial_model_id.as_deref(),
            None,
        ) as u32;
    session.set_model_limits(
        initial_context_window,
        initial_max_output_tokens,
        initial_compaction_threshold,
    );

    if let Some(ref hooks) = lifecycle_hooks {
        if !hooks.pre_tool_use.is_empty() {
            let hooks_for_pre = hooks.clone();
            let session_for_pre = session.clone();
            let pre_handler: PreToolHookHandler =
                std::sync::Arc::new(move |_sid, tool_name, tool_input| {
                    let ctx = session_for_pre.hook_context();
                    let hooks = hooks_for_pre.clone();
                    let name = tool_name.to_string();
                    let input = tool_input.clone();
                    let outcome = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(run_pre_tool(&hooks, &ctx, &name, &input))
                    });
                    match outcome.decision {
                        codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Allow => {
                            PreToolHookDecision::Allow
                        }
                        codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Deny => {
                            PreToolHookDecision::Deny(
                                outcome
                                    .reason
                                    .unwrap_or_else(|| "Denied by pre_tool_use hook".to_string()),
                            )
                        }
                        codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Continue => {
                            PreToolHookDecision::Continue
                        }
                        codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Ask => {
                            PreToolHookDecision::Continue
                        }
                    }
                });
            register_pre_tool_hook(uuid, pre_handler);
        }
    }

    let (mcp_injection_rx, _mcp_connections) = codelet_tools::init_mcp_session(uuid);

    Ok(SessionCreationResult {
        session,
        input_rx,
        mcp_injection_rx,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    /// Feature: spec/features/extract-shared-session-creation.feature
    /// Scenario: Shared helper preserves all existing session setup behavior
    #[test]
    fn shared_helper_preserves_session_setup_behavior() {
        // @step Given the shared session creation helper
        let helper_content = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("session_creation_helper.rs"),
        )
        .expect("session_creation_helper.rs should exist");

        // @step When it creates a session
        // @step Then lifecycle hooks are loaded from the project path
        assert!(
            helper_content.contains("load_lifecycle_hooks"),
            "Helper must load lifecycle hooks"
        );

        // @step And pre-tool hooks are registered if lifecycle hooks exist
        assert!(
            helper_content.contains("register_pre_tool_hook"),
            "Helper must register pre-tool hooks"
        );

        // @step And MCP session is initialized
        assert!(
            helper_content.contains("init_mcp_session"),
            "Helper must initialize MCP session"
        );

        // @step And model limits are set
        assert!(
            helper_content.contains("set_model_limits"),
            "Helper must set model limits"
        );

        // @step And thinking level is applied
        assert!(
            helper_content.contains("set_base_thinking_level"),
            "Helper must set base thinking level"
        );
    }

    /// Feature: spec/features/extract-shared-session-creation.feature
    /// Scenario: Model limits and thinking level are set by shared helper
    #[test]
    fn model_limits_and_thinking_level_are_set() {
        // @step Given the shared session creation helper
        let helper_content = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("session_creation_helper.rs"),
        )
        .expect("session_creation_helper.rs should exist");

        // @step When it creates a session
        // @step Then the persisted default thinking level is applied
        assert!(
            helper_content.contains("load_default_thinking_level"),
            "Helper must load default thinking level"
        );
        assert!(
            helper_content.contains("set_base_thinking_level"),
            "Helper must set base thinking level"
        );

        // @step And model limits (context window, max output tokens, compaction threshold) are set
        assert!(
            helper_content.contains("resolve_compaction_threshold"),
            "Helper must resolve compaction threshold"
        );
        assert!(
            helper_content.contains("set_model_limits"),
            "Helper must set model limits"
        );
    }
}
