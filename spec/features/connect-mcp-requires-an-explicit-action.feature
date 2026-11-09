@BUG-582
@done
@MCP-011
@tools
@tool-execution
@rust
Feature: ConnectMCP requires an explicit action and reports serialization errors
  """
  McpConnectArgs.action had #[serde(default)] (McpAction::Connect), so a call
  without an action connected; ConnectMcpTool::call passed
  serde_json::to_value(&args).unwrap_or_default() to the pre_tool_use hook (a
  failure handed the hook Null); and definition() fell back to
  {"type":"object"} when the schema failed to serialize.
  Now action is required (no Default on McpAction), the hook payload error
  refuses the call, and the schema is serialized once: if that ever fails
  the definition advertises a closed empty schema naming the error and every
  call is refused with it (definition() has no error channel).
  Tests: rust/tools/tests/mcp011_connect_mcp_requires_an_explicit_action.rs.
  """

  Background: User Story
    As an agent calling ConnectMCP
    I want to have every ConnectMCP call state its action and fail with the real error when arguments or schema cannot be serialized
    So that a malformed call is never silently treated as a connect

  Scenario: A ConnectMCP call without an action is rejected
    Given ConnectMCP arguments naming a stdio server but no action
    When they are parsed
    Then parsing fails naming the missing action

  Scenario: The ConnectMCP schema requires the action
    Given the ConnectMCP tool definition
    When its parameter schema is read
    Then action is listed as required
