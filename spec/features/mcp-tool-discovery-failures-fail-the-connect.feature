@done
@MCP-012
@tools
@tool-execution
@rust
Feature: MCP tool discovery failures fail the connect
  """
  establish_stdio (stdio_and_routing.rs) and the remote serve helper
  (remote_connect.rs, http and sse) called list_all_tools() after the
  handshake: an error was logged with warn! and replaced by an empty tool
  list (the connection succeeded with no tools), and the call had no
  timeout, so a ConnectMCP to a server that never answers tools/list hung
  the tool call forever (MCP-010 bounded only the daemon path).
  Now tools/list runs under the same timeout as the handshake; an error or
  a timeout returns the connect failure (the service is dropped) and
  nothing is cached.
  Tests: rust/tools/tests/mcp012_tool_discovery_failures_fail_the_connect.rs
  (a sh stdio fixture that completes the handshake, then errors or stays
  silent on tools/list).
  """

  Background: User Story
    As an agent connecting an MCP server
    I want to have a failing or silent tools/list fail the connection with the reason
    So that a server is never connected with a silently empty tool list or left hanging the tool call

  Scenario: A tools/list error fails the connect
    Given a stdio MCP server that completes the handshake and answers tools/list with the error "tools are broken"
    When ConnectMCP connects to it
    Then the connect fails naming "tools are broken" and no connection is cached

  Scenario: An unanswered tools/list fails the connect within the timeout
    Given a stdio MCP server that completes the handshake and never answers tools/list
    When ConnectMCP connects to it
    Then the connect fails saying tool discovery did not complete within the MCP timeout and no connection is cached
