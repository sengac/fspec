@BUG-121
Feature: Submitting empty text in /role dialog does not clear the role

  """
  The NAPI session_set_role binding handles empty role_name by calling session.clear_role() on the Rust Session. AgentView.tsx onSubmit already sends '' for empty submissions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. NAPI session_set_role rejects empty string with error instead of calling session.clear_role()
  #   2. session_set_role with empty role_name must call session.clear_role() and return Ok(())
  #   3. AgentView onSubmit catch block should display the error to the user instead of silently swallowing it
  #
  # EXAMPLES:
  #   1. User sets role to 'reviewer' → opens /role → clears text area → presses Tab+Enter on OK → session_set_role('', ...) called → Rust returns Err('Role name cannot be empty') → role persists, error shown in catch block
  #   2. After fix: User sets role to 'reviewer' → opens /role → clears text area → submits → session_set_role('', ...) calls session.clear_role() → role removed → RoleBanner disappears
  #
  # ========================================

  Background: User Story
    As a user with an active session role
    I want to clear the role by submitting empty text in the /role dialog
    So that the role is removed and the RoleBanner disappears

  @NAPI
  Scenario: session_set_role with empty string clears the role
    Given a session exists with role "reviewer"
    When session_set_role is called with an empty role_name
    Then the session role should be cleared
    And session_get_role should return null
