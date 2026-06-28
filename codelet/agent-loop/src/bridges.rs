//! Bridge wiring helpers (RPC-072 Phase B lift).
//!
//! Lifted from `codelet/napi/src/bridges.rs:69-130` — the two NAPI-free
//! `register_*_handler` factories that the agent loop calls on every
//! session creation (and after `/model` / `/provider` changes) so the
//! per-session DeepSearch / AgentManager closures capture the latest
//! provider / model / context_window / max_output values.
//!
//! Only the two NAPI-free factories are lifted here. The NAPI-side
//! `bridges.rs` also contains TSFN/SessionManager-specific helpers
//! (`emit_block_notification_to_tui`, `init_bridge_metadata_providers`,
//! etc.) that belong in `codelet-napi` and stay there.
//!
//! These helpers reference the lifted handler implementations in:
//!   * `crate::deep_search_handler::execute_deep_search`
//!   * `crate::agent_manager_handler::create_handler`

use uuid::Uuid;

/// BUG-132 / MODEL-004 / RPC-072 Phase B: Build and register a
/// DeepSearch handler for the given session.
///
/// Captures provider, model, context_window, max_output_tokens from the
/// inner session's ProviderManager at call time. The agent loop body
/// invokes this on session creation and after model changes so the
/// captured values stay in sync with the user's `/model` / `/provider`
/// selections.
///
/// MODEL-004: Uses facade_override() when set, mirroring the agent_loop
/// dispatch path so DeepSearch routes through the same provider as the
/// foreground turn.
pub fn register_deep_search_handler(
    session_id: Uuid,
    inner_session: &codelet_cli::session::Session,
    project_path: std::path::PathBuf,
) {
    let deep_search_provider = inner_session
        .provider_manager()
        .facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| inner_session.current_provider_name().to_string());
    let deep_search_model = inner_session.current_model_id().map(|s| s.to_string());
    let deep_search_context_window = inner_session.provider_manager().raw_model_context_window();
    let deep_search_max_output = inner_session
        .provider_manager()
        .raw_model_max_output_tokens();

    let deep_search_handler: codelet_tools::DeepSearchHandler =
        std::sync::Arc::new(move |query, scope, max_depth, max_recursion_depth| {
            let path = project_path.clone();
            let provider = deep_search_provider.clone();
            let model = deep_search_model.clone();
            Box::pin(async move {
                crate::deep_search_handler::execute_deep_search(
                    &path,
                    &query,
                    scope.as_deref(),
                    max_depth,
                    &provider,
                    model.as_deref(),
                    0, // RLM-002: Parent session starts at depth 0
                    max_recursion_depth,
                    deep_search_context_window,
                    deep_search_max_output,
                )
                .await
            })
        });
    codelet_tools::set_deep_search_handler(session_id, Some(deep_search_handler));
}

/// BUG-132 / AMGR-013 / RPC-072 Phase B: Build and register an
/// AgentManager handler for the given session.
///
/// Captures `selected_model_string()` (provider/model in registry format,
/// BUG-136) plus the spawner's per-model context_window / max_output so
/// subordinate spawns inherit the right model envelope.
pub fn register_agent_manager_handler(
    session_id: Uuid,
    inner_session: &codelet_cli::session::Session,
    project: String,
) {
    let full_model_string = inner_session.provider_manager().selected_model_string();
    let spawner_context_window = inner_session.provider_manager().raw_model_context_window();
    let spawner_max_output = inner_session
        .provider_manager()
        .raw_model_max_output_tokens();
    let agent_manager_handler = crate::agent_manager_handler::create_handler(
        project,
        full_model_string,
        spawner_context_window,
        spawner_max_output,
    );
    codelet_tools::set_agent_manager_handler(session_id, Some(agent_manager_handler));
}

#[cfg(test)]
mod tests {
    use codelet_providers::ProviderManager;

    /// BUG-132: Extract the values that would be captured by the DeepSearch handler.
    ///
    /// This is a testable pure function that returns the four values that the
    /// DeepSearch handler closure captures from a ProviderManager. Used by tests
    /// to verify the facade_override logic and value extraction without needing
    /// to construct a full handler closure.
    #[allow(dead_code)]
    fn extract_deep_search_handler_values(
        pm: &ProviderManager,
    ) -> (String, Option<String>, Option<usize>, Option<usize>) {
        let provider = pm
            .facade_override()
            .map(|s| s.to_string())
            .unwrap_or_else(|| pm.current_provider_name().to_string());
        let model = pm.selected_model_id();
        let context_window = pm.raw_model_context_window();
        let max_output = pm.raw_model_max_output_tokens();
        (provider, model, context_window, max_output)
    }

    /// BUG-132: Extract the values that would be captured by the AgentManager handler.
    ///
    /// Returns (model_string, context_window, max_output) matching what
    /// `register_agent_manager_handler` captures.
    #[allow(dead_code)]
    fn extract_agent_manager_handler_values(
        pm: &ProviderManager,
    ) -> (Option<String>, Option<usize>, Option<usize>) {
        let model_string = pm.selected_model_string();
        let context_window = pm.raw_model_context_window();
        let max_output = pm.raw_model_max_output_tokens();
        (model_string, context_window, max_output)
    }
}
