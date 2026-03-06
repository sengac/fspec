@done
@MCP-002
Feature: MCP tools not available in same turn as ConnectMCP call

  """
  Uses existing ToolServerHandle.add_tool() and remove_tool() APIs — store handle in McpSessionState via set_tool_server_handle()
  ToolServerHandle is Clone (wraps Sender<ToolServerRequest>) — safe to store as Option<ToolServerHandle> in McpSessionState
  run_with_provider! macro sets handle after agent build via set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ToolServerHandle is Clone (wraps a channel Sender) and supports add_tool() for dynamic mid-turn tool registration
  #   2. After ConnectMcpTool successfully connects, it must register new tools with the running agent's ToolServerHandle so they are callable in the same turn
  #   3. The ToolServerHandle must be stored in per-session MCP state (McpSessionState) so ConnectMcpTool can access it at call time
  #   4. Only newly connected server tools should be added mid-turn — tools from previously connected servers are already registered
  #   5. If ToolServerHandle is not yet set (e.g. race condition), the connect should still succeed — tools just appear next turn as before
  #   6. Disconnect should remove tools from the running agent's ToolServerHandle via remove_tool()
  #
  # EXAMPLES:
  #   1. Agent calls ConnectMCP for playwright, then immediately calls mcp__playwright__browser_navigate — succeeds in same turn
  #   2. Agent calls ConnectMCP for server A, then calls ConnectMCP for server B, then calls tools from both — both work in same turn
  #   3. Agent calls ConnectMCP then disconnect for same server in same turn — tools are removed and subsequent calls fail gracefully
  #   4. ToolServerHandle not yet set when ConnectMcpTool runs — connect succeeds, tools appear next turn (graceful degradation)
  #   5. Tools connected in previous turns still work alongside newly connected same-turn tools
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to use MCP tools immediately after connecting to an MCP server in the same turn
    So that I can connect and invoke MCP tools without requiring a separate user turn

  Scenario: Same-turn tool invocation after connect
    Given an MCP session is initialized with a ToolServerHandle
    When ConnectMcpTool successfully connects to an MCP server "playwright"
    Then the newly discovered tools are registered with the running agent's ToolServerHandle
    And calling "mcp__playwright__browser_navigate" succeeds in the same turn

  Scenario: Multiple servers connected in same turn
    Given an MCP session is initialized with a ToolServerHandle
    When ConnectMcpTool connects to server "serverA" with 2 tools
    And ConnectMcpTool connects to server "serverB" with 3 tools
    Then the ToolServerHandle contains tools from both "serverA" and "serverB"
    And calling tools from either server succeeds

  Scenario: Disconnect removes tools from running agent
    Given an MCP session is initialized with a ToolServerHandle
    And ConnectMcpTool has connected to server "playwright" with tools registered
    When ConnectMcpTool disconnects from server "playwright"
    Then the tools from "playwright" are removed from the ToolServerHandle

  Scenario: Graceful degradation when ToolServerHandle is not set
    Given an MCP session is initialized without a ToolServerHandle
    When ConnectMcpTool successfully connects to an MCP server "playwright"
    Then the connection succeeds and tools are stored in the connection map
    And no error occurs during the connect call
    But the tools are not registered with any ToolServerHandle

  Scenario: Previous-turn tools coexist with same-turn tools
    Given an MCP session is initialized with a ToolServerHandle
    And server "existing_server" was connected in a previous turn with tools already registered
    When ConnectMcpTool connects to a new server "new_server"
    Then tools from both "existing_server" and "new_server" are available
    And only "new_server" tools were added mid-turn via add_tool

  Scenario: ToolServerHandle is stored after agent build
    Given an MCP session is initialized
    When the run_with_provider macro builds an agent
    Then the agent's ToolServerHandle is stored in McpSessionState via set_mcp_tool_server_handle
    And subsequent ConnectMcpTool calls can access it for mid-turn registration
