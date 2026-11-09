@done
@BUG-427
@tui
@multi-workspace
@rust
Feature: The board creates isolated sessions in the selected workspace
  """
  fspec-tui app/session_creation_tasks.rs handled isolated creation before the
  workspace branch and dropped the selected workspace. Isolated creation with a
  selected workspace now calls FspecBackend::create_isolated_session_in_workspace
  (both transports and the test mock implement it).
  Tests: rust/fspec-tui/tests/bug427_board_isolated_session_workspace.rs.
  """

  Background: User Story
    As a user working on a card in a non-start workspace
    I want to create an isolated session for it in that workspace's repository
    So that the worktree is cut from the right repository instead of the daemon's start workspace

  Scenario: An isolated agent starts in the selected workspace
    Given the user has selected workspace "beta"
    When the user creates a new isolated agent from the board
    Then the isolated session is created in beta's path
