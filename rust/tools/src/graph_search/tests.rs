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
                GraphSearchAction::AstStats { .. } => {
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
        // @step And the graph database is empty
        // (the mock handler answers like an empty graph: every count is zero)
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'ast_stats'
        let result = execute_graph_search(session_id, GraphSearchAction::AstStats { root: None });

        // @step Then the result contains JSON with AST node and edge type counts all at zero
        let stats: serde_json::Value = serde_json::from_str(&result).unwrap();
        for section in ["nodes", "edges"] {
            let counts = stats[section].as_object().unwrap();
            assert!(!counts.is_empty(), "{section} counts missing: {result}");
            assert!(
                counts.values().all(|count| count == 0),
                "every {section} count should be zero: {result}"
            );
        }

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
                root: None,
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
        let found: serde_json::Value = serde_json::from_str(&result).unwrap();
        let entities = found["results"].as_array().unwrap();
        assert_eq!(entities.len(), 1, "one matching entity: {result}");

        // @step And each result includes the entity name and type
        assert_eq!(entities[0]["name"], "login");
        assert_eq!(entities[0]["type"], "Function");

        // Cleanup
        set_graph_search_handler(session_id, None);
    }

    // ============================================================================
    // Scenario: Tool is available when agent session starts
    // ============================================================================
    #[tokio::test]
    async fn test_tool_is_available_when_session_starts() {
        use rig::tool::Tool;

        let session_id = Uuid::new_v4();

        // @step Given an agent session is started
        set_graph_search_handler(session_id, Some(mock_handler()));
        assert!(has_graph_search_handler(session_id));

        // @step When the tool definitions are listed
        let definition = crate::graph_search::GraphSearchTool::new(session_id)
            .definition(String::new())
            .await;

        // @step Then GraphSearch appears in the list with its JSON schema
        assert_eq!(definition.name, "GraphSearch");
        assert_eq!(definition.parameters["type"], "object");

        // @step And the schema describes all 16 action types with their parameters
        let action_types = definition.parameters["properties"]["action_type"]["enum"]
            .as_array()
            .unwrap();
        assert_eq!(action_types.len(), 16, "{action_types:?}");
        // Every action must be callable with only the parameters the schema
        // describes: fill each described string parameter and parse.
        let properties = definition.parameters["properties"].as_object().unwrap();
        let described: serde_json::Map<String, serde_json::Value> = properties
            .iter()
            .filter(|(name, schema)| *name != "action_type" && schema["type"] == "string")
            .map(|(name, _)| (name.clone(), serde_json::json!("x")))
            .collect();
        for action_type in action_types {
            let mut call = described.clone();
            call.insert("action_type".to_string(), action_type.clone());
            let parsed = serde_json::from_value::<GraphSearchAction>(call.into());
            assert!(
                parsed.is_ok(),
                "schema action_type {action_type} lacks a described parameter: {parsed:?}"
            );
        }

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
        let result = execute_graph_search(session_id, GraphSearchAction::AstStats { root: None });

        // @step Then the result is a descriptive error message indicating the handler is not available
        assert!(
            result.contains("not available")
                || result.contains("No handler")
                || result.contains("error"),
            "Should return descriptive error: {result}"
        );

        // @step And no panic or crash occurs
        // (execute_graph_search returned a message instead of panicking)
    }

    // ============================================================================
    // Scenario: AstIndex action deserializes without root (backwards compatible)
    // ============================================================================
    #[test]
    fn test_ast_index_deserializes_without_root() {
        // @step Given a JSON payload for ast_index with no root field
        let json = r#"{"action_type":"ast_index"}"#;

        // @step When the action is deserialized
        let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

        // @step Then it should succeed with root set to None
        assert!(
            result.is_ok(),
            "AstIndex should parse without root: {:?}",
            result.err()
        );
        if let Ok(GraphSearchAction::AstIndex { root, .. }) = result {
            assert!(root.is_none(), "Root should be None when omitted");
        } else {
            panic!("Expected AstIndex variant, got: {:?}", result.unwrap());
        }
    }

    // ============================================================================
    // Scenario: AstIndex action deserializes with explicit root
    // ============================================================================
    #[test]
    fn test_ast_index_deserializes_with_root() {
        // @step Given a JSON payload for ast_index with root "tmp/my-repo"
        let json = r#"{"action_type":"ast_index","root":"tmp/my-repo"}"#;

        // @step When the action is deserialized
        let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

        // @step Then it should succeed with root set to "tmp/my-repo"
        assert!(
            result.is_ok(),
            "AstIndex should parse with root: {:?}",
            result.err()
        );
        if let Ok(GraphSearchAction::AstIndex { root, .. }) = result {
            assert_eq!(
                root.as_deref(),
                Some("tmp/my-repo"),
                "Root should match provided value"
            );
        } else {
            panic!("Expected AstIndex variant, got: {:?}", result.unwrap());
        }
    }

    // ============================================================================
    // Scenario: AstIndex action dispatches through handler with root
    // ============================================================================
    #[test]
    fn test_ast_index_dispatches_with_root() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'ast_index' and root 'tmp/repo'
        let result = execute_graph_search(
            session_id,
            GraphSearchAction::AstIndex {
                root: Some("tmp/repo".to_string()),
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

        // @step And no error is returned
        assert!(
            !result.contains("error"),
            "Should not contain error: {result}"
        );

        // Cleanup
        set_graph_search_handler(session_id, None);
    }

    // ============================================================================
    // Scenario: AstIndex action dispatches through handler without root
    // ============================================================================
    #[test]
    fn test_ast_index_dispatches_without_root() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'ast_index' and no root
        let result = execute_graph_search(
            session_id,
            GraphSearchAction::AstIndex {
                root: None,
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
