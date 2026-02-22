@feature-management
@cli
@done
@session-management
@tui
@GIT-031
Feature: BoardView CreateSessionDialog callback ignores isolated parameter
  """
  BoardView's CreateSessionDialog onConfirm callback must accept isolated parameter and pass it to handleCreateSessionConfirm. Follow AgentView pattern (line 8060). Integration point: BoardView.tsx lines 612-618.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BoardView must pass isolated parameter from CreateSessionDialog to session creation logic
  #   2. When isolated=true, must call createIsolatedSession instead of createSession
  #
  # EXAMPLES:
  #   1. User opens dialog from board, toggles Isolated ON, confirms, session created via createIsolatedSession
  #   2. User opens dialog from board, leaves Isolated OFF (default), confirms, session created via createSession
  #
  # ========================================
  Background: User Story
    As a user
    I want to confirm session creation with isolated toggle enabled
    So that create an isolated session with git worktree

  Scenario: Create isolated session when toggle is enabled
    Given I am viewing the Create Session dialog from the board
    When I toggle Isolated mode ON and confirm
    Then an isolated session should be created with a git worktree

  Scenario: Create normal session when toggle is disabled
    Given I am viewing the Create Session dialog from the board
    When I leave Isolated mode OFF (default) and confirm
    Then a normal session should be created without a git worktree
