//! DeepSearch handler — creates ephemeral sub-agent for scoped corpus analysis
//!
//! Feature: spec/features/deep-search.feature
//!
//! This handler implements the actual deep search logic:
//! 1. Creates an ephemeral session_id (Uuid::new_v4())
//! 2. Registers a SessionSearch handler for the ephemeral session
//! 3. Builds a system prompt describing the scope
//! 4. Builds a rig agent with read-only tools and provider-aware request config
//! 5. Executes query with provider-specific mode (Codex streams internally)
//! 6. Cleans up the SessionSearch handler (guaranteed via drop guard)
//! 7. Returns the final answer string
//!
//! IMPORTANT: The tool list here (7 read-only tools) MUST stay in sync with
//! `codelet_tools::SUB_AGENT_TOOL_NAMES` and `codelet_tools::sub_agent_tool_names()`.
//! If you add or remove a tool below, update the constant in
//! `codelet/tools/src/deep_search/mod.rs` as well.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::deep_search_provider_config::request_config_for_provider;
use codelet_tools::{
    AstGrepTool, BashTool, GlobTool, GrepTool, LsTool, ReadTool,
    SessionSearchTool, SUB_AGENT_TOOL_COUNT, build_system_prompt,
    set_session_search_handler,
};
use codelet_core::RigAgent;
use futures::Stream;
use futures::StreamExt;
use codelet_providers::LlmProvider;
use rig::client::CompletionClient;
use uuid::Uuid;

/// Drop guard that ensures the ephemeral SessionSearch handler is always
/// cleaned up, even if `build_and_run_agent` panics.
struct SessionSearchCleanup(Uuid);

impl Drop for SessionSearchCleanup {
    fn drop(&mut self) {
        set_session_search_handler(self.0, None);
    }
}

/// Execute a deep search by spawning an ephemeral sub-agent.
///
/// This is called from the DeepSearch handler registered in session_manager.
/// It creates a fresh sub-agent with read-only tools, runs it to completion,
/// and returns the answer.
///
/// BUG-102: The sub-agent inherits the parent session's provider and model
/// via `provider_name` and `model_id`. This ensures the sub-agent uses the
/// same LLM as the parent session, avoiding "Model is required" errors.
pub async fn execute_deep_search(
    project_path: &Path,
    query: &str,
    scope: Option<&[String]>,
    max_depth: usize,
    provider_name: &str,
    model_id: Option<&str>,
) -> Result<String, String> {
    // 1. Create ephemeral session_id — NOT shared with parent
    let ephemeral_session_id = Uuid::new_v4();

    // 2. Register SessionSearch handler for ephemeral session
    //    compaction_trimming = false (ephemeral sub-agent never compacts)
    let session_search_handler = crate::session_search_handler::create_handler(
        project_path.to_path_buf(),
        Arc::new(AtomicBool::new(false)),
    );
    set_session_search_handler(ephemeral_session_id, Some(session_search_handler));

    // Drop guard ensures cleanup even if build_and_run_agent panics
    let _cleanup = SessionSearchCleanup(ephemeral_session_id);

    // 3. Build system prompt with scope description
    let scope_vec = scope.unwrap_or_default();
    let system_prompt = build_system_prompt(scope_vec);

    // 4. Build rig agent with read-only tools and run to completion
    build_and_run_agent(
        ephemeral_session_id,
        &system_prompt,
        query,
        max_depth,
        provider_name,
        model_id,
    ).await

    // 5. Cleanup: _cleanup dropped here (or on panic), removing the handler
}

/// Macro to build an agent with the standard 7 read-only tools and run it.
///
/// Each provider returns a different generic `Agent<T>` type, so we can't
/// return them from a match arm. This macro avoids duplicating the tool
/// chain + RigAgent wrapping for each provider.
macro_rules! build_and_run {
    ($provider:expr, $request_config:expr, $session_id:expr, $query:expr, $max_depth:expr, $provider_name:expr) => {{
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

        let agent = agent_builder
            .tool(ReadTool::new($session_id))
            .tool(GrepTool::new($session_id))
            .tool(AstGrepTool::new($session_id))
            .tool(GlobTool::new($session_id))
            .tool(LsTool::new($session_id))
            .tool(BashTool::new($session_id))
            .tool(SessionSearchTool::new($session_id))
            .build();

        let rig_agent = RigAgent::new(agent, $max_depth);
        if provider_uses_streaming_execution($provider_name) {
            collect_final_response_from_stream(rig_agent.prompt_streaming($query).await).await
        } else {
            rig_agent
                .prompt($query)
                .await
                .map_err(|e| format!("DeepSearch sub-agent failed: {e}"))
        }
    }};
}

fn provider_uses_streaming_execution(provider_name: &str) -> bool {
    provider_name == "codex"
}

async fn collect_final_response_from_stream<S, R>(stream: S) -> Result<String, String>
where
    S: Stream<Item = Result<rig::agent::MultiTurnStreamItem<R>, anyhow::Error>>,
    R: Clone + Unpin + rig::completion::GetTokenUsage,
{
    let mut last_final_response: Option<String> = None;
    let stream = stream;
    futures::pin_mut!(stream);

    while let Some(item) = stream.next().await {
        match item {
            Ok(rig::agent::MultiTurnStreamItem::FinalResponse(final_response)) => {
                last_final_response = Some(final_response.response().to_string());
            }
            Ok(_) => {}
            Err(error) => {
                return Err(format!("DeepSearch sub-agent failed: {error}"));
            }
        }
    }

    match last_final_response {
        Some(response) => Ok(response),
        None => Err("DeepSearch sub-agent failed: missing final response from streaming execution".to_string()),
    }
}

/// Build a sub-agent with read-only tools and run it to completion.
///
/// BUG-102: Provider-agnostic. Uses `with_provider_and_model()` to inherit
/// the parent session's provider and model selection, avoiding the need for
/// `select_model()` and the "Model is required" error.
///
/// Adds exactly `SUB_AGENT_TOOL_COUNT` tools (7): Read, Grep, AstGrep, Glob,
/// Ls, Bash, SessionSearch. This must stay in sync with
/// `codelet_tools::SUB_AGENT_TOOL_NAMES`.
async fn build_and_run_agent(
    session_id: Uuid,
    system_prompt: &str,
    query: &str,
    max_depth: usize,
    provider_name: &str,
    model_id: Option<&str>,
) -> Result<String, String> {
    // Compile-time assertion: if SUB_AGENT_TOOL_COUNT changes, this reminds
    // the developer to verify the tool list below still matches.
    const _: () = assert!(SUB_AGENT_TOOL_COUNT == 7, "tool count changed — update build_and_run_agent tool list");

    // BUG-102: Create ProviderManager with the parent session's provider and model.
    // Uses with_provider_and_model() which sets selected_model directly without
    // needing async model registry or select_model() call.
    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        provider_name,
        model_id,
    ).map_err(|e| format!("Failed to create ProviderManager: {e}"))?;

    // BUG-102: Get provider dynamically based on the parent session's provider type.
    // Each provider returns a different generic Agent<T>, so we use a macro to
    // build and run the agent for each provider type.
    match provider_name {
        "claude" => {
            let provider = manager.get_claude()
                .map_err(|e| format!("Failed to get Claude provider: {e}"))?;
            let request_config = request_config_for_provider(
                provider_name,
                provider.model(),
                system_prompt,
                provider.is_oauth_mode(),
            )
            .map_err(|e| format!("Failed to build Claude DeepSearch config: {e}"))?;
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name)
        }
        "openai" => {
            let provider = manager.get_openai()
                .map_err(|e| format!("Failed to get OpenAI provider: {e}"))?;
            let request_config = request_config_for_provider(
                provider_name,
                provider.model(),
                system_prompt,
                false,
            )
            .map_err(|e| format!("Failed to build OpenAI DeepSearch config: {e}"))?;
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name)
        }
        "gemini" => {
            let provider = manager.get_gemini()
                .map_err(|e| format!("Failed to get Gemini provider: {e}"))?;
            let request_config = request_config_for_provider(
                provider_name,
                provider.model(),
                system_prompt,
                false,
            )
            .map_err(|e| format!("Failed to build Gemini DeepSearch config: {e}"))?;
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name)
        }
        "codex" => {
            let provider = manager.get_codex()
                .map_err(|e| format!("Failed to get Codex provider: {e}"))?;
            let request_config = request_config_for_provider(
                provider_name,
                provider.model(),
                system_prompt,
                false,
            )
            .map_err(|e| format!("Failed to build Codex DeepSearch config: {e}"))?;
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name)
        }
        "zai" => {
            let provider = manager.get_zai()
                .map_err(|e| format!("Failed to get Z.AI provider: {e}"))?;
            let request_config = request_config_for_provider(
                provider_name,
                provider.model(),
                system_prompt,
                false,
            )
            .map_err(|e| format!("Failed to build Z.AI DeepSearch config: {e}"))?;
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name)
        }
        _ => {
            Err(format!(
                "Unsupported provider for DeepSearch sub-agent: {provider_name}. \
                 Supported: claude, openai, gemini, codex, zai"
            ))
        }
    }
}

#[cfg(test)]
mod tests;
