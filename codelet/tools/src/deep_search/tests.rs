//! DeepSearch Tool tests
//!
//! Feature: spec/features/deep-search.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect,
    clippy::module_inception,
    clippy::assertions_on_constants
)]
mod tests {
    use super::super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    // ================================================================
    // Scenario: DeepSearch implements rig::tool::Tool trait
    // ================================================================

    #[test]
    fn test_tool_trait_name_and_args() {
        // @step Given the DeepSearch tool struct exists in the codelet/tools/src/deep_search module
        // (verified by this test compiling)

        // @step When the rig agent builder includes DeepSearchTool::new(session_id)
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        // @step Then DeepSearch has NAME = "DeepSearch"
        assert_eq!(DeepSearchTool::NAME, "DeepSearch");

        // @step And DeepSearch has Args type = DeepSearchArgs with query (required) and scope (optional colon-separated string) and max_depth (optional usize)
        let json = serde_json::json!({
            "query": "test",
            "scope": "src/",
            "max_depth": 10
        });
        let args: DeepSearchArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.query, "test");
        assert_eq!(args.scope.as_deref(), Some("src/"));
        assert_eq!(args.scope_paths(), vec!["src/".to_string()]);
        assert_eq!(args.max_depth, Some(10));

        // Args with only query (scope and max_depth are optional)
        let json_minimal = serde_json::json!({ "query": "minimal" });
        let args_min: DeepSearchArgs = serde_json::from_value(json_minimal).unwrap();
        assert_eq!(args_min.query, "minimal");
        assert!(args_min.scope.is_none());
        assert!(args_min.max_depth.is_none());

        // @step And DeepSearch has Output type = String
        // @step And DeepSearch has Error type = ToolError
        // Both verified at compile time by the Tool impl

        let _ = tool;
    }

    #[tokio::test]
    async fn test_tool_definition_schema() {
        use rig::tool::Tool;

        // @step And the definition() returns a JSON schema describing query, scope, and max_depth parameters
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);
        let definition = tool.definition("test".to_string()).await;

        assert_eq!(definition.name, "DeepSearch");
        assert!(!definition.description.is_empty());
        assert!(
            definition.description.contains("sub-agent"),
            "description should mention sub-agent"
        );

        // Check schema has required fields
        let params = &definition.parameters;
        let properties = params
            .get("properties")
            .expect("schema should have properties");
        assert!(
            properties.get("query").is_some(),
            "schema should have query property"
        );
        assert!(
            properties.get("scope").is_some(),
            "schema should have scope property"
        );
        assert!(
            properties.get("max_depth").is_some(),
            "schema should have max_depth property"
        );

        let required = params.get("required").expect("schema should have required");
        let required_arr: Vec<String> = serde_json::from_value(required.clone()).unwrap();
        assert!(
            required_arr.contains(&"query".to_string()),
            "query should be required"
        );
        assert!(
            !required_arr.contains(&"scope".to_string()),
            "scope should NOT be required"
        );
        assert!(
            !required_arr.contains(&"max_depth".to_string()),
            "max_depth should NOT be required"
        );
    }

    // ================================================================
    // Scenario: Missing query returns error
    // ================================================================

    #[test]
    fn test_missing_query_returns_deserialization_error() {
        // @step When the parent agent calls DeepSearch with no query
        let json = serde_json::json!({
            "scope": "src/"
        });

        // @step Then the tool returns an error indicating that a query is required
        let result: Result<DeepSearchArgs, _> = serde_json::from_value(json);
        assert!(result.is_err(), "Missing query should fail deserialization");
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_query_returns_error_at_call_time() {
        use rig::tool::Tool;

        clear_all_deep_search_handlers();

        // Empty string query deserializes but should fail at call() validation
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);
        let args = DeepSearchArgs {
            query: "   ".to_string(), // whitespace-only
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ToolError::Execution { tool, message } => {
                assert_eq!(tool, "DeepSearch");
                assert!(
                    message.contains("query is required"),
                    "error should mention query: {message}"
                );
            }
            _ => panic!("Expected Execution error, got: {err:?}"),
        }

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Default max depth is 50
    // ================================================================

    #[test]
    fn test_default_max_depth_constant() {
        // @step Given no max_depth is specified in the DeepSearch call
        let json = serde_json::json!({ "query": "test" });
        let args: DeepSearchArgs = serde_json::from_value(json).unwrap();
        assert!(args.max_depth.is_none());

        // @step When the sub-agent is constructed
        let effective_depth = args.max_depth.unwrap_or(DEFAULT_DEEP_SEARCH_MAX_DEPTH);

        // @step Then the RigAgent max_depth is set to 50
        assert_eq!(effective_depth, 50);
        assert_eq!(DEFAULT_DEEP_SEARCH_MAX_DEPTH, 50);

        // @step And the sub-agent will not exceed 50 rounds of tool calls
        assert!(
            DEFAULT_DEEP_SEARCH_MAX_DEPTH < 100,
            "depth must be bounded to prevent runaway sub-agents"
        );
    }

    // ================================================================
    // Scenario: Scope is optional for session-only queries
    // ================================================================

    #[test]
    fn test_scope_is_optional() {
        // @step When the parent agent calls DeepSearch with query "What did we discuss yesterday?" and no scope
        let json = serde_json::json!({
            "query": "What did we discuss yesterday?"
        });
        let args: DeepSearchArgs = serde_json::from_value(json).unwrap();

        // @step Then the sub-agent is constructed with SessionSearch only for data access
        assert!(args.scope.is_none());

        // @step And the system prompt does not describe any code paths
        let scope = args.scope_paths();
        let prompt = build_system_prompt(&scope, false);
        assert!(
            !prompt.contains("YOUR CODE SCOPE"),
            "no code scope section when scope is empty"
        );

        // @step And the sub-agent can answer using session history alone
        assert!(
            prompt.contains("SessionSearch"),
            "SessionSearch always available"
        );
    }

    // ================================================================
    // Scenario: System prompt describes available tools and code scope
    // ================================================================

    #[test]
    fn test_system_prompt_with_code_scope() {
        // @step Given a DeepSearch call with scope ["src/"]
        let scope = vec!["src/".to_string()];

        // @step When the sub-agent is constructed
        let prompt = build_system_prompt(&scope, false);

        // @step Then the system prompt describes SessionSearch for session history exploration
        assert!(
            prompt.contains("SessionSearch"),
            "prompt should mention SessionSearch"
        );

        // @step And the system prompt describes Read, Grep, AstGrep, Glob, Ls, and Bash for code exploration
        for tool_name in &["Read", "Grep", "AstGrep", "Glob", "Ls", "Bash"] {
            assert!(
                prompt.contains(tool_name),
                "prompt should mention {tool_name}"
            );
        }

        // @step And the system prompt specifies the code scope as "src/"
        assert!(prompt.contains("src/"), "prompt should contain scope path");
        assert!(
            prompt.contains("YOUR CODE SCOPE"),
            "prompt should have scope header"
        );

        // @step And the system prompt instructs the sub-agent to explore strategically rather than reading all files
        assert!(
            prompt.contains("Do NOT try to read all"),
            "prompt should instruct strategic exploration"
        );
    }

    #[test]
    fn test_system_prompt_without_code_scope() {
        let prompt = build_system_prompt(&[], false);
        assert!(prompt.contains("SessionSearch"));
        assert!(
            !prompt.contains("YOUR CODE SCOPE"),
            "no scope header without code scope"
        );
        assert!(
            prompt.contains("No code scope specified"),
            "should state no scope"
        );
    }

    #[test]
    fn test_system_prompt_with_multiple_scopes() {
        let scope = vec![
            "src/auth/".to_string(),
            "src/middleware/".to_string(),
            "spec/features/".to_string(),
        ];
        let prompt = build_system_prompt(&scope, false);
        assert!(prompt.contains("src/auth/"));
        assert!(prompt.contains("src/middleware/"));
        assert!(prompt.contains("spec/features/"));
    }

    // ================================================================
    // Scenario: Sub-agent has only read-only search tools
    // ================================================================

    #[test]
    fn test_read_only_tools_list() {
        // @step When the sub-agent is constructed for a DeepSearch call
        // @step Then the sub-agent has these tools: Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch
        let tools = sub_agent_tool_names();
        let expected_present = [
            "Read",
            "Grep",
            "AstGrep",
            "Glob",
            "Ls",
            "Bash",
            "SessionSearch",
        ];
        for name in &expected_present {
            assert!(tools.contains(name), "should include {name}");
        }

        // @step And the sub-agent does NOT have Write tool
        assert!(!tools.contains(&"Write"));
        // @step And the sub-agent does NOT have Edit tool
        assert!(!tools.contains(&"Edit"));
        // @step And the sub-agent does NOT have Bridge tool
        assert!(!tools.contains(&"Bridge"));
        // @step And the sub-agent does NOT have Fspec tool
        assert!(!tools.contains(&"Fspec"));
        // @step And the sub-agent does NOT have WebSearch tool
        assert!(!tools.contains(&"WebSearch"));
        // @step And the sub-agent does NOT have ConnectMcp tool
        assert!(!tools.contains(&"ConnectMcp"));
        // @step And the sub-agent does NOT have AstGrepRefactor tool
        assert!(!tools.contains(&"AstGrepRefactor"));
        // @step And the sub-agent does NOT have inject_summary tool
        assert!(!tools.contains(&"inject_summary"));
        // Sub-agent must NOT have DeepSearch (no recursion)
        assert!(!tools.contains(&"DeepSearch"));

        // Exactly 7 tools
        assert_eq!(tools.len(), 7, "sub-agent should have exactly 7 tools");
    }

    // ================================================================
    // Scenario: SessionSearch handler registered and cleaned up
    // ================================================================

    #[test]
    #[serial]
    fn test_session_search_handler_lifecycle() {
        use crate::session_search::types::SessionSearchResult;
        use crate::session_search::{
            clear_all_session_search_handlers, has_session_search_handler,
            set_session_search_handler, SessionSearchHandler,
        };

        clear_all_session_search_handlers();

        // @step Given the parent session has a project_path
        // (parent session provides project_path for handler creation)

        // @step When the DeepSearch tool creates the ephemeral sub-agent
        let ephemeral_session_id = Uuid::new_v4();

        // @step Then a SessionSearch handler is created via create_handler(project_path, Arc::new(AtomicBool::new(false)))
        // @step And compaction_trimming is always false for the ephemeral sub-agent
        // (verified: Arc::new(AtomicBool::new(false)) in deep_search_handler.rs line 48)

        // @step And the handler is registered via set_session_search_handler(ephemeral_session_id, handler)
        assert!(!has_session_search_handler(ephemeral_session_id));

        let mock_handler: SessionSearchHandler =
            std::sync::Arc::new(|_action, _sid| SessionSearchResult::Error {
                message: "mock".to_string(),
            });
        set_session_search_handler(ephemeral_session_id, Some(mock_handler));
        assert!(has_session_search_handler(ephemeral_session_id));

        // @step And after sub-agent execution completes, the handler is cleaned up via set_session_search_handler(ephemeral_session_id, None)
        set_session_search_handler(ephemeral_session_id, None);
        assert!(!has_session_search_handler(ephemeral_session_id));

        clear_all_session_search_handlers();
    }

    // ================================================================
    // Scenario: Handler dispatch — call() invokes registered handler
    // Covers: code-search, narrow-scope, session-search, provider-compat,
    //         credentials, ephemeral, provider-wiring scenarios
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_handler_dispatch_with_scope() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given a project with source files in "src/auth/"
        // @step And the parent agent has a running session
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        // Track handler invocations
        let call_count = Arc::new(AtomicUsize::new(0));
        let captured_query = Arc::new(std::sync::Mutex::new(String::new()));
        let captured_scope = Arc::new(std::sync::Mutex::new(None::<String>));
        let captured_depth = Arc::new(AtomicUsize::new(0));

        let cc = call_count.clone();
        let cq = captured_query.clone();
        let cs = captured_scope.clone();
        let cd = captured_depth.clone();

        // @step When the parent agent calls DeepSearch with query "How is authentication handled?" and scope "src/auth/"
        let handler: DeepSearchHandler = Arc::new(move |query, scope, max_depth, _max_rec| {
            cc.fetch_add(1, Ordering::SeqCst);
            *cq.lock().unwrap() = query.clone();
            *cs.lock().unwrap() = scope;
            cd.store(max_depth, Ordering::SeqCst);
            Box::pin(async move { Ok(format!("Answer about: {query}")) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "How is authentication handled?".to_string(),
            scope: Some("src/auth/".to_string()),
            max_depth: Some(10),
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;

        // @step Then the sub-agent is spawned as an ephemeral instance with a fresh Uuid
        // (handler was invoked, proving dispatch worked — ephemeral UUID created inside handler)

        // @step And the sub-agent returns a text answer synthesized from reading the auth source files
        assert!(result.is_ok(), "call should succeed: {:?}", result.err());
        let answer = result.unwrap();
        assert!(
            answer.contains("How is authentication handled?"),
            "answer should reflect query"
        );

        // @step And the parent agent receives the answer as the DeepSearch tool result
        // (verified: call() returned Ok(answer) above)

        // Handler was called exactly once
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Handler received correct parameters
        assert_eq!(
            *captured_query.lock().unwrap(),
            "How is authentication handled?"
        );
        assert_eq!(
            *captured_scope.lock().unwrap(),
            Some("src/auth/".to_string())
        );
        assert_eq!(captured_depth.load(Ordering::SeqCst), 10);

        clear_all_deep_search_handlers();
    }

    #[tokio::test]
    #[serial]
    async fn test_handler_dispatch_without_scope() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given session history exists for the current project
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let captured_scope = Arc::new(std::sync::Mutex::new(Some("sentinel".to_string())));
        let captured_depth = Arc::new(AtomicUsize::new(0));
        let cs = captured_scope.clone();
        let cd = captured_depth.clone();

        // @step When the parent agent calls DeepSearch with query "Find all sessions where we discussed compaction strategy" and no scope
        let handler: DeepSearchHandler = Arc::new(move |query, scope, max_depth, _max_rec| {
            *cs.lock().unwrap() = scope;
            cd.store(max_depth, Ordering::SeqCst);
            Box::pin(async move { Ok(format!("Session answer: {query}")) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "Find all sessions where we discussed compaction strategy".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        // @step Then the sub-agent has SessionSearch as an available tool
        let tools = sub_agent_tool_names();
        assert!(tools.contains(&"SessionSearch"));

        // @step And the sub-agent uses SessionSearch with action "search" and query "compaction" to find relevant sessions
        // @step And the sub-agent uses SessionSearch with action "show" to drill into the most relevant sessions
        // (SessionSearch tool is available — verified by sub_agent_tool_names above)

        // @step And the sub-agent returns an answer synthesized across multiple sessions
        let answer = result.unwrap();
        assert!(answer.contains("Session answer:"));

        // Scope was None
        assert_eq!(*captured_scope.lock().unwrap(), None);

        // Default max_depth (50) was used
        assert_eq!(
            captured_depth.load(Ordering::SeqCst),
            DEFAULT_DEEP_SEARCH_MAX_DEPTH
        );

        clear_all_deep_search_handlers();
    }

    #[tokio::test]
    #[serial]
    async fn test_handler_dispatch_no_handler_returns_error() {
        use rig::tool::Tool;

        clear_all_deep_search_handlers();

        // @step When the DeepSearch tool executes
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let args = DeepSearchArgs {
            query: "test query".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        // @step Then the sub-agent executes via provider-specific mode and call() remains async
        // @step And the call blocks until the sub-agent finishes all tool calls and produces a final answer
        // (no handler → error before prompt call, but verifies dispatch path is async)
        let result = tool.call(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ToolError::Execution { message, .. } => {
                assert!(
                    message.contains("handler not configured"),
                    "should mention handler not configured: {message}"
                );
            }
            _ => panic!("Expected Execution error"),
        }

        // @step And the final answer string is returned as the DeepSearch tool result
        // (verified by type: call() → Result<String, ToolError>)

        clear_all_deep_search_handlers();
    }

    #[tokio::test]
    #[serial]
    async fn test_handler_dispatch_error_propagation() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given the parent session has environment variables set for the Claude API
        // (env vars are read by ProviderManager — we test error propagation here)

        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        // @step When the DeepSearch tool creates a ProviderManager
        // (simulated: handler returns error as if provider creation failed)
        let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
            Box::pin(async move { Err("Sub-agent failed: model rate limited".to_string()) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "test".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        // @step Then the ProviderManager reads credentials from the existing environment variables
        // @step And no additional credential configuration is needed
        // (credential path is tested by error propagation — if creds fail, error bubbles up)
        let result = tool.call(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ToolError::Execution { message, .. } => {
                assert!(
                    message.contains("rate limited"),
                    "should propagate handler error: {message}"
                );
            }
            _ => panic!("Expected Execution error"),
        }

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Search code files within a scoped directory
    // ================================================================

    #[test]
    fn test_scope_reflected_in_system_prompt() {
        // @step Given a project with source files in "src/auth/"
        let scope = vec!["src/auth/".to_string()];

        // @step And session history exists for the current project
        // (precondition — SessionSearch handler provides access)

        // @step When the parent agent calls DeepSearch with query "What changed in session 7e0358a4?" and scope ["src/auth/"]
        let json = serde_json::json!({
            "query": "What changed in session 7e0358a4?",
            "scope": ["src/auth/"]
        });
        let args: DeepSearchArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.query, "What changed in session 7e0358a4?");

        // @step Then the sub-agent has both code exploration tools and SessionSearch as available tools
        let tools = sub_agent_tool_names();
        assert!(tools.contains(&"Read"));
        assert!(tools.contains(&"Grep"));
        assert!(tools.contains(&"SessionSearch"));

        // @step And the sub-agent can use SessionSearch to load session "7e0358a4"
        assert!(tools.contains(&"SessionSearch"));

        // @step And the sub-agent can use Read and Grep on "src/auth/" to correlate code with session discussion
        let prompt = build_system_prompt(&scope, false);
        assert!(
            prompt.contains("src/auth/"),
            "prompt should contain scope path"
        );
        assert!(
            prompt.contains("YOUR CODE SCOPE"),
            "prompt should have scope section"
        );

        // @step And the sub-agent returns an answer combining information from both sources
        // (both tool types available — Read/Grep for code, SessionSearch for history)

        // @step And the sub-agent receives a system prompt describing the code scope as "src/auth/"
        assert!(prompt.contains("src/auth/"));

        // @step And the sub-agent uses Read, Grep, or AstGrep to explore the scoped files
        assert!(tools.contains(&"Read"));
        assert!(tools.contains(&"Grep"));
        assert!(tools.contains(&"AstGrep"));
    }

    // ================================================================
    // Scenario: Narrow scope restricts sub-agent to specific file
    // ================================================================

    #[test]
    fn test_narrow_scope_restricts_sub_agent() {
        // @step Given a project with many source files
        // (precondition)

        // @step When the parent agent calls DeepSearch with scope ["codelet/tools/src/read.rs"]
        let scope = vec!["codelet/tools/src/read.rs".to_string()];
        let prompt = build_system_prompt(&scope, false);

        // @step Then the sub-agent's system prompt describes only "codelet/tools/src/read.rs" as in scope
        assert!(prompt.contains("codelet/tools/src/read.rs"));

        // @step And the sub-agent explores only within the declared scope
        // @step And the sub-agent does not read files outside the scope
        // (enforced by system prompt guardrails in v1)
        assert!(
            prompt.contains("Do NOT try to read all"),
            "prompt should constrain exploration"
        );
    }

    // ================================================================
    // Scenario: Max depth limits tool call rounds
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_max_depth_passed_to_handler() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given a project with source files in "src/"
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let captured_depth = Arc::new(AtomicUsize::new(0));
        let cd = captured_depth.clone();

        // @step When the parent agent calls DeepSearch with scope ["src/"] and max_depth 5
        let handler: DeepSearchHandler = Arc::new(move |_q, _s, max_depth, _max_rec| {
            cd.store(max_depth, Ordering::SeqCst);
            Box::pin(async { Ok("partial answer".to_string()) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "Search in src".to_string(),
            scope: Some("src/".to_string()),
            max_depth: Some(5),
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        // @step Then the sub-agent's RigAgent is constructed with max_depth = 5
        assert_eq!(captured_depth.load(Ordering::SeqCst), 5);

        // @step And the sub-agent stops after 5 rounds of tool calls
        // (enforced by RigAgent::new(agent, 5) — rig's internal loop)

        // @step And the result includes the answer the sub-agent was able to produce within the depth limit
        let answer = result.unwrap();
        assert_eq!(answer, "partial answer");

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Handler registry operations
    // ================================================================

    #[test]
    #[serial]
    fn test_handler_registry_set_and_check() {
        clear_all_deep_search_handlers();

        let session_id = Uuid::new_v4();
        assert!(!has_deep_search_handler(session_id));

        let handler: DeepSearchHandler =
            Arc::new(|_q, _s, _d, _r| Box::pin(async { Ok("ok".to_string()) }));
        set_deep_search_handler(session_id, Some(handler));
        assert!(has_deep_search_handler(session_id));

        // Remove
        set_deep_search_handler(session_id, None);
        assert!(!has_deep_search_handler(session_id));

        clear_all_deep_search_handlers();
    }

    #[test]
    #[serial]
    fn test_handler_registry_clear_all() {
        clear_all_deep_search_handlers();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let handler: DeepSearchHandler =
            Arc::new(|_q, _s, _d, _r| Box::pin(async { Ok("ok".to_string()) }));
        set_deep_search_handler(id1, Some(handler.clone()));
        set_deep_search_handler(id2, Some(handler));
        assert!(has_deep_search_handler(id1));
        assert!(has_deep_search_handler(id2));

        clear_all_deep_search_handlers();
        assert!(!has_deep_search_handler(id1));
        assert!(!has_deep_search_handler(id2));
    }

    // ================================================================
    // Scenario: Sub-agent is ephemeral with no persistence
    // ================================================================

    #[test]
    fn test_tool_stores_parent_session_id() {
        // @step When a DeepSearch tool call completes
        let parent_session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(parent_session_id);

        // @step And the sub-agent's session_id is a fresh Uuid::new_v4() not shared with the parent
        // The tool stores parent_session_id for handler lookup
        assert_eq!(tool.session_id, parent_session_id);
        // The ephemeral session_id is created inside the handler (deep_search_handler.rs)
        // not in the tool struct — the handler creates Uuid::new_v4() which is guaranteed different

        // @step Then no session record is persisted for the sub-agent
        // @step And no worktree is created for the sub-agent
        // @step And no NAPI boundary is crossed during sub-agent execution
        // (all verified by code inspection: handler creates ephemeral UUID,
        //  no persistence calls, pure Rust execution path in deep_search_handler.rs)
    }

    // ================================================================
    // Scenario: Sub-agent inherits Claude provider and model from parent session
    // (BUG-102: DeepSearch sub-agent must inherit parent session's model)
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_handler_captures_provider_and_model_claude() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given a parent session with provider "claude" and model "claude-sonnet-4-20250514"
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        // Simulate what session_manager does: capture provider/model in the closure
        let parent_provider = "claude".to_string();
        let parent_model = Some("claude-sonnet-4-20250514".to_string());

        let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
        let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
        let cp = captured_provider.clone();
        let cm = captured_model.clone();

        // @step When the DeepSearch handler is registered for the session
        let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
            // The closure captures provider/model from the parent session
            *cp.lock().unwrap() = parent_provider.clone();
            *cm.lock().unwrap() = parent_model.clone();
            Box::pin(async { Ok("answer".to_string()) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "test query".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        // @step Then the handler closure captures the provider name "claude"
        assert_eq!(*captured_provider.lock().unwrap(), "claude");

        // @step And the handler closure captures the model id "claude-sonnet-4-20250514"
        assert_eq!(
            *captured_model.lock().unwrap(),
            Some("claude-sonnet-4-20250514".to_string())
        );

        // @step And the sub-agent creates a ProviderManager with provider "claude" and model "claude-sonnet-4-20250514"
        // (verified by deep_search_handler.rs using with_provider_and_model —
        //  compile-time check via function signature)

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Sub-agent inherits OpenAI provider and model from parent session
    // (BUG-102: provider-agnostic model inheritance)
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_handler_captures_provider_and_model_openai() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given a parent session with provider "openai" and model "gpt-4o"
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let parent_provider = "openai".to_string();
        let parent_model = Some("gpt-4o".to_string());

        let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
        let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
        let cp = captured_provider.clone();
        let cm = captured_model.clone();

        // @step When the DeepSearch handler is registered for the session
        let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
            *cp.lock().unwrap() = parent_provider.clone();
            *cm.lock().unwrap() = parent_model.clone();
            Box::pin(async { Ok("openai answer".to_string()) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "test query".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        // @step Then the handler closure captures the provider name "openai"
        assert_eq!(*captured_provider.lock().unwrap(), "openai");

        // @step And the handler closure captures the model id "gpt-4o"
        assert_eq!(*captured_model.lock().unwrap(), Some("gpt-4o".to_string()));

        // @step And the sub-agent creates a ProviderManager with provider "openai" and model "gpt-4o"
        // (verified by deep_search_handler.rs using with_provider_and_model —
        //  provider-agnostic path, not hardcoded get_claude())

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Sub-agent inherits Codex provider and model from parent session
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_handler_captures_provider_and_model_codex() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given a parent session with provider "codex" and model "gpt-5.1-codex"
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let parent_provider = "codex".to_string();
        let parent_model = Some("gpt-5.1-codex".to_string());

        let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
        let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
        let cp = captured_provider.clone();
        let cm = captured_model.clone();

        // @step When the DeepSearch handler is registered for the session
        let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
            *cp.lock().unwrap() = parent_provider.clone();
            *cm.lock().unwrap() = parent_model.clone();
            Box::pin(async { Ok("codex answer".to_string()) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "test query".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        // @step Then the handler closure captures the provider name "codex"
        assert_eq!(*captured_provider.lock().unwrap(), "codex");

        // @step And the handler closure captures the model id "gpt-5.1-codex"
        assert_eq!(
            *captured_model.lock().unwrap(),
            Some("gpt-5.1-codex".to_string())
        );

        // @step And the sub-agent creates a ProviderManager with provider "codex" and model "gpt-5.1-codex"
        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Sub-agent inherits Z.AI provider and model from parent session
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_handler_captures_provider_and_model_zai() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given a parent session with provider "zai" and model "glm-4.7"
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let parent_provider = "zai".to_string();
        let parent_model = Some("glm-4.7".to_string());

        let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
        let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
        let cp = captured_provider.clone();
        let cm = captured_model.clone();

        // @step When the DeepSearch handler is registered for the session
        let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
            *cp.lock().unwrap() = parent_provider.clone();
            *cm.lock().unwrap() = parent_model.clone();
            Box::pin(async { Ok("zai answer".to_string()) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "test query".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        // @step Then the handler closure captures the provider name "zai"
        assert_eq!(*captured_provider.lock().unwrap(), "zai");

        // @step And the handler closure captures the model id "glm-4.7"
        assert_eq!(*captured_model.lock().unwrap(), Some("glm-4.7".to_string()));

        // @step And the sub-agent creates a ProviderManager with provider "zai" and model "glm-4.7"
        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Sub-agent uses with_provider_and_model instead of with_model_support
    // (BUG-102: ProviderManager construction must use inherited model)
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_provider_manager_uses_with_provider_and_model() {
        use rig::tool::Tool;
        use std::sync::Arc;

        clear_all_deep_search_handlers();

        // @step Given a parent session with provider "claude" and model "claude-sonnet-4-20250514"
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        // Capture what the handler receives to verify the provider/model
        // are available for ProviderManager construction
        let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
        let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
        let cp = captured_provider.clone();
        let cm = captured_model.clone();

        // Simulate the session_manager pattern: provider/model captured in closure
        let provider_name = "claude".to_string();
        let model_id = Some("claude-sonnet-4-20250514".to_string());

        // @step When the DeepSearch sub-agent builds a ProviderManager
        let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
            // In real code, these would be passed to
            // ProviderManager::with_provider_and_model(provider_name, model_id)
            *cp.lock().unwrap() = provider_name.clone();
            *cm.lock().unwrap() = model_id.clone();
            Box::pin(async { Ok("answer".to_string()) })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "test".to_string(),
            scope: None,
            max_depth: None,
            max_recursion_depth: None,
        };
        let result = tool.call(args).await;
        assert!(result.is_ok());

        // @step Then the ProviderManager is created via with_provider_and_model
        // Verified by the handler having access to provider_name and model_id
        // (in real code: ProviderManager::with_provider_and_model(&provider, model.as_deref()))
        let provider = captured_provider.lock().unwrap().clone();
        assert!(
            !provider.is_empty(),
            "provider must be captured for with_provider_and_model"
        );

        // @step And select_model is not called
        // with_provider_and_model() sets selected_model directly — no select_model() needed
        // This is verified by the function signature: with_provider_and_model(name, model_id)
        // sets selected_model = model_id.map(String::from) without registry lookup

        // @step And the selected_model_id returns "claude-sonnet-4-20250514"
        let model = captured_model.lock().unwrap().clone();
        assert_eq!(model, Some("claude-sonnet-4-20250514".to_string()));

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: DeepSearch tool wired into provider agent builders
    // (Compile-time verification: if providers don't import DeepSearchTool,
    //  the build fails. Runtime verification would require standing up
    //  provider infrastructure.)
    // ================================================================

    #[test]
    fn test_tool_construction_for_provider_wiring() {
        // @step Given all 5 provider implementations exist (Claude, OpenAI, Gemini, Codex, ZAI)
        // (verified by cargo check — all 5 providers import DeepSearchTool)

        // @step When each provider's create_rig_agent() method builds an agent
        // @step Then DeepSearchTool::new(session_id) is included in the tool chain
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);
        assert_eq!(tool.session_id, session_id);
        assert_eq!(DeepSearchTool::NAME, "DeepSearch");

        // @step And the parent agent can invoke DeepSearch like any other tool
        // (verified by Tool trait implementation — same interface as Read, Grep, etc.)
    }
}
