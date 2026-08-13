// Feature: spec/features/graphsearch-tool-definition-handler-registration.feature
//
// GraphSearch Tool Definition & Handler Registration
// Tests for the tool schema, handler map, and error handling.
// Updated for KGRAPH-024: Uses only AST and Learnings actions.

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect,
    clippy::module_inception
)]
mod tests {
    use crate::graph_search::{
        execute_graph_search, has_graph_search_handler, set_graph_search_handler,
        GraphSearchAction, GraphSearchHandler,
    };
    use std::sync::Arc;
    use uuid::Uuid;

    /// Create a mock handler that returns canned responses per action type.
    fn mock_handler() -> GraphSearchHandler {
        Arc::new(
            |action: GraphSearchAction, _session_id: Uuid| match action {
                GraphSearchAction::AstStats => {
                    r#"{"nodes":{"File":0,"Function":0},"edges":{"Contains":0}}"#.to_string()
                }
                GraphSearchAction::AstSearch { query, .. } => {
                    format!(r#"{{"results":[{{"name":"{query}","type":"Function"}}]}}"#)
                }
                GraphSearchAction::AstIndex { .. } => {
                    r#"{"action":"ast_index","entities_loaded":0}"#.to_string()
                }
                GraphSearchAction::LearningsStats => {
                    r#"{"nodes":{"Learning":0,"Decision":0},"edges":{"RelatesTo":0}}"#.to_string()
                }
                GraphSearchAction::LearningsSearch { query, .. } => {
                    format!(r#"{{"results":[{{"name":"{query}","category":"decision"}}]}}"#)
                }
                _ => r#"{"results":[]}"#.to_string(),
            },
        )
    }

    // ============================================================================
    // Scenario: AstStats action returns JSON on empty graph
    // ============================================================================
    #[test]
    fn test_ast_stats_action_returns_json_on_empty_graph() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'ast_stats'
        let result = execute_graph_search(session_id, GraphSearchAction::AstStats);

        // @step Then the result contains JSON with AST node and edge type counts
        assert!(
            result.contains("\"File\":0"),
            "Stats should contain File count: {result}"
        );
        assert!(
            result.contains("\"Contains\":0"),
            "Stats should contain Contains count: {result}"
        );

        // @step And no error is returned
        assert!(
            !result.contains("error"),
            "Stats should not contain error: {result}"
        );

        // Cleanup
        set_graph_search_handler(session_id, None);
    }

    // ============================================================================
    // Scenario: AstSearch action returns matching code entities
    // ============================================================================
    #[test]
    fn test_ast_search_action_returns_matching_entities() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'ast_search' and query 'login'
        let result = execute_graph_search(
            session_id,
            GraphSearchAction::AstSearch {
                query: "login".to_string(),
                entity_type: None,
                limit: None,
                path: None,
                search_mode: None,
                decorator: None,
                parameter: None,
            },
        );

        // @step Then the result contains matching AST entities
        assert!(
            result.contains("results"),
            "Should contain results array: {result}"
        );
        assert!(
            result.contains("login"),
            "Should contain queried name: {result}"
        );

        // Cleanup
        set_graph_search_handler(session_id, None);
    }

    // ============================================================================
    // Scenario: Tool is available when agent session starts
    // ============================================================================
    #[test]
    fn test_tool_is_available_when_session_starts() {
        let session_id = Uuid::new_v4();

        // @step Given an agent session is started
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the tool definitions are listed
        let handler_exists = has_graph_search_handler(session_id);

        // @step Then GraphSearch appears in the list
        assert!(handler_exists, "GraphSearch handler should be registered");

        // Cleanup
        set_graph_search_handler(session_id, None);
    }

    // ============================================================================
    // Scenario: Unregistered handler returns descriptive error
    // ============================================================================
    #[test]
    fn test_unregistered_handler_returns_descriptive_error() {
        let session_id = Uuid::new_v4();

        // @step Given no GraphSearch handler is registered for the current session
        set_graph_search_handler(session_id, None);
        assert!(!has_graph_search_handler(session_id));

        // @step When the agent calls GraphSearch with any action
        let result = execute_graph_search(session_id, GraphSearchAction::AstStats);

        // @step Then the result is a descriptive error message indicating the handler is not available
        assert!(
            result.contains("not available")
                || result.contains("No handler")
                || result.contains("error"),
            "Should return descriptive error: {result}"
        );
    }

    // ============================================================================
    // Scenario: AstIndex action deserializes without path (backwards compatible)
    // ============================================================================
    #[test]
    fn test_ast_index_deserializes_without_path() {
        // @step Given a JSON payload for ast_index with no path field
        let json = r#"{"action_type":"ast_index"}"#;

        // @step When the action is deserialized
        let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

        // @step Then it should succeed with path set to None
        assert!(
            result.is_ok(),
            "AstIndex should parse without path: {:?}",
            result.err()
        );
        if let Ok(GraphSearchAction::AstIndex { path, .. }) = result {
            assert!(path.is_none(), "Path should be None when omitted");
        } else {
            panic!("Expected AstIndex variant, got: {:?}", result.unwrap());
        }
    }

    // ============================================================================
    // Scenario: AstIndex action deserializes with explicit path
    // ============================================================================
    #[test]
    fn test_ast_index_deserializes_with_path() {
        // @step Given a JSON payload for ast_index with path "tmp/my-repo"
        let json = r#"{"action_type":"ast_index","path":"tmp/my-repo"}"#;

        // @step When the action is deserialized
        let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

        // @step Then it should succeed with path set to "tmp/my-repo"
        assert!(
            result.is_ok(),
            "AstIndex should parse with path: {:?}",
            result.err()
        );
        if let Ok(GraphSearchAction::AstIndex { path, .. }) = result {
            assert_eq!(
                path.as_deref(),
                Some("tmp/my-repo"),
                "Path should match provided value"
            );
        } else {
            panic!("Expected AstIndex variant, got: {:?}", result.unwrap());
        }
    }

    // ============================================================================
    // Scenario: AstIndex action dispatches through handler with path
    // ============================================================================
    #[test]
    fn test_ast_index_dispatches_with_path() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'ast_index' and path 'tmp/repo'
        let result = execute_graph_search(
            session_id,
            GraphSearchAction::AstIndex {
                path: Some("tmp/repo".to_string()),
                reset: None,
                incremental: None,
            },
        );

        // @step Then the handler receives and processes the action
        assert!(
            result.contains("ast_index") || result.contains("entities_loaded"),
            "Should return index result: {result}"
        );

        // @step And no error is returned
        assert!(
            !result.contains("error"),
            "Should not contain error: {result}"
        );

        // Cleanup
        set_graph_search_handler(session_id, None);
    }

    // ============================================================================
    // Scenario: AstIndex action dispatches through handler without path
    // ============================================================================
    #[test]
    fn test_ast_index_dispatches_without_path() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'ast_index' and no path
        let result = execute_graph_search(
            session_id,
            GraphSearchAction::AstIndex {
                path: None,
                reset: None,
                incremental: None,
            },
        );

        // @step Then the handler receives and processes the action
        assert!(
            result.contains("ast_index") || result.contains("entities_loaded"),
            "Should return index result: {result}"
        );

        // Cleanup
        set_graph_search_handler(session_id, None);
    }
}
