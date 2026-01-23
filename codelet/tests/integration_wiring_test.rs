
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration test to verify REFAC-003 and REFAC-004 are fully wired up
//!
//! This test proves:
//! 1. ClaudeProvider implements LlmProvider trait correctly (REFAC-003)
//! 2. RigAgent is accessible and properly configured (REFAC-004)
//! 3. All 7 tools implement rig::tool::Tool (REFAC-004)
//! 4. Everything compiles and is exported properly

use codelet::agent::{RigAgent, DEFAULT_MAX_DEPTH};
use codelet::providers::{ClaudeProvider, LlmProvider};
use codelet::tools::{
    AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, ReadTool, ToolRegistry, WebSearchTool,
    WriteTool,
};

#[test]
fn test_refac_003_claude_provider_is_accessible() {
    // Verify ClaudeProvider implements LlmProvider trait
    fn assert_implements_llm_provider<T: LlmProvider>() {}
    assert_implements_llm_provider::<ClaudeProvider>();

    // Verify provider methods exist (compile-time check)
    fn _verify_provider_methods(provider: &ClaudeProvider) {
        let _ = provider.name();
        let _ = provider.model();
        let _ = provider.context_window();
        let _ = provider.max_output_tokens();
        let _ = provider.supports_caching();
        let _ = provider.supports_streaming();
    }
}

#[test]
fn test_refac_004_rig_agent_is_accessible() {
    // Verify RigAgent is exported from codelet::agent
    // RigAgent is generic over CompletionModel
    fn _verify_rig_agent_exists<M: rig::completion::CompletionModel + 'static>() -> &'static str {
        std::any::type_name::<RigAgent<M>>()
    }
    _verify_rig_agent_exists::<rig::providers::anthropic::completion::CompletionModel>();

    // Verify DEFAULT_MAX_DEPTH constant is exported
    assert_eq!(DEFAULT_MAX_DEPTH, 10, "Default max depth should be 10");

    // Verify RigAgent has expected methods (compile-time check)
    // Note: RigAgent is now generic, so we use a generic function
    fn _verify_rig_agent_methods<M: rig::completion::CompletionModel + 'static>(
        agent: &RigAgent<M>,
    ) -> usize {
        agent.max_depth()
    }
}

#[test]
fn test_refac_004_all_tools_implement_rig_tool_trait() {
    // Verify all 9 tools implement rig::tool::Tool trait (WEB-001 added WebSearchTool)
    fn assert_implements_rig_tool<T: rig::tool::Tool>() {}

    assert_implements_rig_tool::<ReadTool>();
    assert_implements_rig_tool::<WriteTool>();
    assert_implements_rig_tool::<EditTool>();
    assert_implements_rig_tool::<BashTool>();
    assert_implements_rig_tool::<GrepTool>();
    assert_implements_rig_tool::<GlobTool>();
    assert_implements_rig_tool::<AstGrepTool>();
    assert_implements_rig_tool::<WebSearchTool>(); // WEB-001: Added WebSearchTool
}

#[test]
fn test_all_tools_registered_in_default_registry() {
    // Verify ToolRegistry has all 7 tools
    let registry = ToolRegistry::default();
    let tools = registry.list();

    assert_eq!(tools.len(), 9, "Should have exactly 9 tools including WebSearchTool");

    // Verify each tool is present
    assert!(tools.contains(&"Read"), "Should have Read tool");
    assert!(tools.contains(&"Write"), "Should have Write tool");
    assert!(tools.contains(&"Edit"), "Should have Edit tool");
    assert!(tools.contains(&"Bash"), "Should have Bash tool");
    assert!(tools.contains(&"Grep"), "Should have Grep tool");
    assert!(tools.contains(&"Glob"), "Should have Glob tool");
    assert!(tools.contains(&"AstGrep"), "Should have AstGrep tool");
}

#[test]
fn test_rig_is_reexported_from_lib() {
    // Verify rig is re-exported from codelet
    use codelet::rig;

    // Verify we can access rig types through codelet::rig
    fn _verify_rig_types_accessible() {
        let _ = std::any::type_name::<rig::completion::ToolDefinition>();
        let _ = std::any::type_name::<
            rig::agent::Agent<rig::providers::anthropic::completion::CompletionModel>,
        >();
    }
}

#[tokio::test]
async fn test_tool_registry_can_provide_definitions() {
    // Verify ToolRegistry can generate tool definitions for LLM API
    let registry = ToolRegistry::default();
    let definitions = registry.definitions();

    assert_eq!(definitions.len(), 7, "Should generate 7 tool definitions");

    // Verify each definition has required fields
    for def in definitions {
        assert!(!def.name.is_empty(), "Tool name should not be empty");
        assert!(
            !def.description.is_empty(),
            "Tool description should not be empty for {}",
            def.name
        );
        assert!(
            def.input_schema.is_object(),
            "Tool input_schema should be an object for {}",
            def.name
        );
    }
}

#[test]
fn test_claude_provider_auth_modes() {
    // Verify AuthMode enum is accessible (indirectly through ClaudeProvider methods)
    // We can't test actual authentication without API keys, but we can verify the structure

    // Verify provider has OAuth-related methods (compile-time check)
    fn _verify_oauth_methods(provider: &ClaudeProvider) {
        let _ = provider.is_oauth_mode();
        let _ = provider.get_system_prompt_prefix();
        let _ = provider.get_anthropic_beta_header();
    }
}
