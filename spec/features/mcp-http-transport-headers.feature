@done
@authentication
@tool-discovery
@MCP-004
@rpc
@session
@external-bridge
Feature: MCP HTTP transport sends configured headers
  """
  Bug MCP-004: connect_http (rust/tools/src/mcp/http_and_session_registry.rs) built the Streamable HTTP
  transport with from_uri and dropped every configured header with a warning, so servers that require
  credentials (for example Authorization: Bearer for the Paseo Bots relay) refused fspec.

  Fix: connect_http parses each header into an http HeaderName/HeaderValue (marked sensitive) and builds the
  transport with rmcp 1.2 StreamableHttpClientTransport::from_config(with_uri(url).custom_headers(..)), so the
  headers go on every request of that connection. rmcp refuses reserved MCP headers (accept, mcp-session-id,
  last-event-id) itself. Both front doors share it: the ConnectMCP tool and the daemon's
  configure_session_mcp_servers, whose wire type SessionMcpTransport::Http gains headers (bincode order url,
  headers). Header values never appear in Debug output or errors; headers live in the connection's own
  transport config, never in global state.
  """

  Background: User Story
    As an agent or daemon client connecting an authenticated Streamable HTTP MCP server
    I want the headers I configure to be sent on every MCP request
    So that servers that require credentials accept fspec without the credentials leaking

  Scenario: A configured bearer header authenticates the handshake, discovery and a tool call
    Given a Streamable HTTP MCP server that requires the bearer "token-a" on every request
    When connect_http connects server "remote" with header Authorization "Bearer token-a"
    Then the connection lists the tool "search"
    And calling "mcp__remote__search" returns the fixture result

  Scenario: A server that requires credentials refuses a connection without headers
    Given a Streamable HTTP MCP server that requires the bearer "token-a" on every request
    When connect_http connects server "remote" without headers
    Then the connect fails saying the server requires authentication

  Scenario: A reserved MCP header is refused
    Given a Streamable HTTP MCP server that requires the bearer "token-a" on every request
    When connect_http connects server "remote" with header Mcp-Session-Id "forged"
    Then the connect fails naming the reserved header "mcp-session-id"

  Scenario: Concurrent connections each send only their own token
    Given two Streamable HTTP MCP servers requiring the bearers "token-a" and "token-b"
    When server "alpha" with token "token-a" and server "beta" with token "token-b" connect concurrently
    Then both connections list the tool "search"

  Scenario: A daemon client configures an HTTP MCP server with headers for a session
    Given a token-armed daemon with an idle session A and an HTTP MCP server requiring the bearer "token-a"
    When a bearer-authenticated client configures http server "remote" with header Authorization "Bearer token-a"
    Then the response lists server "remote" with its tool "search"

  Scenario: An invalid header value is refused without revealing it
    Given a token-armed daemon with an idle session A and an HTTP MCP server requiring the bearer "token-a"
    When a bearer-authenticated client configures stdio server "good" and http server "remote" with header X-Token "secret-value\nsplit"
    Then the request fails with an error naming the header "X-Token"
    And the error does not contain "secret-value"
    And session A has no MCP connections

  Scenario: Debug output of an HTTP transport shows header names but never values
    Given an HTTP session MCP transport with header Authorization "Bearer secret-token"
    When it is formatted with Debug
    Then the output contains "Authorization"
    And the output does not contain "secret-token"
