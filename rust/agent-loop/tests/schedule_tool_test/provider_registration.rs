//! Feature: spec/features/schedule-ai-tool.feature

// =============================================================================
// Scenario: ScheduleTool registered in all provider agent builders
// =============================================================================

/// BUG-450: a provider's whole module tree (see provider_source.rs).
fn read_provider_source(filename: &str) -> String {
    super::provider_source::read_provider_source(filename).unwrap_or_else(|error| panic!("{error}"))
}
#[test]
fn test_schedule_tool_registered_in_claude_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("claude.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "claude.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "claude.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}
#[test]
fn test_schedule_tool_registered_in_gemini_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("gemini.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "gemini.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "gemini.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}
#[test]
fn test_schedule_tool_registered_in_openai_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("openai.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "openai.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "openai.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}
#[test]
fn test_schedule_tool_registered_in_zai_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("zai.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "zai.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "zai.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}
#[test]
fn test_schedule_tool_registered_in_codex_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("codex/agent.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "codex/agent.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "codex/agent.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}
