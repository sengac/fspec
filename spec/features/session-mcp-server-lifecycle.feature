@BUG-582
@done
@MCP-003
@MCP-008
@rpc
@session
@external-bridge
@authentication
@tool-discovery
Feature: Session MCP server names, lifecycle, transports and wire order
  """
  Companion of session-mcp-server-configuration.feature (MCP-003): a configured server name is
  unique within its session, configuration needs a live session, destroying the session releases
  its servers, a Streamable HTTP server can be configured like a stdio one, and the new
  FspecService method configure_session_mcp_servers is appended after every existing method
  (RPC-449 frozen wire order).
  """

  Background: User Story
    As a trusted daemon client such as fspec-paseo
    I want to configure named MCP servers for one session through the daemon before its next prompt
    So that the session's agent can use the tools my integration supplies, isolated from other sessions

  Scenario: Duplicate server names in one request are refused
    Given a token-armed daemon with an idle session A whose server "notes" is configured
    When a bearer-authenticated client configures "notes" and two servers both named "docs"
    Then the request fails naming "docs"
    And session A still has exactly one connection named "notes"

  Scenario: An unknown session is refused
    Given a token-armed daemon
    When a bearer-authenticated client configures a server for a session that does not exist
    Then the request fails naming the session

  Scenario: Destroying a session releases its configured servers
    Given a token-armed daemon with session A whose server "notes" is configured
    When session A is destroyed
    Then session A's MCP registry entry is gone

  Scenario: A Streamable HTTP MCP server is configured for a session
    Given a token-armed daemon with an idle session A and a local Streamable HTTP MCP server
    When a bearer-authenticated client configures http server "remote" with that url for session A
    Then the response lists server "remote" with its tool "search"

  Scenario: configure_session_mcp_servers is appended after every existing method
    Given the FspecService methods in declaration order
    When the index of configure_session_mcp_servers is read
    Then configure_session_mcp_servers keeps index 129, after every method declared before it
