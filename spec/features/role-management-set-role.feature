@done
@agent-manager
@AMGR-012
Feature: Role management — set_role AgentManager action

  """
  set_role action dispatched in agent_manager handler — calls session_set_role(session_id, role_name, None, None) NAPI binding; empty role calls session_clear_role or sets empty
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. set_role AgentManager action sets or replaces the role on a target session — empty/null role clears it
  #   2. set_role action defaults session_id to caller's own session when not specified
  #   3. Role appears in get_status and list action responses
  #
  # EXAMPLES:
  #   1. Agent calls set_role action with role='test-helper' and no session_id → role set on caller's own session → response: { session_id: 'own-id', role: 'test-helper' }
  #   2. Agent calls set_role with session_id='sub-123' and role='code-reviewer' → role set on subordinate session → response: { session_id: 'sub-123', role: 'code-reviewer' }
  #   3. Agent calls set_role with role='' (empty string) → role cleared → response: { session_id: 'target-id', role: null }
  #   4. Agent calls set_role with session_id='nonexistent' → error response: { error: true, code: 'session_not_found', message: '...' }
  #   5. Agent calls get_status for session with role='architect' → response includes role: 'architect' field
  #   6. Agent calls list → sessions with roles show role field in their entries
  #
  # ========================================

  Background: User Story
    As a user
    I want to set or edit a role on any session via the AgentManager set_role action
    So that I can customize the system prompt overlay for sessions to specialize their behavior

  Scenario: set_role on own session with no session_id specified
    Given I have a session with ID "own-session"
    When I call the set_role action with role "test-helper" and no session_id
    Then the role is set on the caller's own session
    And the response contains session_id "own-session" and role "test-helper"

  Scenario: set_role on a specific session by ID
    Given a session exists with ID "sub-123"
    When I call the set_role action with session_id "sub-123" and role "code-reviewer"
    Then the role is set on session "sub-123"
    And the response contains session_id "sub-123" and role "code-reviewer"

  Scenario: Clear role with empty string
    Given a session exists with ID "target-id" and role "old-role"
    When I call the set_role action with session_id "target-id" and role ""
    Then the role is cleared on session "target-id"
    And the response contains session_id "target-id" and role null

  Scenario: set_role on non-existent session returns error
    When I call the set_role action with session_id "nonexistent" and role "any"
    Then an error response is returned with code "session_not_found"

  Scenario: Role appears in get_status response
    Given a session exists with ID "sess-1" and role "architect"
    When I call the get_status action for session "sess-1"
    Then the response includes role "architect"

  Scenario: Role appears in list response
    Given sessions exist with roles set
    When I call the list action
    Then each session entry includes its role field
