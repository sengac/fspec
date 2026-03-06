@EXT-001
Feature: fspec Browser Agent Chrome Extension

  """
  Service worker runs a Streamable HTTP MCP server using offscreen document or chrome.sockets API for TCP listening. Alternatives: use chrome.offscreen to create a hidden page that runs the HTTP server, or use a tiny Node.js bridge process spawned alongside
  MV3 service workers cannot open TCP ports directly. The HTTP server MUST run in either: (a) an offscreen document (chrome.offscreen API), (b) a companion Node.js native messaging host process, or (c) use the chrome.sockets API (if available in MV3). Option (b) is most reliable and follows the mcp-chrome pattern
  For bidirectional events via ConnectMCP: ConnectMCP's rmcp ClientHandler has on_tool_list_changed callback. When the extension sends MCP notifications/tools/list_changed, rmcp re-fetches tools. For custom browser events, use MCP notifications with custom method names that get injected into the session via watcher_input_tx
  Content scripts run in an ISOLATED WORLD — they share the page's DOM but NOT the page's JavaScript context. navigator.modelContext is only accessible from the page's MAIN world. To bridge this, use chrome.scripting.executeScript({ world: 'MAIN' }) to inject scripts into the page context. These main-world scripts communicate back via window.postMessage(), which the content script relays to the service worker via chrome.runtime.sendMessage.
  Architecture layers: Main-World Injected Script (WebMCP tool discovery/invocation per tab, runs in page context) → Content Script (isolated world relay, postMessage ↔ chrome.runtime bridge) → Service Worker (tool registry, browser control APIs, event aggregation, native messaging client) → Native Messaging Host (Node.js, runs Streamable HTTP MCP server with GET-based SSE for persistent notifications) → ConnectMCP (fspec agent client)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The extension MUST expose a Streamable HTTP MCP server (default port 19876) that the agent connects to via ConnectMCP(transport: 'http', url: 'http://localhost:19876/mcp')
  #   2. The extension MUST discover WebMCP tools registered by websites via navigator.modelContext and expose them as MCP tools namespaced as webmcp__<origin>__<toolName>
  #   3. The extension MUST provide native browser control tools: navigate, screenshot, get_page_content, list_tabs, switch_tab, close_tab, click_element, fill_form, execute_script
  #   4. The extension source code lives in the extension/ directory at project root with its own package.json, build system (Vite/esbuild + TypeScript), and manifest.json
  #   5. The extension requires Chrome 146+ with the WebMCP for testing flag enabled (chrome://flags/#web-mcp) for WebMCP tool discovery; browser control tools work on any Chrome version
  #   6. Browser-to-agent events (page navigation, WebMCP tool registration changes, console errors) MUST be sent as MCP server-initiated notifications via the GET-based SSE stream of Streamable HTTP transport (client opens GET /mcp to receive persistent server→client notifications)
  #   7. The extension MUST be a Manifest V3 Chrome extension with a service worker background script, content scripts (isolated world relay), main-world injected scripts for WebMCP tool discovery/invocation, and a popup UI for connection status
  #   8. Connection lifecycle: ConnectMCP connects → POST /mcp initialize handshake → GET /mcp opens persistent SSE stream for server→client notifications; ConnectMCP disconnects or session ends → DELETE /mcp → SSE stream closes
  #
  # EXAMPLES:
  #   1. Agent calls ConnectMCP(transport: 'http', url: 'http://localhost:19876/mcp') → extension responds with MCP initialize → tools/list returns browser_navigate, browser_screenshot, list_tabs, plus any WebMCP tools from open tabs
  #   2. Agent calls mcp__ext__browser_navigate({url: 'https://example.com'}) → extension navigates active tab → returns page title and final URL after redirects
  #   3. Agent calls mcp__ext__browser_screenshot({tabId: 123, fullPage: true}) → extension captures full-page screenshot using chrome.tabs.captureVisibleTab → returns base64 PNG image
  #   4. User clicks a link that navigates to new URL → extension fires MCP notification {method: 'notifications/browser/navigation', params: {tabId: 123, url: 'https://new-page.com', title: 'New Page'}} → agent receives via SSE and can react
  #   5. Extension popup shows: server status (listening/stopped), port number, connected clients count, list of available tools grouped by source (native vs WebMCP per tab)
  #   6. User navigates to a WebMCP-enabled travel site → site calls navigator.modelContext.registerTool({name: 'searchFlights', ...}) → main-world injected script detects registration → extension adds webmcp__travel-demo.bandarra.me__searchFlights to MCP tool list → sends notifications/tools/list_changed to agent via GET SSE stream
  #   7. WebMCP-enabled site unregisters a tool via navigator.modelContext.unregisterTool('oldTool') → main-world discovery script detects removal → extension removes from tool list → sends notifications/tools/list_changed via GET SSE stream → agent's next tools/list call reflects updated list
  #   8. Agent calls webmcp__example.com__submitForm({name: 'John', email: 'john@test.com'}) → extension injects main-world script via chrome.scripting.executeScript({world: 'MAIN'}) → main-world script calls the WebMCP tool's execute() function in the page context → result relayed back via postMessage → content script → service worker → native host → agent receives structured result
  #
  # ========================================

  Background: User Story
    As a AI agent developer
    I want to connect to a Chrome extension via ConnectMCP and interact with WebMCP-enabled websites bidirectionally
    So that I can discover and call structured website tools, control browser tabs, and receive real-time browser events without fragile DOM scraping or screen automation

  # -----------------------------------------------------------
  # Connection & Initialization
  # -----------------------------------------------------------

  @connection @EXT-003
  Scenario: Connect to extension and discover available tools
    Given the fspec Browser Agent Chrome extension is installed and running
    And the native messaging host is listening on port 19876
    When the agent calls ConnectMCP with transport "http" and url "http://localhost:19876/mcp"
    Then the extension responds with a successful MCP initialize handshake
    And tools/list returns native browser control tools including "browser_navigate", "browser_screenshot", and "browser_list_tabs"
    And the agent receives an Mcp-Session-Id header for subsequent requests

  @connection @EXT-003
  Scenario: Connection lifecycle with SSE stream and session termination
    Given the agent has an active MCP connection to the extension
    And a GET /mcp SSE stream is open for server-to-client notifications
    When the agent disconnects via ConnectMCP
    Then a DELETE /mcp request is sent to terminate the session
    And the SSE stream closes
    And all mcp__ext__ tools are removed from the agent's tool list

  @native-messaging @EXT-003
  Scenario: Native messaging host reads and writes Chrome native messaging frames
    Given the native messaging host process is running
    When the Chrome extension sends a JSON message via stdin with a 4-byte little-endian length prefix
    Then the host reads the length prefix and parses the JSON payload
    And the host can write responses to stdout using the same 4-byte length prefix format

  @native-messaging @EXT-003
  Scenario: Route tool call from MCP client through native messaging to extension
    Given the native messaging host is listening on port 19876
    And a Chrome extension is connected via native messaging stdin/stdout
    When the agent sends a POST /mcp request with a tools/call JSON-RPC message
    Then the host writes a native messaging frame to stdout with a correlation ID
    And the host holds the HTTP response open until the extension replies on stdin
    And the host returns the extension's response as a JSON-RPC result to the agent

  @sse @EXT-003
  Scenario: SSE notification stream delivers extension events to agent
    Given the agent has an active MCP session with a valid Mcp-Session-Id
    When the agent opens a GET /mcp request with Accept header "text/event-stream"
    Then the server responds with status 200 and Content-Type "text/event-stream"
    And the SSE stream stays open for the duration of the session
    And when the extension sends a browser event via stdin the host writes it as an SSE data line

  @registration @EXT-003
  Scenario: Register native messaging host with Chrome
    Given the native messaging host script exists at extension/host/native-host.js
    When the user runs the host with "--register" flag and "--extension-id" with a valid Chrome extension ID
    Then the host writes a com.fspec.browser.agent.json manifest to the platform-specific Chrome NativeMessagingHosts directory
    And the manifest contains the correct host name "com.fspec.browser.agent"
    And the manifest contains the absolute path to the host script
    And the manifest contains the extension ID in allowed_origins

  @error-handling @EXT-003
  Scenario: Reject requests with missing session ID
    Given the native messaging host MCP server is running
    And a session has been initialized with a valid Mcp-Session-Id
    When a POST /mcp request arrives without an Mcp-Session-Id header for a non-initialize method
    Then the server responds with status 400 Bad Request

  @error-handling @EXT-003
  Scenario: Reject requests with invalid session ID
    Given the native messaging host MCP server is running
    When a POST /mcp request arrives with an Mcp-Session-Id that does not match any active session
    Then the server responds with status 404 Not Found

  # -----------------------------------------------------------
  # Service Worker & Content Script Message Routing
  # -----------------------------------------------------------

  @messaging @EXT-004
  Scenario: Service worker connects to native messaging host on startup
    Given the fspec Browser Agent Chrome extension is installed
    When the service worker activates
    Then the service worker calls chrome.runtime.connectNative with host name "com.fspec.browser.agent"
    And a native messaging port is established for bidirectional communication
    And the service worker logs the connection status

  @messaging @EXT-004
  Scenario: Service worker relays tool calls between native host and content scripts
    Given the fspec Browser Agent Chrome extension is installed and running
    And the service worker has an active native messaging connection to the host
    When the native host sends a tool call message with a correlation ID
    Then the service worker routes the call to the appropriate handler
    And the service worker sends the result back to the native host via the native messaging port with the matching correlation ID

  @messaging @EXT-004
  Scenario: Content script relays WebMCP tool registration from main world to service worker
    Given the content script is running on a web page in tab 42
    When the main-world script posts a message with type "FSPEC_WEBMCP_TOOL_REGISTERED" and tool metadata
    Then the content script forwards the message to the service worker via chrome.runtime.sendMessage
    And the service worker receives the message with the sender tab ID 42
    And the service worker updates its internal tool registry
    And the service worker forwards a TOOLS_CHANGED message to the native host

  @messaging @EXT-004
  Scenario: Service worker routes tool invocation to correct tab via content script
    Given the service worker has WebMCP tools registered from tab 42
    When the native host sends a tool call for a WebMCP tool on tab 42
    Then the service worker sends the invocation request to tab 42 via chrome.tabs.sendMessage
    And the content script relays the request to the main world via window.postMessage
    And the main world executes the tool and posts the result back
    And the content script relays the result to the service worker via chrome.runtime.sendMessage
    And the service worker returns the result to the native host

  @messaging @EXT-004
  Scenario: Service worker handles native messaging port disconnect and reconnects
    Given the service worker has an active native messaging connection
    When the native messaging port disconnects
    Then the service worker detects the disconnection via port.onDisconnect
    And the service worker waits before attempting reconnection
    And the service worker establishes a new native messaging connection

  @messaging @EXT-004
  Scenario: Service worker responds to status queries from popup
    Given the service worker is running with an active native messaging connection
    And the tool registry contains 5 tools
    When the popup sends a message with type "FSPEC_GET_STATUS"
    Then the service worker responds with connection status, tool count, and native messaging state

  # -----------------------------------------------------------
  # Native Browser Control Tools
  # -----------------------------------------------------------

  @browser-control @EXT-005
  Scenario: Navigate browser tab to URL
    Given the agent has an active MCP connection to the extension
    When the agent calls mcp__ext__browser_navigate with url "https://example.com"
    Then the extension navigates the active tab to "https://example.com"
    And the tool returns the final URL after any redirects
    And the tool returns the page title

  @browser-control @EXT-005
  Scenario: Capture full-page screenshot
    Given the agent has an active MCP connection to the extension
    And tab 123 is displaying a web page
    When the agent calls mcp__ext__browser_screenshot with tabId 123 and fullPage true
    Then the extension captures a screenshot using chrome.tabs.captureVisibleTab
    And the tool returns a base64-encoded PNG image

  @browser-control @EXT-005
  Scenario: List all open browser tabs
    Given the agent has an active MCP connection to the extension
    And the browser has multiple tabs open
    When the agent calls mcp__ext__browser_list_tabs
    Then the tool returns a list of all open tabs with their IDs, URLs, and titles

  @browser-control @EXT-005
  Scenario: Execute JavaScript in a browser tab
    Given the agent has an active MCP connection to the extension
    When the agent calls mcp__ext__browser_execute_script with code "document.title"
    Then the extension executes the script in the active tab
    And the tool returns the script result

  @browser-control @EXT-005
  Scenario: Switch to a specific browser tab
    Given the agent has an active MCP connection to the extension
    And tab 42 exists in the browser
    When the agent calls mcp__ext__browser_switch_tab with tabId 42
    Then the extension activates tab 42 and focuses its window
    And the tool returns confirmation with the tab info

  @browser-control @EXT-005
  Scenario: Close a browser tab
    Given the agent has an active MCP connection to the extension
    And tab 42 is open with url "https://example.com"
    When the agent calls mcp__ext__browser_close_tab with tabId 42
    Then the extension closes tab 42
    And the tool returns confirmation with the closed tab's URL

  @browser-control @EXT-005
  Scenario: Get page content as text
    Given the agent has an active MCP connection to the extension
    And the active tab is displaying a web page
    When the agent calls mcp__ext__browser_get_page_content with format "text"
    Then the tool returns the page title, URL, and inner text content

  @browser-control @EXT-005
  Scenario: Get page content as HTML
    Given the agent has an active MCP connection to the extension
    And the active tab is displaying a web page
    When the agent calls mcp__ext__browser_get_page_content with format "html"
    Then the tool returns the page title, URL, and outer HTML content

  @browser-control @EXT-005
  Scenario: Click an element on the page
    Given the agent has an active MCP connection to the extension
    And the active tab contains an element matching selector "#submit-btn"
    When the agent calls mcp__ext__browser_click_element with selector "#submit-btn"
    Then the extension clicks the element matching the selector
    And the tool returns confirmation that the element was clicked

  @browser-control @EXT-005
  Scenario: Click element fails when selector not found
    Given the agent has an active MCP connection to the extension
    And the active tab does not contain an element matching selector "#nonexistent"
    When the agent calls mcp__ext__browser_click_element with selector "#nonexistent"
    Then the tool returns an error indicating the element was not found

  @browser-control @EXT-005
  Scenario: Fill a form field on the page
    Given the agent has an active MCP connection to the extension
    And the active tab contains an input element matching selector "#email"
    When the agent calls mcp__ext__browser_fill_form with selector "#email" and value "test@example.com"
    Then the extension sets the input value and dispatches input and change events
    And the tool returns confirmation with the selector and value

  @browser-control @EXT-005
  Scenario: Navigate browser history backward
    Given the agent has an active MCP connection to the extension
    When the agent calls mcp__ext__browser_go_back
    Then the extension navigates the active tab back in history
    And the tool returns confirmation of the navigation direction

  @browser-control @EXT-005
  Scenario: Navigate browser history forward
    Given the agent has an active MCP connection to the extension
    When the agent calls mcp__ext__browser_go_forward
    Then the extension navigates the active tab forward in history
    And the tool returns confirmation of the navigation direction

  @browser-control @integration @EXT-005
  Scenario: All native browser tools are listed in tools/list response
    Given the agent has an active MCP connection to the extension
    When the agent calls tools/list
    Then the response includes all 11 native browser control tools
    And each tool has a name, description, and inputSchema

  # -----------------------------------------------------------
  # Bidirectional Browser Events (Server → Agent)
  # -----------------------------------------------------------

  @events @EXT-007
  Scenario: Receive navigation event when user navigates to new URL
    Given the agent has an active MCP connection to the extension
    And a GET /mcp SSE stream is open for notifications
    When the user clicks a link that navigates tab 123 to "https://new-page.com"
    Then the extension fires an MCP notification with method "notifications/browser/navigation"
    And the notification params include tabId 123, url "https://new-page.com", and title "New Page"
    And the agent receives the notification via the SSE stream

  @events @EXT-007
  Scenario: Receive tool list changed notification when WebMCP tools are discovered
    Given the agent has an active MCP connection to the extension
    And a GET /mcp SSE stream is open for notifications
    When a website registers a new WebMCP tool via navigator.modelContext.registerTool
    Then the extension sends a "notifications/tools/list_changed" MCP notification via SSE
    And the agent's next tools/list call includes the newly discovered WebMCP tool

  # -----------------------------------------------------------
  # WebMCP Tool Discovery
  # -----------------------------------------------------------

  @webmcp @EXT-006
  Scenario: Discover WebMCP tool registered by website
    Given the agent has an active MCP connection to the extension
    And the user navigates to a WebMCP-enabled site at "https://travel-demo.bandarra.me"
    When the site calls navigator.modelContext.registerTool with name "searchFlights"
    Then the main-world injected script detects the tool registration
    And the extension adds "webmcp__travel-demo.bandarra.me__searchFlights" to the MCP tool list
    And the agent receives a "notifications/tools/list_changed" notification via SSE

  @webmcp @EXT-006
  Scenario: Remove WebMCP tool when website unregisters it
    Given the agent has an active MCP connection to the extension
    And the extension has a discovered WebMCP tool "webmcp__example.com__oldTool"
    When the website calls navigator.modelContext.unregisterTool with name "oldTool"
    Then the main-world discovery script detects the removal
    And the extension removes "webmcp__example.com__oldTool" from the tool list
    And the agent receives a "notifications/tools/list_changed" notification via SSE
    And the agent's next tools/list call no longer includes "webmcp__example.com__oldTool"

  # -----------------------------------------------------------
  # WebMCP Tool Invocation
  # -----------------------------------------------------------

  @webmcp @EXT-006
  Scenario: Invoke a WebMCP tool registered by a website
    Given the agent has an active MCP connection to the extension
    And the website at "https://example.com" has registered a WebMCP tool "submitForm"
    When the agent calls webmcp__example.com__submitForm with params name "John" and email "john@test.com"
    Then the extension injects a main-world script via chrome.scripting.executeScript with world "MAIN"
    And the main-world script calls the WebMCP tool's execute function in the page context
    And the result is relayed back via postMessage to the content script
    And the content script forwards the result to the service worker via chrome.runtime
    And the agent receives the structured result from the tool call

  # -----------------------------------------------------------
  # Extension Popup UI
  # -----------------------------------------------------------

  @popup @EXT-008
  Scenario: Popup displays connection status and available tools
    Given the fspec Browser Agent Chrome extension is installed
    When the user opens the extension popup
    Then the popup shows the server status as "listening" or "stopped"
    And the popup shows the configured port number
    And the popup shows the count of connected clients
    And the popup shows available tools grouped by source as native and WebMCP per tab

  # -----------------------------------------------------------
  # Extension Structure
  # -----------------------------------------------------------

  @structure @EXT-002
  Scenario: Extension source code lives in extension directory
    Given the fspec repository is cloned
    When I inspect the extension directory at project root
    Then the extension directory contains its own package.json
    And the extension directory contains a manifest.json for Manifest V3
    And the extension directory contains a TypeScript build system
    And the manifest.json declares a service worker background script
    And the manifest.json declares content scripts for all URLs

  @EXT-002
  @structure
  Scenario: Extension builds and produces loadable Chrome extension output
    Given the extension directory contains package.json and build configuration
    When I run the build command in the extension directory
    Then the build produces dist/service-worker.js as an ES module
    And the build produces dist/content-script.js
    And the build produces dist/popup.js and popup.html
    And the manifest.json references the built files correctly
    And the manifest.json includes required permissions for activeTab, tabs, scripting, storage, offscreen, and nativeMessaging


  @events
  @EXT-007
  Scenario: Receive tab created event when user opens a new tab
    Given the agent has an active MCP connection to the extension
    And a GET /mcp SSE stream is open for notifications
    When the user opens a new browser tab
    Then the extension fires an MCP notification with method "notifications/browser/tab_created"
    And the notification params include the new tabId and url
    And the agent receives the notification via the SSE stream


  @events
  @EXT-007
  Scenario: Receive tab closed event when user closes a tab
    Given the agent has an active MCP connection to the extension
    And a GET /mcp SSE stream is open for notifications
    When the user closes browser tab 123
    Then the extension fires an MCP notification with method "notifications/browser/tab_closed"
    And the notification params include tabId 123
    And the agent receives the notification via the SSE stream


  @events
  @EXT-007
  Scenario: Receive load complete event when page finishes loading
    Given the agent has an active MCP connection to the extension
    And a GET /mcp SSE stream is open for notifications
    When tab 123 finishes loading the page at "https://example.com"
    Then the extension fires an MCP notification with method "notifications/browser/load_complete"
    And the notification params include tabId 123, url "https://example.com", and title
    And the agent receives the notification via the SSE stream


  @events
  @EXT-007
  Scenario: Closing tab with WebMCP tools triggers both tab closed and tools changed notifications
    Given the agent has an active MCP connection to the extension
    And a GET /mcp SSE stream is open for notifications
    And tab 456 has WebMCP tools registered
    When the user closes browser tab 456
    Then the WebMCP tools from tab 456 are unregistered from the tool registry
    And the agent receives a "notifications/browser/tab_closed" notification
    And the agent receives a "notifications/tools/list_changed" notification

