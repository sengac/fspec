@EXT-009
Feature: WebMCP dynamic tool registration not detected — polyfill libraries bypass navigator.modelContext

  """
  Layered discovery in webmcp-discovery.ts: Layer 1 navigator.modelContext (existing), Layer 2 WebMCP.prototype interception, Layer 3 post-load snapshot, Layer 4 ModelContextTesting (opportunistic). manifest.json run_at changed to document_start. Injector uses injectImmediately: true.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The discovery script must intercept navigator.modelContext.registerTool() for native Chrome WebMCP API and W3C-compliant polyfills
  #   2. The discovery script must intercept WebMCP class prototype registerTool() for polyfill libraries that bypass navigator.modelContext
  #   3. A post-load snapshot must scan well-known globals (window.webMCP, window.mcp) to discover tools registered before injection
  #   4. Content script must run at document_start (not document_idle) to avoid missing early tool registrations
  #   5. The MAIN-world script must be injected as early as possible using injectImmediately when available
  #   6. Double-injection guard must prevent the same discovery script from running twice on the same page
  #   7. Tool execute callbacks must be captured from all discovery layers so invocation works regardless of registration source
  #   8. If ModelContextTesting API is available (WebMCPTesting flag enabled), prefer its ontoolchange event and listTools() over monkey-patching
  #
  # EXAMPLES:
  #   1. Site uses new WebMCP().registerTool('calculator', ...) — tool appears in MCP tools/list
  #   2. Site uses navigator.modelContext.registerTool({name: 'weather'}) — tool appears in MCP tools/list (existing behavior)
  #   3. Site registers tool during initial script execution before document_idle — tool still discovered via early injection
  #   4. Site assigns window.webMCP = new WebMCP() after page load — Object.defineProperty trap catches it and wraps registerTool
  #   5. WebMCP tools registered before extension injection are discovered via post-load snapshot of window.webMCP.getTools()
  #   6. Tool registered via WebMCP library can be invoked through MCP and the execute callback fires in page context
  #   7. Discovery script injected twice on same page only runs once (double-injection guard)
  #   8. When ModelContextTesting is available, ontoolchange fires and listTools() catches tools registered before injection
  #
  # ========================================

  Background: User Story
    As a AI agent user
    I want to have WebMCP tools discovered regardless of how the page registers them
    So that I can use dynamically registered tools from any WebMCP-compatible website

  @polyfill-bypass
  Scenario: Discover tools registered via WebMCP library class
    Given the discovery script is active in the page's main world
    And the page creates a WebMCP instance and calls registerTool with name "calculator"
    When the tool registration is intercepted by the WebMCP prototype wrapper
    Then a FSPEC_WEBMCP_TOOL_REGISTERED message is posted with tool name "calculator"
    And the execute callback is stored for later invocation

  @native-api
  Scenario: Discover tools registered via native navigator.modelContext
    Given the discovery script is active in the page's main world
    And navigator.modelContext exists as a native browser API
    When the page calls navigator.modelContext.registerTool with name "weather"
    Then a FSPEC_WEBMCP_TOOL_REGISTERED message is posted with tool name "weather"
    And the execute callback is stored for later invocation

  @injection-timing
  Scenario: Discover tools registered during initial page script execution
    Given the content script runs at document_start
    And the MAIN-world discovery script is injected before page scripts execute
    When the page registers a tool during its initial script execution
    Then the monkey-patch is already in place to intercept the registration
    And the tool appears in the MCP tools list

  @polyfill-bypass
  Scenario: Trap late assignment of WebMCP class on window
    Given the discovery script is active in the page's main world
    And window.WebMCP does not yet exist
    When the page assigns a WebMCP class to window.WebMCP
    Then the Object.defineProperty trap intercepts the assignment
    And the new class's prototype.registerTool is wrapped with the interceptor

  @post-load-snapshot
  Scenario: Discover pre-existing tools via post-load snapshot
    Given the discovery script is active in the page's main world
    And a WebMCP instance on window.webMCP already has tools registered
    When the post-load snapshot runs after a short delay
    Then all tools from the instance's getTools() are discovered
    And FSPEC_WEBMCP_TOOL_REGISTERED messages are posted for each undiscovered tool

  @invocation
  Scenario: Invoke a tool registered via WebMCP library
    Given a tool "calculator" was registered via the WebMCP library
    And the execute callback was captured by the discovery script
    When a FSPEC_INVOKE_TOOL message arrives for tool "calculator" with arguments
    Then the stored execute callback is called with the provided arguments
    And a FSPEC_INVOKE_RESULT message is posted with the result

  @guard
  Scenario: Prevent double-injection of discovery script
    Given the discovery script has already been injected into the page
    And the __fspec_webmcp_discovery_active flag is set on window
    When the discovery script function is called again
    Then the function returns immediately without re-initializing
    And no duplicate interceptors are installed

  @model-context-testing
  Scenario: Use ModelContextTesting API when available
    Given the discovery script is active in the page's main world
    And navigator.modelContext.testing exists with ontoolchange and listTools
    When the ModelContextTesting API is detected
    Then the ontoolchange event handler is registered for real-time notifications
    And listTools() is called to discover tools registered before injection

  @injection-timing
  Scenario: Injector uses early injection strategy
    Given the WebMCP injector is initialized with chrome.scripting and chrome.tabs
    When a tab triggers the injection
    Then the discovery script is injected into the MAIN world
    And the injection uses the earliest available timing
