@RPC-444
@session
@multi-workspace
@BUG-290
Feature: Sessions use their workspace instead of the daemon cwd
  """
  rust/sessions/src/default_project.rs: SessionManager::set_default_project(path) records the
  start workspace (build_service sets it from --workspace or cwd) and default_project() returns
  it, falling back to std::env::current_dir(). The sessions crate's current_dir() call sites
  (create_session_checked, create_isolated_session, worktree prune, blocklist list, schedule CRUD,
  effective cwd fallback) use default_project() instead. SessionManagerHandle gains
  create_session_in_workspace(workspace, role) (default: unsupported), exposed as an RPC that
  first opens the workspace through SharedFspecService::workspace.
  """

  Background: User Story
    As a client of a daemon serving several workspaces
    I want sessions to belong to the workspace I chose
    So that agents work in the right project even when the daemon's cwd differs

  Scenario: Sessions created without a workspace use the start workspace
    Given a session manager whose default project is "alpha" while the process cwd is elsewhere
    When a session is created without naming a workspace
    Then the session's project is "alpha"

  Scenario: Creating a session in a named workspace
    Given a session manager whose default project is "alpha" and another project "beta"
    When a session is created in workspace "beta" with the role "reviewer"
    Then the session's project is beta's canonical path
    And the session has the role "reviewer"

  Scenario: Without a default project there is no project to fall back to
    Given a session manager with no default project
    When its default project is read
    Then it is an error, not the process cwd
