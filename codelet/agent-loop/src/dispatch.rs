//! Provider dispatch (RPC-072 lift from
//! `codelet/napi/src/agent_loop.rs:75-297`).
//!
//! Verbatim lift of the [`run_with_provider!`] macro, the
//! [`agent_loop_dispatch_supports_provider`] structural predicate, and
//! the [`InputWithImages`] helper struct. These are the building blocks
//! the [`crate::agent_loop`] body uses to dispatch a per-turn prompt
//! to the session's currently-selected provider via the canonical Rig
//! streaming engine in
//! [`codelet_cli::interactive::run_agent_stream_with_images`].
//!
//! ## Lift notes
//!
//! - The macro body is identical to the NAPI source: every NAPI-side
//!   import the macro expands to (`codelet_tools::*`,
//!   `codelet_core::RigAgent`, `codelet_cli::interactive::*`) already
//!   lives in NAPI-free crates, so the lift compiles unchanged.
//! - The structural predicate was `pub(crate)` in the NAPI source.
//!   It is `pub` here because the
//!   [`crate::agent_loop::agent_loop_dispatch_tests`] module — a
//!   verbatim port of the NAPI-side `agent_loop_dispatch_tests` —
//!   lives next door and exercises it.
//! - [`InputWithImages`] is `pub(crate)` so the agent loop body can
//!   build it inside the `tokio::select!`; downstream consumers should
//!   not construct it directly.

use codelet_sessions::background_session::BridgeImageData;

/// Macro to reduce duplication in provider handling.
/// Each provider returns a different concrete type, so we must match and call
/// run_agent_stream in each branch. This macro eliminates the boilerplate.
///
/// TOOL-012: Now passes session.id to create_rig_agent() so tools know which
/// session's handler to use at call time.
/// BRIDGE-007: Updated to use run_agent_stream_with_images for multimodal support.
#[macro_export]
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
/// [`run_with_provider!`] match inside [`crate::agent_loop`].
///
/// This function is kept in lock-step with the match arms so tests can
/// assert structural support for a provider without having to spin up a
/// full session. If you add an arm to the match, add the same provider
/// name here. If you remove an arm, remove it here.
///
/// RPC-069: under `#[cfg(feature = "test-support")]` the `"stub"`
/// provider is also accepted — mirrors the feature-gated `"stub" =>`
/// arm in `agent_loop.rs`. Without the feature, the stub falls through
/// to the custom-provider discovery path and ultimately fails as
/// "Unsupported provider: stub" — the same behaviour a release build
/// must exhibit.
///
/// Feature: spec/features/github-copilot-end-to-end-integration.feature
/// Feature: spec/features/stub-provider-rig-dispatch.feature
#[must_use]
pub fn agent_loop_dispatch_supports_provider(provider_name: &str) -> bool {
    #[cfg(feature = "test-support")]
    {
        if provider_name == "stub" {
            return true;
        }
    }
    matches!(
        provider_name,
        "claude" | "openai" | "gemini" | "zai" | "codex" | "github-copilot" | "copilot"
    )
}

/// Input with optional images for multimodal support (BRIDGE-007).
///
/// Constructed inside the agent loop body when draining either
/// `input_rx` (regular user input) or `incoming_message_rx` (supervisor
/// / bridge input with attached images). The Rig streaming engine
/// consumes this via [`run_with_provider!`].
pub(crate) struct InputWithImages {
    /// The text prompt
    pub(crate) text: String,
    /// Optional thinking config JSON (per-turn override; superimposed on
    /// `session.inner.session_thinking_level`).
    pub(crate) thinking_config: Option<String>,
    /// Optional images from bridge (BRIDGE-007)
    pub(crate) images: Option<Vec<BridgeImageData>>,
}

#[cfg(test)]
mod agent_loop_dispatch_tests {
    //! Feature: spec/features/github-copilot-end-to-end-integration.feature
    //!
    //! PROV-057 Layer 3 — Agent loop dispatch arm for github-copilot.
    //!
    //! Verbatim port of the NAPI-side `agent_loop_dispatch_tests` —
    //! the predicate's contract is identical, so its proof-of-coverage
    //! is identical.

    use super::agent_loop_dispatch_supports_provider;

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

        // @step And the response stream completes without an "Unsupported provider" error
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

    /// PROV-057 Layer 3 upgrade smoke test: see the NAPI source for the
    /// full rationale. The closure body proves the
    /// `CopilotProvider::create_rig_agent` signature still matches what
    /// the dispatch macro expects.
    #[test]
    fn copilot_create_rig_agent_signature_matches_dispatch_macro_contract() {
        use codelet_providers::copilot::CopilotProvider;

        let _create_rig_agent_ref =
            |provider: &CopilotProvider,
             session_id: uuid::Uuid,
             preamble: Option<&str>,
             thinking: Option<serde_json::Value>| {
                provider.create_rig_agent(session_id, preamble, thinking)
            };

        assert!(
            agent_loop_dispatch_supports_provider("github-copilot"),
            "github-copilot dispatch arm must be present whenever \
             CopilotProvider::create_rig_agent is wired up"
        );
    }
}
