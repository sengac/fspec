@done
@high
@integration
@agent-manager
@tools
@BUG-240
Feature: Clarify AgentManager caller attribution versus spawned-session work-unit focus
  """
  Implementation:
  - Update the shared schema decorator and AgentManager model-visible guidance, not admission behavior. Shared and legacy NAPI management routes both use the shared AgentManagerTool. Do not add a child initialization argument.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Root work_unit_id always attributes the calling session, never a spawned or targeted session, and does not change authorization.
  #   2. AgentManager guidance must explain orchestration on card A while delegating card B; role and message text do not assign authoritative worker focus. Workers attribute their own calls.
  #   3. Required attribution validation, exact NONE clearing, business argument stripping, session isolation and authorization bindings remain unchanged. No child-focus initialization API is added.
  #
  # EXAMPLES:
  #   1. Published tool definitions explicitly identify the caller and distinguish spawned or targeted sessions.
  #   2. A caller attributes spawn to AUTH-001 while role text delegates AUTH-002; the child starts cleared until its own admitted call.
  #   3. Targeted management actions leave target focus unchanged; NONE clears only the caller and invalid attribution never invokes the handler.
  #
  # ========================================
  Background: User Story
    As an agent coordinating concurrent sessions
    I want to distinguish my invocation attribution from the worker task
    So that delegation never accidentally changes the wrong session focus

  Scenario: Published definitions distinguish caller attribution from delegation
    Given an agent requesting ordinary and AgentManager tool definitions
    When the model-visible definitions are published
    Then root attribution explicitly names the calling session and excludes spawned or targeted sessions
    And AgentManager explains orchestration on AUTH-001 while delegating AUTH-002 and workers attributing their own calls
    And required validation NONE clearing and permission separation remain documented

  Scenario: Spawning a worker does not assign caller attribution to the child
    Given a caller focused on AUTH-003 with authorization bound to AUTH-001
    When it spawns a worker with attribution AUTH-001 and role text delegating AUTH-002
    Then the caller focuses AUTH-001 and the new worker has cleared activity
    And the worker selects AUTH-002 only through its own admitted call without changing the caller
    And caller authorization and worktree bindings remain unchanged

  Scenario: Targeted management preserves session isolation and strict admission
    Given a caller and worker focused on different project cards
    When the caller manages the worker using explicit attribution and NONE
    Then only caller activity changes and worker focus is unchanged
    And invalid missing null empty and foreign card attribution never reaches the management handler
    And the business handler receives no root attribution and authorization remains unchanged
