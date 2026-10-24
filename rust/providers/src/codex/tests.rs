use super::CodexProvider;
use uuid::Uuid;

// =========================================================================
// Unit tests for build_reasoning_params — the core function that constructs
// the reasoning JSON matching codex-rs wire format.
// Feature: spec/features/codex-provider-reasoning-configuration.feature
// =========================================================================

#[test]
fn build_reasoning_params_with_high_effort_config() {
    // @step Given I have a thinking_config with reasoning effort "high" and summary "auto"
    let config = serde_json::json!({"reasoning": {"effort": "high", "summary": "auto"}});

    // @step When I call build_reasoning_params with the thinking_config
    let params = CodexProvider::build_reasoning_params(Some(&config));

    // @step Then the params should contain reasoning.effort "high"
    assert_eq!(params["reasoning"]["effort"], "high");
    // @step And the params should contain reasoning.summary "auto"
    assert_eq!(params["reasoning"]["summary"], "auto");
    // @step And the params should contain store as false
    assert_eq!(params["store"], false);
    // @step And the params should contain include with "reasoning.encrypted_content"
    assert_eq!(params["include"][0], "reasoning.encrypted_content");
}

#[test]
fn build_reasoning_params_with_medium_effort_config() {
    let config = serde_json::json!({"reasoning": {"effort": "medium", "summary": "auto"}});
    let params = CodexProvider::build_reasoning_params(Some(&config));

    assert_eq!(params["reasoning"]["effort"], "medium");
    assert_eq!(params["reasoning"]["summary"], "auto");
    assert_eq!(params["store"], false);
    assert_eq!(params["include"][0], "reasoning.encrypted_content");
}

#[test]
fn build_reasoning_params_with_low_effort_config() {
    let config = serde_json::json!({"reasoning": {"effort": "low", "summary": "auto"}});
    let params = CodexProvider::build_reasoning_params(Some(&config));

    assert_eq!(params["reasoning"]["effort"], "low");
    assert_eq!(params["reasoning"]["summary"], "auto");
}

#[test]
fn build_reasoning_params_defaults_to_high_when_none() {
    // @step When I call build_reasoning_params with None as thinking_config
    let params = CodexProvider::build_reasoning_params(None);

    // @step Then the params should contain default reasoning.effort "high"
    assert_eq!(params["reasoning"]["effort"], "high");
    // @step And the params should contain default reasoning.summary "auto"
    assert_eq!(params["reasoning"]["summary"], "auto");
    // @step And store and include should still be present
    assert_eq!(params["store"], false);
    assert_eq!(params["include"][0], "reasoning.encrypted_content");
}

#[test]
fn build_reasoning_params_defaults_to_high_when_empty_config() {
    // Empty config (from ThinkingLevel::Off flowing through as Some({}))
    // should still get default reasoning because Codex models always need it
    let config = serde_json::json!({});
    let params = CodexProvider::build_reasoning_params(Some(&config));

    assert_eq!(params["reasoning"]["effort"], "high");
    assert_eq!(params["reasoning"]["summary"], "auto");
}

/// Feature: spec/features/codex-native-tool-facades.feature
///
/// Scenario: Codex agent does not expose non-native glob tool
/// BUG-107: glob is NOT part of the native Codex CLI tool set
#[tokio::test]
async fn create_rig_agent_does_not_expose_non_native_glob_tool() {
    // @step Given a Codex agent built with create_rig_agent
    let provider = CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
        .expect("Provider should initialize");

    let session_id = Uuid::new_v4();
    let agent = provider.create_rig_agent(session_id, None, None);

    // @step When the agent tool definitions are inspected
    let tool_defs = agent
        .tool_server_handle
        .get_tool_defs(None)
        .await
        .expect("Tool definitions should be available");

    let tool_names: Vec<&str> = tool_defs.iter().map(|def| def.name.as_str()).collect();

    // @step Then the tool list does not contain "glob"
    assert!(
        !tool_names.contains(&"glob"),
        "Codex agent should NOT expose non-native 'glob' tool, but found: {tool_names:?}"
    );
    assert!(
        !tool_names.contains(&"Glob"),
        "Codex agent should NOT expose 'Glob' tool either"
    );

    // @step And the tool list contains "shell_command"
    assert!(tool_names.contains(&"shell_command"));
    // @step And the tool list contains "read_file"
    assert!(tool_names.contains(&"read_file"));
    // @step And the tool list contains "list_dir"
    assert!(tool_names.contains(&"list_dir"));
    // @step And the tool list contains "grep_files"
    assert!(tool_names.contains(&"grep_files"));
    // @step And the tool list contains "apply_patch"
    assert!(tool_names.contains(&"apply_patch"));

    // @step And the agent does not have "Write" or "Edit" tools registered
    // BUG-105 Rule [6]: WriteTool and EditTool removed to prevent competing edit interfaces
    assert!(
        !tool_names.contains(&"Write"),
        "Codex agent should NOT expose 'Write' tool (BUG-105), but found: {tool_names:?}"
    );
    assert!(
        !tool_names.contains(&"Edit"),
        "Codex agent should NOT expose 'Edit' tool (BUG-105), but found: {tool_names:?}"
    );

    // @step Then the agent's tool list includes a tool named "view_image"
    // BUG-112: Codex-native view_image tool for viewing local image files
    assert!(
        tool_names.contains(&"view_image"),
        "Codex agent should expose 'view_image' tool (BUG-112), but found: {tool_names:?}"
    );
}

/// Feature: spec/features/codex-shell-exec-facades.feature
///
/// Scenario: Both facades are registered in Codex create_rig_agent
#[tokio::test]
async fn create_rig_agent_exposes_shell_and_exec_command_tools() {
    // @step Given a CodexProvider with create_rig_agent configured
    let provider = CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
        .expect("Provider should initialize");

    // @step When the agent is built with session_id
    let session_id = Uuid::new_v4();
    let agent = provider.create_rig_agent(session_id, None, None);

    let tool_defs = agent
        .tool_server_handle
        .get_tool_defs(None)
        .await
        .expect("Tool definitions should be available");

    let tool_names: Vec<&str> = tool_defs.iter().map(|def| def.name.as_str()).collect();

    // @step Then the tool list contains "shell"
    assert!(
        tool_names.contains(&"shell"),
        "Codex agent should expose 'shell' tool (BUG-114), but found: {tool_names:?}"
    );

    // @step And the tool list contains "exec_command"
    assert!(
        tool_names.contains(&"exec_command"),
        "Codex agent should expose 'exec_command' tool (BUG-114), but found: {tool_names:?}"
    );

    // @step And shell uses ExecToolFacadeWrapper with CodexShellFacade
    // Verify shell tool has the correct schema (array command)
    let shell_def = tool_defs.iter().find(|d| d.name == "shell").unwrap();
    assert_eq!(
        shell_def.parameters["properties"]["command"]["type"],
        "array"
    );
    assert_eq!(shell_def.parameters["required"][0], "command");

    // @step And exec_command uses ExecToolFacadeWrapper with CodexExecCommandFacade
    // Verify exec_command tool has the correct schema (string cmd)
    let exec_def = tool_defs.iter().find(|d| d.name == "exec_command").unwrap();
    assert_eq!(exec_def.parameters["properties"]["cmd"]["type"], "string");
    assert_eq!(exec_def.parameters["required"][0], "cmd");

    // @step And write_stdin uses ExecToolFacadeWrapper with CodexWriteStdinFacade (BUG-115)
    assert!(
        tool_names.contains(&"write_stdin"),
        "Codex agent should expose 'write_stdin' tool (BUG-115), but found: {tool_names:?}"
    );
    let write_def = tool_defs.iter().find(|d| d.name == "write_stdin").unwrap();
    assert_eq!(
        write_def.parameters["properties"]["session_id"]["type"],
        "string"
    );
    assert_eq!(write_def.parameters["required"][0], "session_id");
}

#[test]
fn build_reasoning_params_preserves_custom_effort_from_thinking_config() {
    // Verify that user's thinking level preference is respected when provided
    let config = serde_json::json!({"reasoning": {"effort": "low", "summary": "auto"}});
    let params = CodexProvider::build_reasoning_params(Some(&config));

    // Should use the provided effort, NOT the default "high"
    assert_eq!(
        params["reasoning"]["effort"], "low",
        "User's effort level should be preserved, not overridden with default"
    );
}
