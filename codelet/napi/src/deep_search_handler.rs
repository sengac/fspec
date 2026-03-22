//! DeepSearch handler — creates ephemeral sub-agent for scoped corpus analysis
//!
//! Feature: spec/features/deep-search.feature
//! Feature: spec/features/recursive-deepsearch.feature
//!
//! This handler implements the actual deep search logic:
//! 1. Creates an ephemeral session_id (Uuid::new_v4())
//! 2. Registers a SessionSearch handler for the ephemeral session
//! 3. RLM-002: Optionally registers a DeepSearch handler for recursive children
//! 4. Builds a system prompt describing the scope (with recursion strategy if enabled)
//! 5. Builds a rig agent with read-only tools and provider-aware request config
//!    (plus DeepSearch tool if recursion is enabled at this depth)
//! 6. Executes query with provider-specific mode (Codex/Z.AI stream internally)
//! 7. Cleans up all handlers (guaranteed via drop guards)
//! 8. Returns the final answer string
//!
//! IMPORTANT: The base tool list (7 read-only tools) MUST stay in sync with
//! `codelet_tools::SUB_AGENT_TOOL_NAMES` and `codelet_tools::sub_agent_tool_names()`.
//! When recursion is enabled, DeepSearch is added as the 8th tool at runtime.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::deep_search_provider_config::request_config_for_provider;
use codelet_tools::{
    AstGrepTool, BashTool, GlobTool, GrepTool, LsTool, ReadTool,
    SessionSearchTool, DeepSearchTool, GraphSearchTool, SUB_AGENT_TOOL_COUNT,
    build_system_prompt, set_session_search_handler,
    set_deep_search_handler, set_graph_search_handler,
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

/// Drop guard that ensures the ephemeral DeepSearch handler is always
/// cleaned up for recursive child sessions.
struct DeepSearchCleanup(Uuid);

impl Drop for DeepSearchCleanup {
    fn drop(&mut self) {
        set_deep_search_handler(self.0, None);
    }
}

/// Drop guard that ensures the ephemeral GraphSearch handler is always
/// cleaned up, even if `build_and_run_agent` panics.
struct GraphSearchCleanup(Uuid);

impl Drop for GraphSearchCleanup {
    fn drop(&mut self) {
        set_graph_search_handler(self.0, None);
    }
}

/// Owned-types wrapper for recursive calls from handler closures.
/// The handler closure captures owned Strings; this forwards to the
/// reference-based `execute_deep_search` without lifetime issues.
/// Returns an explicitly `Send` future for the handler type signature.
#[allow(clippy::too_many_arguments)]
fn execute_deep_search_owned(
    project_path: std::path::PathBuf,
    query: String,
    scope: Option<Vec<String>>,
    max_depth: usize,
    provider_name: String,
    model_id: Option<String>,
    depth: usize,
    max_recursion_depth: usize,
) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
    Box::pin(async move {
        execute_deep_search(
            &project_path,
            &query,
            scope.as_deref(),
            max_depth,
            &provider_name,
            model_id.as_deref(),
            depth,
            max_recursion_depth,
        )
        .await
    })
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
///
/// RLM-002: Accepts `depth` and `max_recursion_depth` to support recursive
/// self-invocation. When `depth < max_recursion_depth`, the sub-agent gets
/// a DeepSearch tool configured at `depth + 1`.
#[allow(clippy::too_many_arguments)]
pub async fn execute_deep_search(
    project_path: &Path,
    query: &str,
    scope: Option<&[String]>,
    max_depth: usize,
    provider_name: &str,
    model_id: Option<&str>,
    depth: usize,
    max_recursion_depth: usize,
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
    let _ss_cleanup = SessionSearchCleanup(ephemeral_session_id);

    // RLM-002: Determine if this sub-agent can recurse further
    let can_recurse = depth < max_recursion_depth;

    // RLM-002: If recursion enabled, register a DeepSearch handler for the
    // child so IT can spawn its own recursive sub-agents at depth + 1.
    let _ds_cleanup = if can_recurse {
        let child_project_path = project_path.to_path_buf();
        let child_provider = provider_name.to_string();
        let child_model = model_id.map(|s| s.to_string());
        let child_depth = depth + 1;

        let child_handler: codelet_tools::DeepSearchHandler =
            Arc::new(move |query, scope, max_depth, max_recursion_depth| {
                let path = child_project_path.clone();
                let provider = child_provider.clone();
                let model = child_model.clone();
                execute_deep_search_owned(
                    path,
                    query,
                    scope,
                    max_depth,
                    provider,
                    model,
                    child_depth,
                    max_recursion_depth,
                )
            });
        set_deep_search_handler(ephemeral_session_id, Some(child_handler));
        Some(DeepSearchCleanup(ephemeral_session_id))
    } else {
        None
    };

    // KGRAPH-009: Register GraphSearch handler for ephemeral session when graph is available
    let graph_available = crate::graph::registry::is_graph_initialized(
        crate::graph::registry::LEARNINGS_GRAPH,
    ) || crate::graph::registry::is_graph_initialized(
        crate::graph::registry::AST_CODE_GRAPH,
    );
    let _gs_cleanup = if graph_available {
        let gs_handler = crate::graph_search_handler::create_handler();
        set_graph_search_handler(ephemeral_session_id, Some(gs_handler));
        Some(GraphSearchCleanup(ephemeral_session_id))
    } else {
        None
    };

    // 3. Build system prompt with scope description and recursion awareness
    //    KGRAPH-024: Inject learnings graph context when available
    let scope_vec = scope.unwrap_or_default();
    let mut system_prompt = build_system_prompt(scope_vec, can_recurse);

    if graph_available {
        // KGRAPH-024/KGRAPH-022: Inject formatted learnings context when available.
        // Uses build_learnings_context() which queries the Learnings graph for
        // relevant decisions, failed explorations, and conventions, then formats
        // them as structured text (much richer than raw JSON dispatch results).
        // Use block_in_place to avoid holding nanograph's !Send futures across await
        // boundaries (which causes recursion_limit overflow with Lance's complex types).
        let graph_context = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                crate::graph::learnings_context::build_learnings_context(&query).await
            })
        });

        if let Some(context) = graph_context {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(&context);
            system_prompt.push('\n');
        }
    }

    // 4. Build rig agent with read-only tools (+ DeepSearch if recursion enabled, + GraphSearch if available)
    build_and_run_agent(
        ephemeral_session_id,
        &system_prompt,
        query,
        max_depth,
        provider_name,
        model_id,
        can_recurse,
        graph_available,
    ).await

    // 5. Cleanup: drop guards dropped here (or on panic), removing handlers
}

/// Macro to build an agent with the standard 7 read-only tools and run it.
/// When `can_recurse` is true, adds DeepSearch as the 8th tool.
///
/// Each provider returns a different generic `Agent<T>` type, so we can't
/// return them from a match arm. This macro avoids duplicating the tool
/// chain + RigAgent wrapping for each provider.
macro_rules! run_agent {
    ($agent:expr, $max_depth:expr, $query:expr, $provider_name:expr) => {{
        let rig_agent = RigAgent::new($agent, $max_depth);
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

macro_rules! build_and_run {
    ($provider:expr, $request_config:expr, $session_id:expr, $query:expr, $max_depth:expr, $provider_name:expr, $can_recurse:expr, $graph_available:expr) => {{
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

        // RLM-002: Conditionally include DeepSearch tool when recursion is enabled.
        // KGRAPH-009: Conditionally include GraphSearch tool when graph is available.
        // When can_recurse is true, the agent gets base + DeepSearch.
        // When graph_available, GraphSearch is added via tool_server_handle post-build.
        if $can_recurse {
            let agent = agent_builder
                .tool(ReadTool::new($session_id))
                .tool(GrepTool::new($session_id))
                .tool(AstGrepTool::new($session_id))
                .tool(GlobTool::new($session_id))
                .tool(LsTool::new($session_id))
                .tool(BashTool::new($session_id))
                .tool(SessionSearchTool::new($session_id))
                .tool(DeepSearchTool::new($session_id))
                .build();

            // KGRAPH-009: Add GraphSearch tool dynamically when graph is available
            if $graph_available {
                let gs_tool = GraphSearchTool::new($session_id);
                if let Err(e) = agent.tool_server_handle.add_tool(gs_tool).await {
                    tracing::warn!("[KGRAPH] Failed to add GraphSearch to DeepSearch sub-agent: {e}");
                }
            }

            run_agent!(agent, $max_depth, $query, $provider_name)
        } else {
            let agent = agent_builder
                .tool(ReadTool::new($session_id))
                .tool(GrepTool::new($session_id))
                .tool(AstGrepTool::new($session_id))
                .tool(GlobTool::new($session_id))
                .tool(LsTool::new($session_id))
                .tool(BashTool::new($session_id))
                .tool(SessionSearchTool::new($session_id))
                .build();

            // KGRAPH-009: Add GraphSearch tool dynamically when graph is available
            if $graph_available {
                let gs_tool = GraphSearchTool::new($session_id);
                if let Err(e) = agent.tool_server_handle.add_tool(gs_tool).await {
                    tracing::warn!("[KGRAPH] Failed to add GraphSearch to DeepSearch sub-agent: {e}");
                }
            }

            run_agent!(agent, $max_depth, $query, $provider_name)
        }
    }};
}

fn provider_uses_streaming_execution(provider_name: &str) -> bool {
    provider_name == "codex" || provider_name == "zai"
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
/// Adds `SUB_AGENT_TOOL_COUNT` tools (7) in the base case: Read, Grep, AstGrep,
/// Glob, Ls, Bash, SessionSearch. When `can_recurse` is true, adds DeepSearch
/// as the 8th tool for recursive self-invocation (RLM-002).
#[allow(clippy::too_many_arguments)]
async fn build_and_run_agent(
    session_id: Uuid,
    system_prompt: &str,
    query: &str,
    max_depth: usize,
    provider_name: &str,
    model_id: Option<&str>,
    can_recurse: bool,
    graph_available: bool,
) -> Result<String, String> {
    // Compile-time assertion: base tool count must be 7.
    // When can_recurse, we add DeepSearch as the 8th tool at runtime.
    // KGRAPH-009: GraphSearch is added dynamically via tool_server_handle when graph is available.
    const _: () = assert!(SUB_AGENT_TOOL_COUNT == 7, "base tool count changed — update build_and_run_agent tool list");

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
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name, can_recurse, graph_available)
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
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name, can_recurse, graph_available)
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
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name, can_recurse, graph_available)
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
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name, can_recurse, graph_available)
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
            build_and_run!(provider, request_config, session_id, query, max_depth, provider_name, can_recurse, graph_available)
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
