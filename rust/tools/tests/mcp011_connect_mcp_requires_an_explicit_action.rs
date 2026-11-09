//! MCP-011 — ConnectMCP requires an explicit action and reports
//! serialization errors.
//!
//! Feature: spec/features/connect-mcp-requires-an-explicit-action.feature

use codelet_tools::{ConnectMcpTool, McpConnectArgs};
use rig::tool::Tool;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_connect_mcp_call_without_an_action_is_rejected() {
    // @step Given ConnectMCP arguments naming a stdio server but no action
    let arguments = json!({"name": "srv", "transport": "stdio", "command": "srv", "args": []});
    // @step When they are parsed
    let parsed = serde_json::from_value::<McpConnectArgs>(arguments);
    // @step Then parsing fails naming the missing action
    let error = parsed
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(error.contains("action"), "{error}");
}

#[tokio::test]
async fn the_connect_mcp_schema_requires_the_action() -> TestResult {
    // @step Given the ConnectMCP tool definition
    let definition = ConnectMcpTool::new(uuid::Uuid::nil())
        .definition(String::new())
        .await;
    // @step When its parameter schema is read
    let required = definition.parameters["required"].clone();
    // @step Then action is listed as required
    assert!(
        required
            .as_array()
            .is_some_and(|fields| fields.contains(&json!("action"))),
        "{required}"
    );
    Ok(())
}
