@wip
@deferred
@session-management
@RPC-089
@rust
@agent-loop
@rpc
@mcp
Feature: Agent loop drains mcp_injection_rx as a normal turn
  """
  RPC-089 (child of RPC-072 family). The outer loop body must use
  tokio::select! between input_rx.recv() and mcp_injection_rx.recv()
  (gated by mcp_channel_open flag) — exactly as
  codelet/napi/src/agent_loop.rs:323-460 does. The McpInjection channel
  cannot be discarded.

  Originally scenario "MCP injection through mcp_injection_rx is
  processed as a turn" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want MCP server messages to drive Work Agent turns just like user input
    So that connected MCP servers can prompt the agent end-to-end

  Scenario: MCP injection through mcp_injection_rx is processed as a turn
    Given a Work Agent session with FspecAgentHooks installed
    And an MCP server sends an McpInjection through the per-session mcp_injection_rx channel
    When the agent loop's tokio::select! reaches the next iteration
    Then the injection is drained and processed as a normal turn
    And the session emits at least one StreamChunk::Text + StreamChunk::Done in response
