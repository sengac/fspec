//! Graph Database Module — Dual-Graph Architecture
//!
//! Provides embedded nanograph property graph databases for the dual-graph
//! architecture:
//!
//! 1. **AST Code Graph** (`"ast-code"`) — Code structure, dependencies, and relationships
//!    stored at `<project>/.fspec/graph/ast-code.nano/`
//! 2. **Learnings Graph** (`"learnings"`) — Accumulated knowledge, decisions, and conventions
//!    stored at `~/.fspec/graph/learnings.nano/`
//!
//! Uses a registry of named graph instances (see `registry.rs`).

/// Close all graph databases cleanly.
///
/// Should be called on process exit to avoid Lance corruption.
pub fn close_graph_db() {
    registry::close_all_graphs();
}

/// Reset all graph databases.
///
/// Called when the data directory changes (via `set_data_directory()`).
pub fn reset_graph_db() {
    registry::reset_all_graphs();
}

/// Populate the AST code graph from the current project directory.
///
/// Walks the codebase extracting functions, types, imports, and dependencies,
/// then batch-loads everything into the AST graph. Silently skips if the
/// graph is unavailable.
///
/// Called at session start so the GraphSearch tool has data to query.
pub async fn populate_ast_graph() {
    let project_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to get cwd for AST indexing: {e}");
            return;
        }
    };

    let db = match registry::get_graph(registry::AST_CODE_GRAPH).await {
        Ok(db) => db,
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to open AST graph for indexing: {e}");
            return;
        }
    };

    // Walk codebase and extract AST entities
    let mut all_entities = match ast_pipeline::walk_and_extract(&project_root, true) {
        Ok(entities) => entities,
        Err(e) => {
            tracing::warn!("[KGRAPH] AST extraction failed: {e}");
            return;
        }
    };

    // Extract dependencies (non-fatal failures)
    if let Ok(cargo_deps) =
        ast_pipeline::cargo_dep_extractor::extract_cargo_dependencies(&project_root)
    {
        all_entities.extend(cargo_deps);
    }
    if let Ok(npm_deps) =
        ast_pipeline::npm_dep_extractor::extract_npm_dependencies(&project_root)
    {
        all_entities.extend(npm_deps);
    }

    // Deduplicate after merging dep-extractor results (same reason as ast_dispatch)
    let all_entities = ast_pipeline::deduplicate_entities(all_entities);

    if all_entities.is_empty() {
        tracing::debug!("[KGRAPH] No AST entities found to index");
        return;
    }

    match db.load_entities_overwrite(&all_entities).await {
        Ok(count) => {
            tracing::info!(count, "[KGRAPH] AST graph populated at session start");
        }
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to load AST entities: {e}");
        }
    }
}

/// Extract learnings from a compaction DAG summary using LLM extraction.
///
/// Called at compaction boundaries (after inject_summary applies the DAG).
/// Uses the Residue methodology LLM extraction prompt to produce structured
/// Learning, Exploration, and Constraint entities from the DAG text.
///
/// The `llm_response` parameter is the pre-computed LLM response text.
/// The caller (session_manager.rs) is responsible for making the LLM call
/// with `LEARNINGS_EXTRACTION_PROMPT` and passing the response here.
/// This separation enables testing with fixture data.
///
/// When `llm_response` is `None` (LLM unavailable), logs a warning and
/// returns without modifying the graph.
pub async fn extract_learnings_from_dag(dag_text: &str, llm_response: Option<&str>) {
    if dag_text.trim().is_empty() {
        return;
    }

    let db = match registry::get_graph(registry::LEARNINGS_GRAPH).await {
        Ok(db) => db,
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to open Learnings graph for extraction: {e}");
            return;
        }
    };

    let entities = match learnings_extraction::extract_learnings_from_text(dag_text, llm_response) {
        Ok(result) => result.entities,
        Err(e) => {
            tracing::warn!("[KGRAPH] Learnings extraction failed: {e}");
            return;
        }
    };

    if entities.is_empty() {
        tracing::debug!("[KGRAPH] No learnings extracted from DAG summary");
        return;
    }

    match db.load_entities(&entities).await {
        Ok(count) => {
            tracing::info!(count, "[KGRAPH] Learnings extracted from DAG summary");
        }
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to load learnings entities: {e}");
        }
    }
}

/// KGRAPH-021: Call the LLM to extract learnings from a DAG summary.
///
/// Uses the same ProviderManager pattern as DeepSearch: creates a minimal
/// agent with the LEARNINGS_EXTRACTION_PROMPT as system prompt, sends the
/// DAG text as the user message, and returns the LLM response string.
///
/// Returns `None` if the LLM call fails for any reason (provider unavailable,
/// rate limit, timeout, etc.). Errors are logged but never propagated — this
/// is fire-and-forget extraction that must not block the agent loop.
pub async fn call_learnings_extraction_llm(
    provider_name: &str,
    model_id: Option<&str>,
    dag_text: &str,
) -> Option<String> {
    use codelet_providers::LlmProvider;
    use rig::client::CompletionClient;
    use rig::completion::Prompt;

    let manager = codelet_providers::ProviderManager::with_provider_and_model(
        provider_name,
        model_id,
    ).map_err(|e| {
        tracing::warn!("[KGRAPH] Failed to create ProviderManager for learnings extraction: {e}");
        e
    }).ok()?;

    let prompt_text = format!(
        "Extract learnings from this session summary:\n\n{}",
        dag_text
    );

    // Use a macro to handle each provider type (same pattern as DeepSearch build_and_run).
    // Each provider returns a different Agent<T> generic, so we can't abstract over them.
    macro_rules! call_with_provider {
        ($get_method:ident) => {{
            let provider = manager.$get_method().map_err(|e| {
                tracing::warn!("[KGRAPH] Failed to get provider for learnings extraction: {e}");
                e
            }).ok()?;
            let agent = provider
                .client()
                .agent(provider.model())
                .preamble(learnings_extraction::LEARNINGS_EXTRACTION_PROMPT)
                .build();
            match agent.prompt(&prompt_text).await {
                Ok(response) => {
                    tracing::info!(
                        chars = response.len(),
                        "[KGRAPH] LLM learnings extraction response received"
                    );
                    Some(response)
                }
                Err(e) => {
                    tracing::warn!("[KGRAPH] LLM learnings extraction call failed: {e}");
                    None
                }
            }
        }};
    }

    match provider_name {
        "claude" => call_with_provider!(get_claude),
        "openai" => {
            // PROV-051: get_openai requires session_id for cache optimization headers.
            // Learnings extraction is a background task without a user session,
            // so we use a throwaway UUID — cache affinity isn't critical here.
            let provider = manager.get_openai(uuid::Uuid::new_v4()).map_err(|e| {
                tracing::warn!("[KGRAPH] Failed to get provider for learnings extraction: {e}");
                e
            }).ok()?;
            let agent = provider
                .client()
                .agent(provider.model())
                .preamble(learnings_extraction::LEARNINGS_EXTRACTION_PROMPT)
                .build();
            match agent.prompt(&prompt_text).await {
                Ok(response) => {
                    tracing::info!(
                        chars = response.len(),
                        "[KGRAPH] LLM learnings extraction response received"
                    );
                    Some(response)
                }
                Err(e) => {
                    tracing::warn!("[KGRAPH] LLM learnings extraction call failed: {e}");
                    None
                }
            }
        },
        "gemini" => call_with_provider!(get_gemini),
        "zai" => call_with_provider!(get_zai),
        "codex" => call_with_provider!(get_codex),
        _ => {
            tracing::warn!("[KGRAPH] Unsupported provider for learnings extraction: {provider_name}");
            None
        }
    }
}

pub mod ast_call_chain;
pub mod ast_complexity;
pub mod ast_dead_code;
pub mod ast_dispatch;
pub mod ast_hierarchy;
pub mod ast_index;
pub mod ast_pipeline;
pub mod ast_transitive;
pub mod bundle;
pub mod database;
pub mod dispatch_helpers;
pub mod graph_entities;
#[cfg(test)]
mod graph_reset_tests;
pub mod learnings_context;
pub mod learnings_dispatch;
pub mod learnings_extraction;
pub mod llm_response_parser;
pub mod registry;
