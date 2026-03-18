@TUI-081
Feature: Role banner display in AgentView
  """
  New RoleBanner component at src/tui/components/RoleBanner.tsx — reads role from sessionGetRole NAPI binding, renders conditionally below SessionHeader in AgentView.
  RoleBanner integrated into AgentView.tsx between SessionHeader and conversation VirtualList — role state refreshed via existing refreshRustState polling cycle.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a session has an active role, a RoleBanner is displayed directly below the SessionHeader border
  #   2. When no role is set, the RoleBanner is not rendered (zero height)
  #   3. The RoleBanner shows the role text as a single line with cyan 'Role:' prefix and dimmed role text, truncated for long roles
  #   4. The RoleBanner updates when /role dialog is submitted, when set_role is called, or when switching sessions
  #
  # EXAMPLES:
  #   1. Session has role 'security reviewer' → banner shows 'Role: security reviewer'
  #   2. Session has no role → no banner, no gap
  #   3. User sets role via /role → banner appears immediately
  #   4. Very long role text → truncated with ellipsis
  #
  # ========================================
  Background: User Story
    As a developer
    I want to see my active role displayed in the AgentView header area
    So that I know at a glance what role is currently set on the session

  @unit
  Scenario: RoleBanner shows active role text
    Given a session with role set to "security reviewer"
    When the AgentView renders
    Then a RoleBanner is displayed below the SessionHeader border
    And the banner shows "Role:" prefix in cyan
    And the banner shows "security reviewer" as dimmed text

  @unit
  Scenario: RoleBanner hidden when no role set
    Given a session with no role set
    When the AgentView renders
    Then no RoleBanner is displayed
    And there is no empty gap between SessionHeader and conversation

  @unit
  Scenario: RoleBanner appears after setting role via /role dialog
    Given a session with no role set
    When the user submits "code reviewer" via the /role dialog
    Then a RoleBanner appears showing "Role: code reviewer"

  @unit
  Scenario: Long role text is truncated
    Given a session with a very long role text
    When the AgentView renders
    Then the RoleBanner displays the role text truncated to fit the terminal width
