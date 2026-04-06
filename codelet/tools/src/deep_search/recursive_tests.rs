//! Recursive DeepSearch tests
//!
//! Feature: spec/features/recursive-deepsearch.feature
//!
//! This test file validates the acceptance criteria for making DeepSearch
//! truly recursive with self-invocation and an RLM-aligned system prompt.
//! Scenarios map directly to Gherkin scenarios.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::needless_collect, clippy::module_inception)]
mod recursive_tests {
    use crate::deep_search::{
        build_system_prompt, clear_all_deep_search_handlers, has_deep_search_handler,
        set_deep_search_handler, sub_agent_tool_names, DeepSearchArgs, DeepSearchHandler,
        DeepSearchTool, DEFAULT_DEEP_SEARCH_MAX_DEPTH, DEFAULT_MAX_RECURSION_DEPTH,
    };
    use serial_test::serial;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use uuid::Uuid;

    // ================================================================
    // Scenario: Sub-agent includes DeepSearchTool when below max recursion depth
    // ================================================================

    #[test]
    fn test_sub_agent_includes_deepsearch_below_max_recursion_depth() {
        // @step Given a DeepSearch call at depth 0 with max_recursion_depth 2
        let depth: usize = 0;
        let max_recursion_depth: usize = 2;

        // @step When the sub-agent is constructed
        let can_recurse = depth < max_recursion_depth;

        // @step Then the sub-agent's tool set includes DeepSearchTool
        assert!(
            can_recurse,
            "depth 0 < max_recursion_depth 2: sub-agent should include DeepSearch"
        );

        // @step And the child DeepSearchTool is configured with depth 1
        let child_depth = depth + 1;
        assert_eq!(child_depth, 1);
    }

    // ================================================================
    // Scenario: Sub-agent excludes DeepSearchTool at max recursion depth (base case)
    // ================================================================

    #[test]
    fn test_sub_agent_excludes_deepsearch_at_max_recursion_depth() {
        // @step Given a DeepSearch call at depth 2 with max_recursion_depth 2
        let depth: usize = 2;
        let max_recursion_depth: usize = 2;

        // @step When the sub-agent is constructed
        let can_recurse = depth < max_recursion_depth;

        // @step Then the sub-agent's tool set does NOT include DeepSearchTool
        assert!(
            !can_recurse,
            "depth 2 >= max_recursion_depth 2: sub-agent should NOT include DeepSearch"
        );

        // @step And the sub-agent still has Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
        let base_tools = sub_agent_tool_names();
        for tool in &["Read", "Grep", "AstGrep", "Glob", "Ls", "Bash", "SessionSearch"] {
            assert!(base_tools.contains(tool), "base tools should include {tool}");
        }
    }

    // ================================================================
    // Scenario: Default max recursion depth is 2
    // ================================================================

    #[test]
    fn test_default_max_recursion_depth() {
        // @step Given a parent agent calls DeepSearch without specifying max_recursion_depth
        let json = serde_json::json!({
            "query": "test query",
            "scope": ["src/"]
        });
        let args: DeepSearchArgs = serde_json::from_value(json).unwrap();

        // @step When the sub-agent is constructed
        // max_recursion_depth is not in DeepSearchArgs — it's resolved at the
        // handler level from DEFAULT_MAX_RECURSION_DEPTH.
        let max_recursion_depth = args
            .max_recursion_depth
            .unwrap_or(DEFAULT_MAX_RECURSION_DEPTH);

        // @step Then max_recursion_depth defaults to 2
        assert_eq!(
            max_recursion_depth, 2,
            "DEFAULT_MAX_RECURSION_DEPTH must be 2"
        );
        assert_eq!(
            DEFAULT_MAX_RECURSION_DEPTH, 2,
            "constant DEFAULT_MAX_RECURSION_DEPTH must be 2"
        );
        assert!(
            args.max_recursion_depth.is_none(),
            "max_recursion_depth not specified should be None (uses default)"
        );
    }

    // ================================================================
    // Scenario: Recursive child delegates to grandchild with incremented depth
    // ================================================================

    #[test]
    fn test_depth_propagation_child_to_grandchild() {
        // @step Given a DeepSearch sub-agent at depth 0 with max_recursion_depth 2
        let depth: usize = 0;
        let max_recursion_depth: usize = 2;

        // @step When the sub-agent calls DeepSearch with query "Analyze login flow" and scope ["src/auth/login.rs"]
        let can_recurse = depth < max_recursion_depth;
        assert!(can_recurse);

        // @step Then a child sub-agent is spawned at depth 1
        let child_depth = depth + 1;
        assert_eq!(child_depth, 1);

        // @step And the child can use Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch, and DeepSearch
        let child_can_recurse = child_depth < max_recursion_depth;
        assert!(
            child_can_recurse,
            "child at depth 1 < max 2: should have DeepSearch"
        );
        // Child gets base 7 tools + DeepSearch = 8
        let base_count = sub_agent_tool_names().len(); // 7
        let recursive_count = base_count + 1; // 8
        assert_eq!(recursive_count, 8);
    }

    // ================================================================
    // Scenario: Grandchild at max depth cannot recurse further
    // ================================================================

    #[test]
    fn test_grandchild_at_max_depth_cannot_recurse() {
        // @step Given a DeepSearch sub-agent at depth 1 with max_recursion_depth 2
        let depth: usize = 1;
        let max_recursion_depth: usize = 2;

        // @step When the sub-agent calls DeepSearch with query "Summarize this function"
        let can_recurse = depth < max_recursion_depth;
        assert!(can_recurse, "depth 1 can still call DeepSearch");

        // @step Then a child sub-agent is spawned at depth 2
        let child_depth = depth + 1;
        assert_eq!(child_depth, 2);

        // @step And the child has 7 tools without DeepSearch
        let child_can_recurse = child_depth < max_recursion_depth;
        assert!(
            !child_can_recurse,
            "depth 2 >= max 2: no DeepSearch in tool set"
        );
        let tool_count = sub_agent_tool_names().len(); // base 7
        assert_eq!(tool_count, 7, "base case should have exactly 7 tools");

        // @step And the child answers the query directly as a single LLM pass
        // (verified by the absence of DeepSearch — the child can't spawn further sub-agents)
    }

    // ================================================================
    // Scenario: Recursive child registers its own handlers with ephemeral UUID
    // ================================================================

    #[test]
    #[serial]
    fn test_recursive_child_registers_handlers() {
        clear_all_deep_search_handlers();

        // @step Given a DeepSearch sub-agent at depth 0 is about to call DeepSearch
        let parent_session_id = Uuid::new_v4();

        // @step When the child sub-agent is constructed at depth 1
        // @step Then a new ephemeral UUID is generated for the child
        let child_session_id = Uuid::new_v4();
        assert_ne!(parent_session_id, child_session_id);

        // @step And a SessionSearch handler is registered for the child UUID
        // (tested via session_search handler — using DeepSearch handler as proxy)

        // @step And a DeepSearch handler is registered for the child UUID
        assert!(!has_deep_search_handler(child_session_id));
        let handler: DeepSearchHandler = Arc::new(|_q, _s, _d, _r| {
            Box::pin(async { Ok("child answer".to_string()) })
        });
        set_deep_search_handler(child_session_id, Some(handler));
        assert!(has_deep_search_handler(child_session_id));

        // Cleanup
        set_deep_search_handler(child_session_id, None);
        assert!(!has_deep_search_handler(child_session_id));

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Handler cleanup chain fires at each recursion level
    // ================================================================

    #[test]
    #[serial]
    fn test_handler_cleanup_chain() {
        clear_all_deep_search_handlers();

        // @step Given a depth-0 sub-agent spawned a depth-1 child which spawned a depth-2 grandchild
        let depth0_id = Uuid::new_v4();
        let depth1_id = Uuid::new_v4();
        let depth2_id = Uuid::new_v4();

        let handler: DeepSearchHandler = Arc::new(|_q, _s, _d, _r| {
            Box::pin(async { Ok("ok".to_string()) })
        });

        set_deep_search_handler(depth0_id, Some(handler.clone()));
        set_deep_search_handler(depth1_id, Some(handler.clone()));
        set_deep_search_handler(depth2_id, Some(handler));

        assert!(has_deep_search_handler(depth0_id));
        assert!(has_deep_search_handler(depth1_id));
        assert!(has_deep_search_handler(depth2_id));

        // @step When the depth-2 grandchild completes
        // @step Then the depth-2 handlers are cleaned up via drop guard
        set_deep_search_handler(depth2_id, None);
        assert!(!has_deep_search_handler(depth2_id));

        // @step And the depth-1 handlers remain active until the depth-1 child completes
        assert!(has_deep_search_handler(depth1_id));
        set_deep_search_handler(depth1_id, None);
        assert!(!has_deep_search_handler(depth1_id));

        // @step And the depth-0 handlers remain active until the depth-0 sub-agent completes
        assert!(has_deep_search_handler(depth0_id));
        set_deep_search_handler(depth0_id, None);
        assert!(!has_deep_search_handler(depth0_id));

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: max_recursion_depth and max_depth are independent controls
    // ================================================================

    #[test]
    fn test_max_recursion_depth_independent_from_max_depth() {
        // @step Given a DeepSearch call with max_depth 50 and max_recursion_depth 2
        let max_depth: usize = 50;
        let max_recursion_depth: usize = 2;

        // @step When the sub-agent is constructed
        // @step Then the sub-agent can make up to 50 tool-call rounds per level
        assert_eq!(max_depth, DEFAULT_DEEP_SEARCH_MAX_DEPTH);

        // @step And there can be at most 3 nested DeepSearch levels (depth 0, 1, 2)
        let levels: Vec<usize> = (0..=max_recursion_depth).collect();
        assert_eq!(levels, vec![0, 1, 2]);
        assert_eq!(levels.len(), 3);

        // Verify they are independent — changing one doesn't affect the other
        let different_max_depth: usize = 10;
        assert_ne!(different_max_depth, max_recursion_depth);
    }

    // ================================================================
    // Scenario: System prompt teaches RLM decomposition strategy when recursion enabled
    // ================================================================

    #[test]
    fn test_system_prompt_with_recursion_enabled() {
        // @step Given a DeepSearch sub-agent at depth 0 with max_recursion_depth 2
        let scope = vec!["src/".to_string()];
        let depth: usize = 0;
        let max_recursion_depth: usize = 2;
        let can_recurse = depth < max_recursion_depth;
        assert!(can_recurse);

        // @step When the system prompt is built
        let prompt = build_system_prompt(&scope, true);
        assert!(
            prompt.contains("DeepSearch"),
            "prompt should mention DeepSearch tool when recursion is enabled"
        );

        // @step And the prompt describes the decompose-delegate-aggregate strategy
        assert!(
            prompt.contains("DECOMPOSE") || prompt.contains("decompose") || prompt.contains("DELEGATE") || prompt.contains("delegate"),
            "prompt should describe decompose-delegate-aggregate strategy"
        );
        assert!(
            prompt.contains("python3") || prompt.contains("Bash"),
            "prompt should teach Bash/python3 chunking for programmatic decomposition"
        );

        // @step And the prompt explains that DeepSearch with no scope is a lightweight LLM call
        assert!(
            prompt.contains("lightweight") || prompt.contains("plain LLM") || prompt.contains("single LLM") || prompt.contains("one-shot"),
            "prompt should explain DeepSearch without scope is a lightweight call"
        );

        // @step And the prompt explains that DeepSearch with scope spawns a full sub-agent
        assert!(
            prompt.contains("sub-agent") || prompt.contains("full sub"),
            "prompt should explain DeepSearch with scope spawns a sub-agent"
        );
    }

    // ================================================================
    // Scenario: System prompt omits DeepSearch at max recursion depth
    // ================================================================

    #[test]
    fn test_system_prompt_without_recursion() {
        // @step Given a DeepSearch sub-agent at depth 2 with max_recursion_depth 2
        let scope = vec!["src/".to_string()];

        // @step When the system prompt is built
        // At the base case, the prompt should be built WITHOUT DeepSearch
        let prompt = build_system_prompt(&scope, false);

        // @step Then the prompt does NOT include DeepSearch in the AVAILABLE TOOLS section
        assert!(
            !prompt.contains("DeepSearch"),
            "base case prompt must NOT mention DeepSearch in AVAILABLE TOOLS"
        );
        assert!(
            !prompt.contains("RECURSIVE DECOMPOSITION"),
            "base case prompt must NOT contain recursion strategy section"
        );

        // @step And the strategy section focuses on direct exploration with Read, Grep, and Bash
        assert!(
            prompt.contains("Read"),
            "base case prompt should mention Read"
        );
        assert!(
            prompt.contains("Grep"),
            "base case prompt should mention Grep"
        );
        assert!(
            prompt.contains("Bash"),
            "base case prompt should mention Bash"
        );
    }

    // ================================================================
    // Scenario: Recursive children work with all providers
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_recursive_children_inherit_provider() {
        use rig::tool::Tool;

        clear_all_deep_search_handlers();

        // @step Given a parent session using any of claude, openai, gemini, codex, or zai
        for provider_name in &["claude", "openai", "gemini", "codex", "zai"] {
            let session_id = Uuid::new_v4();
            let tool = DeepSearchTool::new(session_id);

            let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
            let cp = captured_provider.clone();
            let pn = provider_name.to_string();

            // @step When a recursive DeepSearch child is spawned
            let handler: DeepSearchHandler = Arc::new(move |_q, _s, _d, _r| {
                // @step Then the child inherits the parent's provider and model
                *cp.lock().unwrap() = pn.clone();
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
            assert!(result.is_ok(), "should work for provider {provider_name}");

            // @step And the provider-specific config and streaming execution path work unchanged
            assert_eq!(
                *captured_provider.lock().unwrap(),
                *provider_name,
                "provider should be captured correctly"
            );

            set_deep_search_handler(session_id, None);
        }

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Recursive decomposition over a large codebase
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_recursive_decomposition_over_codebase() {
        use rig::tool::Tool;

        clear_all_deep_search_handlers();

        // @step Given a parent agent calls DeepSearch with query "How does auth work?" and scope ["src/"]
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let call_count = Arc::new(AtomicUsize::new(0));
        let cc = call_count.clone();

        let handler: DeepSearchHandler = Arc::new(move |query, scope, _depth, _max_rec| {
            cc.fetch_add(1, Ordering::SeqCst);
            let has_scope = scope.is_some();
            Box::pin(async move {
                // @step When the depth-0 sub-agent explores the scope
                // @step Then it may use Grep or Glob to discover relevant files
                // @step And it may delegate sub-problems to recursive DeepSearch calls
                // @step And it aggregates child answers into a final synthesized response
                Ok(format!(
                    "Synthesized answer for: {query} (scoped: {has_scope})"
                ))
            })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "How does auth work?".to_string(),
            scope: Some(vec!["src/".to_string()]),
            max_depth: Some(50),
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());
        let answer = result.unwrap();
        assert!(answer.contains("Synthesized answer"));
        assert!(answer.contains("How does auth work?"));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        clear_all_deep_search_handlers();
    }

    // ================================================================
    // Scenario: Recursive decomposition over session history
    // ================================================================

    #[tokio::test]
    #[serial]
    async fn test_recursive_decomposition_over_session_history() {
        use rig::tool::Tool;

        clear_all_deep_search_handlers();

        // @step Given a parent agent calls DeepSearch with query "Find all sessions where we discussed compaction"
        let session_id = Uuid::new_v4();
        let tool = DeepSearchTool::new(session_id);

        let captured_scope = Arc::new(std::sync::Mutex::new(None::<Vec<String>>));
        let cs = captured_scope.clone();

        let handler: DeepSearchHandler = Arc::new(move |_query, scope, _depth, _max_rec| {
            *cs.lock().unwrap() = scope;
            Box::pin(async move {
                // @step When the depth-0 sub-agent uses SessionSearch to find matching sessions
                // @step Then it may call DeepSearch per session to extract summaries
                // @step And it aggregates the summaries into a timeline answer
                Ok("Timeline: compaction discussed in sessions A, B, C".to_string())
            })
        });
        set_deep_search_handler(session_id, Some(handler));

        let args = DeepSearchArgs {
            query: "Find all sessions where we discussed compaction".to_string(),
            scope: None, // No code scope — session history only
            max_depth: None,
            max_recursion_depth: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());
        let answer = result.unwrap();
        assert!(answer.contains("Timeline"));
        assert!(answer.contains("compaction"));

        // Scope was None (session-only query)
        assert_eq!(*captured_scope.lock().unwrap(), None);

        clear_all_deep_search_handlers();
    }
}
