@wip
@deferred
@session-management
@RPC-083
@rust
@agent-loop
@rpc
@tools
Feature: Agent loop registers tools and executes them end-to-end
  """
  RPC-083 (child of RPC-072 family). The tool registry must reach the
  rig agent via create_rig_agent + RigAgent::with_default_depth so the
  agent can Read/Write/Edit/Bash. MCP wrapper gathering via
  codelet_tools::gather_mcp_tool_wrappers + set_mcp_tool_server_handle
  must run inside each provider arm before the rig agent is wrapped.

  Originally scenario "Tools are registered and executable end-to-end"
  from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want the Work Agent to invoke Read/Write/Edit/Bash tools
    So that the assistant can act on my filesystem like the TS Ink frontend does

  Scenario: Tools are registered and executable end-to-end
    Given a Work Agent session backed by a stub provider that requests a Read tool call
    And a file "/tmp/rpc072-fixture.txt" exists with contents "fspec wins"
    When the user sends "read /tmp/rpc072-fixture.txt"
    Then the scrollback receives a StreamChunk::ToolCall for tool "Read" with the given path
    And the scrollback receives a StreamChunk::ToolResult containing "fspec wins"
    And the scrollback receives a final StreamChunk::Text from the assistant
    And the final StreamChunk::Done arrives after all tool chunks
