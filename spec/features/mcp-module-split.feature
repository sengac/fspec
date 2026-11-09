@done
@BUG-480
@REFAC-032
@REFAC-035
@tools
@refactor
@code-quality
Feature: The dynamic MCP module is split into small files
  """
  rust/tools/src/mcp.rs (3375 lines) keeps its module documentation and imports; its items
  move verbatim, in order, into include!d fragments under rust/tools/src/mcp/ and its unit
  tests into include!d fragments of the same `tests` module, so every item keeps its
  codelet_tools::mcp path and privacy. MCP-003..MCP-006 change this module next. Coverage
  mappings into the moved lines are relocated. REFAC-035: host_config.rs pulls its per-server
  establish() and redact() helpers from the include!d fragment host_establish.rs, so they stay
  private to codelet_tools::mcp::host_config and MCP-010 has room to grow the module.
  BUG-480: connect_tool.rs grew past the limit (MCP-011), so the ToolServerHandle helpers
  add_wrappers_to_handle and remove_server_tools_from_handle, whose only production caller is
  tool_wrappers.rs, move verbatim to the include!d fragment handle_registration.rs.
  """

  Background: User Story
    As an fspec developer
    I want the dynamic MCP module split into small files
    So that MCP transport and session changes touch files under 300 lines

  Scenario: Dynamic MCP source files stay under 300 lines
    Given the dynamic MCP source files in rust/tools/src
    When the line count of each file is measured
    Then no file has 300 or more lines

  Scenario: The dynamic MCP surface is unchanged
    Given code that names the MCP session registry and tool-name helpers through codelet_tools::mcp
    When a session's MCP state is initialised, queried and cleaned up and a tool name is qualified and parsed
    Then the registry reports the session's empty connection map and then none
    And the qualified name parses back to its server and tool

  Scenario: The host MCP configuration's per-server connect helpers live in their own fragment
    Given the host MCP configuration source rust/tools/src/mcp/host_config.rs
    When its per-server connect helpers are looked up
    Then host_config.rs includes host_establish.rs and defines neither establish nor redact
    And host_establish.rs defines both establish and redact

  Scenario: The ConnectMCP tool-server handle helpers live in their own fragment
    Given the ConnectMCP tool source rust/tools/src/mcp/connect_tool.rs
    When its tool-server handle helpers are looked up
    Then mcp.rs includes handle_registration.rs and connect_tool.rs defines neither add_wrappers_to_handle nor remove_server_tools_from_handle
    And handle_registration.rs defines both add_wrappers_to_handle and remove_server_tools_from_handle
