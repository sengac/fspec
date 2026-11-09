@BUG-582
@done
@tool-discovery
@authentication
@MCP-003
@rpc
@session
@external-bridge
Feature: Daemon clients configure a session's MCP servers
  """
  A trusted daemon client (for example fspec-paseo) configures named MCP servers for one session
  through the new FspecService method configure_session_mcp_servers (appended last, RPC-449).
  USER DECISION: the method requires daemon authentication. It is refused unless the daemon was
  started with FSPEC_DAEMON_TOKEN and the request arrived on a WebSocket connection whose
  handshake carried the matching bearer; on a daemon without a token MCP configuration is
  disabled with an error naming FSPEC_DAEMON_TOKEN. There is no loopback trust: an in-process
  (embedded) client is never bearer-authenticated.

  Each server has a unique name, an explicit transport (stdio command with an optional
  environment, or a Streamable HTTP url); the handshake timeout is fspec's own (MCP-008). The
  daemon connects them through the same per-session MCP registry the ConnectMCP tool uses; the
  list replaces the session's whole MCP set (MCP-008) and configuration is atomic,
  servers belong to their session (its next prompt sees their tools, other sessions never do)
  and destroying the session releases them. Commands and environment values never appear in
  logs or errors. HTTP headers (MCP-004), structured stdio arguments (MCP-005) and legacy SSE
  (MCP-006) are separate cards.
  """

  Background: User Story
    As a trusted daemon client such as fspec-paseo
    I want to configure named MCP servers for one session through the daemon before its next prompt
    So that the session's agent can use the tools my integration supplies, isolated from other sessions

  Scenario: A bearer-authenticated client configures a stdio MCP server for a session
    Given a daemon started with FSPEC_DAEMON_TOKEN and an idle session A
    When a client connected with the daemon bearer configures stdio server "notes" for session A
    Then the response lists server "notes" with its tool "search"
    And session A's MCP tools include "mcp__notes__search"

  Scenario: A daemon without FSPEC_DAEMON_TOKEN refuses MCP configuration
    Given a daemon started without FSPEC_DAEMON_TOKEN and an idle session A
    When a client configures stdio server "notes" for session A
    Then the request fails with an error naming FSPEC_DAEMON_TOKEN
    And session A has no MCP connections

  Scenario: An in-process client is not bearer-authenticated
    Given a token-armed daemon service reached in-process without a bearer handshake
    When the in-process client configures stdio server "notes" for session A
    Then the request fails because the connection did not authenticate with the daemon bearer
    And session A has no MCP connections

  Scenario: A configured server is visible only to its own session
    Given a token-armed daemon with idle sessions A and B
    When a bearer-authenticated client configures stdio server "notes" for session A
    Then session A has an MCP connection named "notes"
    And session B has no MCP connections

  Scenario: A failing server rolls back the whole request
    Given a token-armed daemon with an idle session A
    When a bearer-authenticated client configures servers "good" and "broken" where "broken" exits before its handshake
    Then the request fails with an error naming "broken"
    And session A has no MCP connections
