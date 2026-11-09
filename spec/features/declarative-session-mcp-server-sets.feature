@done
@tool-discovery
@MCP-008
@MCP-010
@rpc
@session
@external-bridge
Feature: Declarative session MCP server sets
  """
  USER DECISIONS (2026-11-02, from fspec-paseo FSPEC-074/FSPEC-075):
  1. configure_session_mcp_servers is declarative: the given list becomes the session's whole MCP
  server set. A server not listed is disconnected; a server whose name and transport are identical
  to its daemon-configured connection is kept (not reconnected); a server with the same name and a
  different transport (or one connected by ConnectMCP) is reconnected; a new name is connected. An
  empty list is valid and disconnects every server. New and changed servers are connected first
  and the swap is committed only when all of them succeeded, so a connect failure returns an Err
  naming the server and leaves the previous set intact. The response describes the resulting set.
  2. fspec owns the MCP handshake timeout: one named constant, MCP_HANDSHAKE_TIMEOUT_SECS, used by
  ConnectMCP and the daemon path. SessionMcpServer is { name, transport } (wire change, method
  index unchanged) and refuses unknown fields; ConnectMCP takes no timeout from the model.
  MCP-010: every new and changed server is established concurrently (futures::future::join_all in
  rust/tools/src/mcp/host_config.rs), and each server's handshake and tool discovery together are
  bounded by MCP_HANDSHAKE_TIMEOUT_SECS (rust/tools/src/mcp/host_establish.rs). The swap is still
  committed only when all of them succeeded; otherwise the Err names every failing server in request
  order and the previous set is intact.
  WORST-CASE DURATION: one configure_session_mcp_servers call takes at most MCP_HANDSHAKE_TIMEOUT_SECS
  (10 s) of connect time, whatever the number of servers, plus validation, the session lookup and the
  write-locked swap (milliseconds). tarpc's default client deadline (context::current()) is also 10 s,
  so it is not enough. A daemon client such as fspec-paseo (FSPEC-076) must set its RPC deadline above
  MCP_HANDSHAKE_TIMEOUT_SECS; this feature's tests use MCP_HANDSHAKE_TIMEOUT_SECS + 5 s.
  """

  Background: User Story
    As a trusted daemon client such as fspec-paseo
    I want each configure call to set a session's whole MCP server set under fspec's own handshake timeout
    So that opening or resuming a session converges it on exactly the servers my integration supplies

  Scenario: An unlisted server is disconnected and an identical one is kept
    Given a token-armed daemon with an idle session A whose servers "notes" and "docs" are configured
    When a bearer-authenticated client configures only server "notes" with the same transport
    Then the response lists only server "notes" with its tool "search"
    And session A's only MCP connection is the original "notes" connection

  Scenario: A server whose transport changed is reconnected
    Given a token-armed daemon with an idle session A whose server "notes" is configured
    When a bearer-authenticated client configures server "notes" with different arguments
    Then session A's only MCP connection is a new "notes" connection

  Scenario: An empty list disconnects every server
    Given a token-armed daemon with an idle session A whose server "notes" is configured
    When a bearer-authenticated client configures an empty server list for session A
    Then the response is empty
    And session A has no MCP connections

  Scenario: A failed connect leaves the previous set intact
    Given a token-armed daemon with an idle session A whose servers "notes" and "docs" are configured
    When a bearer-authenticated client configures "notes" with different arguments and "broken" where "broken" exits before its handshake
    Then the request fails with an error naming "broken"
    And session A still has the original "notes" and "docs" connections

  Scenario: A daemon-configured server is bounded by the fspec handshake timeout
    Given a token-armed daemon with an idle session A
    When a bearer-authenticated client configures stdio server "silent" that never answers the handshake
    Then the request fails naming "silent" and the MCP_HANDSHAKE_TIMEOUT_SECS value
    And session A has no MCP connections

  Scenario: Neither front door takes a handshake timeout
    Given the ConnectMCP parameter schema and a SessionMcpServer JSON object carrying timeout_secs
    When the schema is inspected and the object is deserialized
    Then the schema has no timeout property
    And the deserialization fails naming timeout_secs

  Scenario: Several silent servers fail together within one handshake timeout
    Given a token-armed daemon with an idle session A whose server "notes" is configured
    When a bearer-authenticated client configures "notes" and stdio servers "quiet-a", "quiet-b" and "quiet-c" that never answer the handshake
    Then the request fails within twice MCP_HANDSHAKE_TIMEOUT_SECS with an error naming "quiet-a", "quiet-b" and "quiet-c"
    And session A still has only the original "notes" connection

  Scenario: A server that never answers tools/list is bounded by the fspec handshake timeout
    Given a token-armed daemon with an idle session A
    When a bearer-authenticated client configures stdio server "mute" that completes its handshake but never answers tools/list
    Then the request fails within twice MCP_HANDSHAKE_TIMEOUT_SECS naming "mute" and the MCP_HANDSHAKE_TIMEOUT_SECS value
    And session A has no MCP connections
