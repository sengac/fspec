@WATCH-004
Feature: Session Role and Authority Model

  """
  Add SessionRole struct and RoleAuthority enum in session_manager.rs near SessionStatus
  Add role: RwLock<Option<SessionRole>> field to BackgroundSession struct
  Add set_role() and get_role() methods to BackgroundSession impl
  Add NAPI functions session_set_role and session_get_role with #[napi] attribute
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionRole is a struct with name (String), description (Option<String>), and authority (RoleAuthority)
  #   2. RoleAuthority is an enum with two variants: Peer (equal authority, can observe but not override) and Supervisor (elevated authority, can inject directives)
  #   3. BackgroundSession has an optional role field (Option<SessionRole>) - None for regular sessions, Some for watcher sessions
  #   4. NAPI bindings expose session_set_role(session_id, role_name, role_description, authority) and session_get_role(session_id)
  #   5. Role can be set at any time during session lifetime (mutable)
  #   6. Default authority when not specified is Peer (safer default)
  #
  # EXAMPLES:
  #   1. session_set_role(session_id, 'code-reviewer', 'Reviews code changes', 'peer') → session.role = SessionRole { name: 'code-reviewer', description: Some('Reviews code changes'), authority: Peer }
  #   2. session_set_role(session_id, 'supervisor', None, 'supervisor') → session.role = SessionRole { name: 'supervisor', description: None, authority: Supervisor }
  #   3. session_get_role(session_id) on regular session (no role set) → returns None
  #   4. session_get_role(session_id) on watcher session with role → returns SessionRole with name, description, authority
  #   5. session_set_role with invalid authority string → returns error 'Invalid authority: must be peer or supervisor'
  #   6. session_set_role with empty name → returns error 'Role name cannot be empty'
  #
  # ========================================

  Background: User Story
    As a session management system
    I want to distinguish between regular sessions and watcher sessions with different authority levels
    So that watchers can be configured as peers (equal authority) or supervisors (override authority) for different use cases

  @unit
  Scenario: Set peer role with description
    Given a BackgroundSession exists
    When I call set_role with name "code-reviewer", description "Reviews code changes", and authority "peer"
    Then the session role should have name "code-reviewer"
    And the session role should have description "Reviews code changes"
    And the session role should have authority Peer

  @unit
  Scenario: Set supervisor role without description
    Given a BackgroundSession exists
    When I call set_role with name "supervisor", no description, and authority "supervisor"
    Then the session role should have name "supervisor"
    And the session role should have no description
    And the session role should have authority Supervisor

  @unit
  Scenario: Get role on regular session returns None
    Given a BackgroundSession exists
    And no role has been set
    When I call get_role
    Then it should return None

  @unit
  Scenario: Get role on session with role returns role details
    Given a BackgroundSession exists
    And the role has been set to name "test-role" with authority Peer
    When I call get_role
    Then it should return a SessionRole with name "test-role" and authority Peer

  @unit
  Scenario: Set role with invalid authority returns error
    Given a BackgroundSession exists
    When I call set_role with name "test", description None, and authority "invalid"
    Then it should return an error "Invalid authority: must be peer or supervisor"

  @unit
  Scenario: Set role with empty name returns error
    Given a BackgroundSession exists
    When I call set_role with name "", description None, and authority "peer"
    Then it should return an error "Role name cannot be empty"
