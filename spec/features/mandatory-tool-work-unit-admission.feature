@done
@TOOL-026
@tools
@session
@tool-execution
@security
@rust
Feature: Validated tool attribution updates only the invoking session
  """
  Inject session-bound admission policy before typed ToolDyn deserialization,
  facade/Rhai mapping and MCP forwarding. Validate against the trusted session
  project, strip only top-level work_unit_id for delegation, and retain original call
  arguments for audit/history. Latest admitted invocation wins, not latest result.
  Activity attribution is not authorization. Ephemeral read-only agents validate
  attribution without changing their owner's visible focus or permission binding.
  """

  Background: User Story
    As a developer supervising concurrent agents
    I want every tool call to validate its declared work before execution
    So that session card tracking is accurate without weakening existing safeguards

  Scenario: Valid attribution changes caller focus before execution and survives tool failure
    Given two sessions focused on different cards
    When one invokes a tool with valid attribution for another project card
    Then the executor observes that caller's updated focus before performing work
    And the other session remains unchanged
    And a subsequent execution error does not roll back admitted activity

  Scenario: Invalid attribution cannot mutate focus or execute a tool
    Given an attributed tool executor with recorded calls and existing focus
    When calls omit metadata or provide null empty unknown or cross-project attribution
    Then each call returns an actionable validation error
    And no underlying tool is invoked and no focus event is emitted
    And a user-supplied session or project target cannot redirect attribution ownership

  Scenario: NONE clears activity without weakening permissions
    Given a session with stage restrictions and isolated-worktree boundaries
    When it invokes a tool attributed to "NONE"
    Then its activity is explicitly cleared and the session remains alive
    And existing stage and path restrictions remain enforced
    And selecting a more permissive unrelated card does not replace its authorization binding

  Scenario: Metadata is removed before strict delegation while audit arguments are retained
    Given strict MCP typed dynamic and Rhai-mapped tools
    When each receives a valid attributed call with nested business arguments
    Then each executor receives exactly its original business payload without top-level "work_unit_id"
    And nested business "work_unit_id" and "target_work_unit_id" arguments are unchanged
    And original attributed arguments remain available for call history and correlation

  Scenario: Invocation order rather than result completion determines activity
    Given calls admitted for "AUTH-001" then "AUTH-002" followed by "NONE"
    When the earlier calls finish late fail or return duplicate results
    Then the final activity remains explicitly cleared
    And late status-command reconciliation cannot resurrect an older card

  Scenario: Child targets and auxiliary agents do not steal another session's focus
    Given a parent session a managed child and an ephemeral read-only research agent
    When the parent invokes a tool targeting its child and the research agent invokes its tools
    Then attribution belongs to the trusted invoking context rather than the target argument
    And research activity does not change the owner's visible card
    And unknown invoking contexts cannot silently bypass required validation

  Scenario: Hot-added tools and completion tools enforce admission as ordinary tools
    Given an agent with dynamically registered tools and a completion contract
    When a new tool or completion call omits required attribution
    Then admission rejects the call before its side effects or completion signal
    And valid attributed calls use the same enforcement after a provider switch
