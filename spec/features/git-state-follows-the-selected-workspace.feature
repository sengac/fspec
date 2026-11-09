@done
@RPC-454
@rpc
@multi-workspace
@rust
Feature: Git state, branch and footer directory follow the selected workspace
  """
  The TUI subscribed only to the start workspace's git_state_changed
  broadcast and the footer directory and branch came from
  get_workspace_info() for the start workspace; the per-workspace
  GitStateWatcher existed but nothing published it.
  Now each workspace opened besides the start workspace forwards its git
  state onto a workspace-keyed broadcast (SharedFspecService::
  workspace_git_state_rx), pushed over WebSocket as the appended
  Envelope::WorkspaceGitStateUpdate (FspecWsClient::workspace_git_state_rx).
  New RPCs get_workspace_info_in_workspace and git_state_in_workspace
  (appended per RPC-449) read the named workspace. The TUI applies a
  workspace's git state only while it is selected (the start workspace's
  frames only while the start workspace is selected) and, on choosing a
  workspace, loads its directory, branch and git state.
  Tests: rust/fspec-tui/tests/rpc454_git_state_follows_the_selected_workspace.rs.
  """

  Background: User Story
    As a user switching the board between workspaces
    I want to see the selected workspace's git state, branch and directory
    So that the footer and git-driven views never describe another repository

  Scenario: Workspace info over RPC comes from the named workspace
    Given a daemon started in git repository "alpha" and a second git repository "beta" on branch "feature-beta"
    When the workspace info of "beta" is requested over RPC
    Then it names beta's directory and the branch "feature-beta"

  Scenario: An opened workspace's git state is published
    Given a daemon started in git repository "alpha" with workspace "beta" opened
    When a file in "beta" changes
    Then a git-state push tagged with workspace "beta" arrives

  Scenario: Choosing a workspace loads its directory, branch and git state
    Given the board lists workspaces "alpha" and "beta"
    When workspace "beta" is chosen
    Then the backend is asked for the workspace info and git state of "beta"
