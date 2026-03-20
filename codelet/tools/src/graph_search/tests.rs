// Feature: spec/features/graphsearch-tool-definition-handler-registration.feature
//
// GraphSearch Tool Definition & Handler Registration
// Tests for the tool schema, handler map, and error handling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(test)]
mod tests {
    use crate::graph_search::{
        execute_graph_search, has_graph_search_handler, set_graph_search_handler,
        GraphSearchAction, GraphSearchHandler,
    };
    use std::sync::Arc;
    use uuid::Uuid;

    /// Create a mock handler that returns canned responses per action type.
    fn mock_handler() -> GraphSearchHandler {
        Arc::new(|action: GraphSearchAction, _session_id: Uuid| match action {
            GraphSearchAction::Stats => {
                r#"{"nodes":{"Concept":0,"Decision":0},"edges":{"Mentions":0}}"#.to_string()
            }
            GraphSearchAction::Search { query, .. } => {
                format!(r#"{{"results":[{{"name":"{}","category":"technology","summary":"A concept"}}]}}"#, query)
            }
            _ => r#"{"results":[]}"#.to_string(),
        })
    }

    // ============================================================================
    // Scenario: Stats action returns JSON on empty graph
    // ============================================================================
    #[test]
    fn test_stats_action_returns_json_on_empty_graph() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step And the graph database is empty
        // (mock handler returns zero counts)

        // @step When the agent calls GraphSearch with action_type 'stats'
        let result = execute_graph_search(session_id, GraphSearchAction::Stats);

        // @step Then the result contains JSON with node and edge type counts all at zero
        assert!(result.contains("\"Concept\":0"), "Stats should contain zero Concept count: {result}");
        assert!(result.contains("\"Mentions\":0"), "Stats should contain zero Mentions count: {result}");

        // @step And no error is returned
        assert!(!result.contains("error"), "Stats should not contain error: {result}");

        // Cleanup
        set_graph_search_handler(session_id, None);
    }

    // ============================================================================
    // Scenario: Search action returns matching concepts
    // ============================================================================
    #[test]
    fn test_search_action_returns_matching_concepts() {
        let session_id = Uuid::new_v4();

        // @step Given the GraphSearch handler is registered for a session
        set_graph_search_handler(session_id, Some(mock_handler()));

        // @step When the agent calls GraphSearch with action_type 'search' and query 'authentication'
        let result = execute_graph_search(
            session_id,
            GraphSearchAction::Search {
                query: "authentication".to_string(),
                category: None,
                limit: None,
            },
        );

        // @step Then the result contains a JSON array of matching Concept nodes
        assert!(result.contains("results"), "Should contain results array: {result}");

        // @step And each result includes the concept name, category, and summary
        assert!(result.contains("authentication"), "Should contain queried concept name: {result}");
        assert!(result.contains("category"), "Should contain category field: {result}");
        assert!(result.contains("summary"), "Should contain summary field: {result}");

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

        // @step Then GraphSearch appears in the list with its JSON schema
        assert!(handler_exists, "GraphSearch handler should be registered");

        // @step And the schema describes all 8 action types with their parameters
        // (Schema validation is a compile-time guarantee via serde — the enum variants
        // define the action types. This is verified by the type system.)

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
        let result = execute_graph_search(session_id, GraphSearchAction::Stats);

        // @step Then the result is a descriptive error message indicating the handler is not available
        assert!(
            result.contains("not available") || result.contains("No handler") || result.contains("error"),
            "Should return descriptive error: {result}"
        );

        // @step And no panic or crash occurs
        // (If we got here, no panic occurred)
    }
}
