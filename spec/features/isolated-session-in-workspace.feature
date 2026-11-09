@done
@BUG-427
@session
@multi-workspace
@rust
Feature: Isolated sessions are created in the requested workspace
  """
  SessionManagerHandle::create_isolated_session(role) always used the start
  workspace, and the RPC had no workspace parameter, so an isolated session for
  a card in another workspace was cut from the daemon's start repository.
  SessionManagerHandle::create_isolated_session_in_workspace(workspace, role)
  creates it from the named workspace (canonicalized, must be a directory);
  create_isolated_session delegates with the start workspace. A role that
  cannot be applied is now an error instead of being skipped. The RPC
  create_isolated_session_in_workspace is appended to the frozen wire order
  with its MCP tool.
  Tests: rust/sessions/tests/bug427_isolated_session_in_workspace.rs.
  """

  Background: User Story
    As a user working on a card in a non-start workspace
    I want to create an isolated session for it in that workspace's repository
    So that the worktree is cut from the right repository instead of the daemon's start workspace

  Scenario: An isolated session is cut from the requested workspace
    Given a session manager whose start workspace is one repository
    When an isolated session is created in a second repository
    Then the session's project is the second repository and its worktree belongs to it
