@CMPCT-008
Feature: inject_summary Tool Definition and Schema

  """
  File: codelet/tools/src/inject_summary.rs — contains InjectSummaryTool struct, InjectSummaryArgs, InjectSummaryResult, InjectSummaryHandler type alias, global handler registry, set/has/execute/clear functions
  Follows exact pattern of fspec_handler.rs and session_search/handler.rs — per-session Arc<dyn Fn + Send + Sync> stored in global Lazy<RwLock<HashMap<Uuid, Handler>>>
  Consumer: CMPCT-009 (inject_summary_handler.rs in codelet/napi) creates the actual handler closure and registers it via set_inject_summary_handler(). CMPCT-011 triggers the flow that causes the agent to call inject_summary.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool must implement rig::tool::Tool trait with name 'inject_summary'
  #   2. Tool parameters: content (String, required) — the DAG summary content to inject
  #   3. Tool return type: InjectSummaryResult with injected_tokens (u64) and remaining_budget (u64)
  #   4. Handler type alias: pub type InjectSummaryHandler = Arc<dyn Fn(Uuid, String) -> InjectSummaryResult + Send + Sync> — takes session_id and content
  #   5. Global handler registry uses once_cell::sync::Lazy<RwLock<HashMap<Uuid, InjectSummaryHandler>>> like SessionSearchHandler and FspecHandler
  #   6. Tool::call() dispatches to registered handler; returns ToolError when no handler is registered for the session
  #   7. Tool is constructed with session_id (like SessionSearchTool) for per-session handler lookup
  #   8. Module and types must be re-exported from codelet/tools/src/lib.rs
  #   9. JSON schema for parameters validates that content is a required string field
  #   10. Tool description must explain: pins DAG summary as system-level content, drops builder turns, persists across future compactions
  #
  # EXAMPLES:
  #   1. Agent calls inject_summary({content: '# D2: Architecture\n- Using JWT...\n# D1: Current Arc\n...'}) → handler receives content string, returns {injected_tokens: 1250, remaining_budget: 185000}
  #   2. Agent calls inject_summary with no handler registered → returns ToolError with message 'inject_summary handler not configured for session {uuid}'
  #   3. Session A and Session B each have their own InjectSummaryHandler — calling inject_summary on Session A dispatches to Session A's handler, not Session B's
  #   4. set_inject_summary_handler(session_id, None) removes the handler — subsequent calls return error
  #   5. InjectSummaryTool::new(session_id) creates tool scoped to that session, tool.call(args) looks up handler by stored session_id
  #   6. Tool definition() returns JSON schema with content as required string property and description explaining DAG pinning
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to call inject_summary to pin a hierarchical DAG summary as persistent system-level content
    So that complete my self-directed context compression cycle and continue working with compact context

  @tool-trait
  Scenario: Tool implements Rig Tool trait with correct name and schema
    Given the InjectSummaryTool is compiled as part of the codelet-tools crate
    When the tool definition is requested
    Then the tool name should be "inject_summary"
    And the JSON schema should have "content" as a required string parameter
    And the description should explain DAG pinning, builder turn dropping, and compaction persistence

  @handler-dispatch
  Scenario: Successful inject_summary dispatches to registered handler
    Given a session with a registered InjectSummaryHandler
    When the agent calls inject_summary with DAG content
    Then the handler receives the session_id and content string
    And the tool returns InjectSummaryResult with injected_tokens and remaining_budget

  @error-handling
  Scenario: inject_summary with no handler returns error
    Given no InjectSummaryHandler is registered for the session
    When the agent calls inject_summary
    Then the tool returns a ToolError with message containing "not configured"
    And the error message includes the session UUID

  @session-isolation
  Scenario: Concurrent sessions have isolated handlers
    Given Session A and Session B each have a registered InjectSummaryHandler
    When inject_summary is called on Session A
    Then only Session A's handler is invoked
    And Session B's handler is not affected

  @handler-cleanup
  Scenario: Handler removal via set_inject_summary_handler with None
    Given a session with a registered InjectSummaryHandler
    When set_inject_summary_handler is called with None for that session
    Then has_inject_summary_handler returns false for that session
    And subsequent inject_summary calls return an error

  @session-scoping
  Scenario: Tool is constructed with session_id for handler lookup
    Given an InjectSummaryTool created with a specific session_id
    When call() is invoked with InjectSummaryArgs
    Then the tool looks up the handler using its stored session_id
    And the correct per-session handler is dispatched

  @lib-exports
  Scenario: Tool and handler types are exported from lib.rs
    Given the codelet-tools crate lib.rs module declarations
    When inject_summary module is declared
    Then InjectSummaryTool is publicly re-exported
    And InjectSummaryHandler type alias is publicly re-exported
    And set_inject_summary_handler function is publicly re-exported
    And has_inject_summary_handler function is publicly re-exported
    And clear_all_inject_summary_handlers function is publicly re-exported
