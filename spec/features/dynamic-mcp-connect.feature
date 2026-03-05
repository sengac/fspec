@done
@connect
@tools
@MCP-001
Feature: Dynamic MCP: Tool-Driven MCP Integration via ConnectMCP

  """
  ConnectMCP tool uses rmcp crate (same as Codex) for MCP JSON-RPC 2.0 over stdio transport (TokioChildProcess) and Streamable HTTP transport (StreamableHttpClientTransport). Protocol version: 2025-11-25
  MCP lifecycle per connection: spawn/connect → initialize handshake (protocolVersion, capabilities, clientInfo/serverInfo) → notifications/initialized → tools/list → cache tools on session. MCP spec 2025-11-25 section: Lifecycle
  Tool routing: mcp__<server>__<tool>(args) → split on '__' → lookup server in session.mcp_connections → send JSON-RPC {method: 'tools/call', params: {name: '<tool>', arguments: args}} → return content array from response
  Server-initiated messages handled via rmcp ClientHandler trait: notifications/tools/list_changed → re-fetch tools/list + inject notification; notifications/resources/updated → inject notification; sampling/createMessage → inject into session via watcher_input_tx, route LLM response back as sampling result
  Ephemeral session state: session.mcp_connections: HashMap<String, McpConnection> where McpConnection holds RmcpClient, cached Vec<Tool> from tools/list, server InitializeResult, optional child PID. Tool list assembled fresh each LLM call: built_in_tools + mcp_connections.flat_map(tools)
  rmcp API: handler.serve(transport) does full initialize handshake (sends InitializeRequest with ClientInfo, receives ServerInfo, sends InitializedNotification). Returns RunningService<RoleClient, H> where peer() provides list_all_tools(), call_tool(), list_all_resources(), etc. ClientHandler trait has on_tool_list_changed, on_resource_updated, create_message, on_logging_message callbacks
  Sampling round-trip: create_message handler creates oneshot channel pair, injects SamplingRequest(params, oneshot_tx) as WatcherInput variant, agent_loop processes it as LLM input and captures response through oneshot_tx, handler awaits oneshot_rx and returns CreateMessageResult to rmcp
  MCP tool injection: McpToolWrapper implements rig::tool::Tool — each provider's create_rig_agent() accepts optional MCP tool wrappers and appends them to the agent builder. Since agent is rebuilt every turn, new MCP tools appear immediately
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ConnectMCP is a tool call — the agent calls it to establish MCP server connections at runtime (Dynamic MCP), not through config files at startup (Static MCP)
  #   2. ConnectMCP uses the rmcp crate to speak native MCP JSON-RPC — it spawns the server process (stdio) or opens HTTP connection, performs the initialization handshake, and calls tools/list
  #   3. On success, ConnectMCP returns a structured tool result listing the server name, protocol version, and all discovered tools with their schemas — these tools then become callable as mcp__<server>__<tool> in subsequent LLM turns
  #   4. On failure, ConnectMCP returns a structured error (runtime not found, handshake timeout, auth required) so the LLM can reason about it, suggest fixes, or try alternatives
  #   5. MCP connections are ephemeral — cached on the session as a HashMap<String, McpConnection>, gathered fresh each LLM API call, no persistent registry or lifecycle manager
  #   6. Server-initiated MCP messages (notifications, sampling requests, resource updates) are injected into the session via watcher_input_tx so the LLM sees them
  #   7. MCP tool calls (mcp__<server>__<tool>) are routed through the cached connection — look up server in session.mcp_connections, forward via client.call_tool()
  #   8. Connection lifecycle is tied to the session — when session ends, all McpConnections are dropped and child processes killed
  #   9. Skills can describe ConnectMCP calls inline — enabling multi-server workflows where the agent connects to each MCP server as needed during skill execution
  #   10. ConnectMCP tool call — agent decides when to connect dynamically. No auto-start, no config files.
  #   11. For sampling/createMessage: inject as session input, LLM responds, response returned via the rmcp ClientHandler. The MCP spec defines this as a server→client request — our ClientHandler impl handles the round-trip natively.
  #   12. ConnectMCP calls tools/list during initialization and returns the discovered tools to the LLM. Server capabilities are negotiated during the initialize handshake. No separate manifest needed — the MCP protocol handles discovery natively.
  #   13. ConnectMCP supports three actions: connect (default — establish new server connection), disconnect (tear down named connection, kill child process), and list (show all active connections with tool counts and uptime)
  #   14. ConnectMCP enforces a configurable timeout (default 10 seconds) for the initialization handshake — if the server does not complete initialize within the timeout, the process is killed and a structured timeout error is returned to the LLM
  #   15. Config file support for pre-declaring MCP servers is V2 scope — the agent would read a config and call ConnectMCP for each entry. V1 has no config file; all connections are explicit tool calls
  #   16. MCP tool names are namespaced as mcp__<server>__<tool> — the double-underscore prevents collisions between servers that expose tools with the same name, and between MCP tools and built-in tools
  #   17. ConnectMCP connect action accepts optional env (HashMap) for stdio transport and optional headers (HashMap) for HTTP transport — enabling API tokens, credentials, and auth headers
  #
  # EXAMPLES:
  #   1. Agent calls ConnectMCP(name: 'github', transport: 'stdio', command: 'npx -y @modelcontextprotocol/server-github') → tool spawns child process, sends JSON-RPC initialize request, receives server capabilities, calls tools/list → returns structured result listing server name, protocol version, and discovered tools
  #   2. LLM calls mcp__github__create_issue({owner: 'org', repo: 'project', title: 'Bug fix'}) → tool routes to cached github connection → MCP server returns content → tool returns content to LLM as tool result
  #   3. ConnectMCP(name: 'db', command: 'python3 db-mcp-server.py') fails because python3 is not installed → tool returns structured failure with stderr output → LLM can reason about the failure
  #   4. Skill file describes two ConnectMCP calls → agent connects both mid-session → agent has typed tools from both servers → session ends, both child processes killed
  #   5. Connected MCP server sends notifications/tools/list_changed → ClientHandler re-fetches tools/list → injects notification into session → next LLM call includes updated tool schemas
  #   6. MCP server sends sampling/createMessage → injected into session as input → LLM response routed back as sampling result
  #   7. Agent calls ConnectMCP(action: 'list') → sees active connections → calls ConnectMCP(action: 'disconnect', name: 'github') → connection closed, tools removed
  #   8. ConnectMCP with timeout: 5 → server doesn't respond within 5s → process killed, structured timeout error returned
  #
  # ========================================

  Background: User Story
    Given the agent has a ConnectMCP tool available in its tool list
    And the session has a watcher_input_tx channel for message injection

  Scenario: Connect to MCP server via stdio transport
    Given an MCP server command "npx -y @modelcontextprotocol/server-everything" is available
    When the agent calls ConnectMCP with name "everything" and transport "stdio" and command "npx -y @modelcontextprotocol/server-everything"
    Then the tool should spawn a child process for the command
    And the tool should perform the MCP initialize handshake via rmcp
    And the tool should call tools/list to discover available tools
    And the tool should cache the connection in session.mcp_connections under key "everything"
    And the tool should return a structured success result containing the server name
    And the result should list the protocol version
    And the result should list all discovered tools with their names and descriptions

  Scenario: Route tool call through cached MCP connection
    Given an MCP server "github" is connected with tools including "create_issue"
    When the LLM calls tool "mcp__github__create_issue" with arguments owner "org" and repo "project" and title "Bug fix"
    Then the tool should parse the server name "github" and tool name "create_issue" from the qualified name
    And the tool should look up "github" in session.mcp_connections
    And the tool should forward the call via peer().call_tool() with name "create_issue" and the provided arguments
    And the tool should return the MCP server's response content to the LLM

  Scenario: Handle spawn failure with structured error
    Given "python3" is not installed on the system
    When the agent calls ConnectMCP with name "db" and transport "stdio" and command "python3 db-mcp-server.py"
    Then the tool should catch the process spawn error
    And the tool should return a structured failure result to the LLM
    And the failure result should include the error message indicating the command was not found
    And the failure result should not crash the session or leave orphaned state

  Scenario: Connect to MCP server via HTTP transport
    Given a remote MCP server is available at "https://mcp.example.com/db"
    When the agent calls ConnectMCP with name "remote-db" and transport "http" and url "https://mcp.example.com/db"
    Then the tool should create a StreamableHttpClientTransport for the URL
    And the tool should perform the MCP initialize handshake via rmcp
    And the tool should cache the connection under key "remote-db"
    And the tool should return a structured success result with discovered tools

  Scenario: Multi-server workflow with independent connections
    Given the agent connects to MCP server "github" via stdio
    And the agent connects to MCP server "sonar" via http
    When the tool list is assembled for the next LLM API call
    Then the tool list should include tools prefixed with "mcp__github__"
    And the tool list should include tools prefixed with "mcp__sonar__"
    And both connections should be independently cached in session.mcp_connections

  Scenario: Receive server tool list changed notification
    Given an MCP server "github" is connected
    When the MCP server sends a notifications/tools/list_changed notification
    Then the ClientHandler on_tool_list_changed callback should fire
    And the handler should re-fetch the tool list via peer().list_all_tools()
    And the handler should update the cached tools for the "github" connection
    And the handler should inject a notification message into the session via watcher_input_tx
    And the next LLM API call should include the updated tool schemas

  Scenario: Handle server sampling/createMessage request
    Given an MCP server "analysis" is connected
    When the MCP server sends a sampling/createMessage request with messages and maxTokens
    Then the ClientHandler create_message callback should fire
    And the handler should create a oneshot response channel
    And the handler should inject the sampling request into the session via watcher_input_tx with the oneshot sender
    And the agent_loop should process the injection as LLM input and capture the response
    And the captured response should be sent through the oneshot channel
    And the handler should receive the response and return it as CreateMessageResult
    And the MCP server should receive the sampling response

  Scenario: List active MCP connections
    Given MCP server "github" is connected with 5 tools and 12 calls made
    And MCP server "sonar" is connected with 3 tools and 0 calls made
    When the agent calls ConnectMCP with action "list"
    Then the tool should return a summary of all active connections
    And the summary should include server name, uptime, tool count, and call count for each

  Scenario: Disconnect an active MCP connection
    Given MCP server "github" is connected
    When the agent calls ConnectMCP with action "disconnect" and name "github"
    Then the tool should cancel the RunningService for "github"
    And the child process should be killed
    And the connection should be removed from session.mcp_connections
    And the tool should return a confirmation with connection statistics
    And tools prefixed with "mcp__github__" should no longer appear in subsequent LLM calls

  Scenario: Handle connection timeout during initialization
    Given an MCP server command "node slow-mcp.js" that does not complete initialization
    When the agent calls ConnectMCP with name "slow-server" and transport "stdio" and command "node slow-mcp.js" and timeout 5
    Then the tool should wait up to 5 seconds for the MCP handshake to complete
    And when the timeout elapses without a response the tool should kill the child process
    And the tool should return a structured timeout error to the LLM
    And the error should include the timeout duration and indicate the process was started but did not respond

  Scenario: Handle HTTP authentication failure
    Given a remote MCP server at "https://mcp.example.com/secure" requires authentication
    When the agent calls ConnectMCP with name "secure-db" and transport "http" and url "https://mcp.example.com/secure"
    Then the tool should attempt the connection and receive an HTTP 401 response
    And the tool should return a structured auth error to the LLM
    And the error should indicate authentication is required

  Scenario: Session cleanup kills all MCP connections
    Given MCP server "github" is connected via stdio with a child process
    And MCP server "sonar" is connected via http
    When the session ends
    Then all McpConnection entries should be dropped
    And the stdio child process for "github" should be killed
    And the HTTP connection for "sonar" should be closed
    And no orphaned processes should remain

  Scenario: Tool list assembly includes MCP tools alongside built-in tools
    Given the session has built-in tools "Read", "Write", "Bash"
    And MCP server "github" is connected with tools "create_issue" and "list_repos"
    When the tool list is gathered for an LLM API call
    Then the result should contain "Read", "Write", "Bash" as built-in tools
    And the result should contain "mcp__github__create_issue" and "mcp__github__list_repos"
    And MCP tools should include the input_schema from the MCP server's tool definition

  Scenario: Tool name collision prevented by double-underscore namespacing
    Given MCP server "alpha" is connected with a tool named "search"
    And MCP server "beta" is connected with a tool named "search"
    When the tool list is assembled
    Then "mcp__alpha__search" and "mcp__beta__search" should both appear as distinct tools
    And calling "mcp__alpha__search" should route to the "alpha" connection
    And calling "mcp__beta__search" should route to the "beta" connection

  Scenario: ConnectMCP with duplicate server name replaces existing connection
    Given MCP server "github" is already connected
    When the agent calls ConnectMCP with name "github" and a different command
    Then the existing "github" connection should be disconnected first
    And the new connection should replace it in session.mcp_connections
    And the tool should return a success result noting the replacement
