@BUG-582
@done
@tool-discovery
@authentication
@MCP-006
@rpc
@session
@external-bridge
Feature: Legacy SSE MCP client transport
  """
  User decision: fspec implements a legacy (MCP 2024-11-05 HTTP+SSE) client transport because rmcp 1.2 has none, and
  never treats SSE as Streamable HTTP.

  LegacySseTransport (rust/tools/src/mcp/sse_transport.rs) implements rmcp Transport<RoleClient>: it GETs the SSE url
  with the configured headers, waits for the endpoint event, refuses an endpoint on another origin, POSTs every
  JSON-RPC message to the endpoint with the same headers, and forwards each message event to rmcp from a reader task
  that close and drop abort. connect_sse (ConnectMCP transport "sse") shares the serve-with-timeout and discovery tail
  with connect_http. The daemon wire type gains SessionMcpTransport::Sse { url, headers } (variant 2); Debug prints
  header names only and errors never carry the url or header values.
  """

  Background: User Story
    As an agent or daemon client using an MCP server that only speaks the legacy SSE transport
    I want fspec to connect to it over SSE with my configured headers
    So that its tools work without being mistaken for a Streamable HTTP server

  Scenario: ConnectMCP connects a bearer-guarded legacy SSE server and calls its tool
    Given a legacy SSE MCP server that requires the bearer "token-a" on every request
    When the agent calls ConnectMCP for sse server "legacy" with header Authorization "Bearer token-a"
    Then the connection lists the tool "search"
    And calling "mcp__legacy__search" returns the fixture result

  Scenario: A legacy SSE server that requires credentials refuses a connection without headers
    Given a legacy SSE MCP server that requires the bearer "token-a" on every request
    When connect_sse connects server "legacy" without headers
    Then the connect fails saying the server requires authentication

  Scenario: An unreachable SSE url fails the connect
    Given no server listening on a local port
    When connect_sse connects server "gone" to that port
    Then the connect fails naming server "gone"

  Scenario: A server that never sends the endpoint event times out
    Given a legacy SSE MCP server that never sends its endpoint event
    When connect_sse connects server "silent" with a timeout of 1 second
    Then the connect fails with a timeout naming server "silent"

  Scenario: An endpoint on another origin is refused
    Given a legacy SSE MCP server whose endpoint event names "http://other.invalid/messages"
    When connect_sse connects server "foreign" with header Authorization "Bearer token-a"
    Then the connect fails saying the endpoint is on another origin

  Scenario: A Streamable HTTP server is not mistaken for a legacy SSE server
    Given a Streamable HTTP MCP server that refuses GET requests
    When connect_sse connects server "streamable" to its url
    Then the connect fails naming server "streamable"
    And no connection named "streamable" exists

  Scenario: A daemon client configures a legacy SSE server for one session only
    Given a token-armed daemon with idle sessions A and B and a legacy SSE MCP server requiring the bearer "token-a"
    When a bearer-authenticated client configures sse server "legacy" with header Authorization "Bearer token-a" for session A and stdio server "notes" for session B
    Then session A's MCP tools are exactly "mcp__legacy__search"
    And session B's MCP tools are exactly "mcp__notes__search"

  Scenario: A disconnected SSE server can be connected again
    Given a legacy SSE MCP server and a connection "legacy" to it
    When the connection is disconnected and connect_sse connects "legacy" again
    Then the new connection lists the tool "search"

  Scenario: Debug output of an SSE transport hides the url and header values
    Given an SSE session MCP transport with url "https://mcp.example/sse-secret" and header Authorization "Bearer secret-token"
    When it is formatted with Debug
    Then the output contains "Authorization"
    And the output contains neither "sse-secret" nor "secret-token"
