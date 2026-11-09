@RPC-443
@rpc
@multi-workspace
@BUG-290
Feature: Workspace-aware RPC methods
  """
  rust/rpc: list_workspaces() -> Result<Vec<WorkspaceEntry>, String> and
  list_workspace_work_units(workspace: String) -> Result<Vec<WorkUnitInfo>, String> are new
  FspecService methods (declaration in service_declaration/workspace.rs, implementation in
  service_implementation/workspaces.rs, MCP mapping in mcp_dispatch/group_41.rs). WorkspaceEntry
  { path, name, start } lives in codelet-rpc-types. They resolve through
  SharedFspecService::workspace (RPC-442) and codelet_common::workspace_registry (RPC-441).
  """

  Background: User Story
    As an fspec client such as the TUI or paseo
    I want to list a daemon's workspaces and read any workspace's board
    So that I can offer a workspace switcher without restarting the daemon

  Scenario: Listing workspaces puts the start workspace first
    Given a daemon service started in "alpha" that has also opened "beta"
    When a client lists workspaces
    Then the list is alpha marked as the start workspace followed by beta

  Scenario: Reading a named workspace's work units
    Given a daemon service started in "alpha" and a project "beta" with work unit "BETA-001"
    When a client lists the work units of "beta"
    Then the result contains only "BETA-001"
    And listing work units without a workspace still returns alpha's work units

  Scenario: Naming a path that is not a directory is rejected
    Given a daemon service started in "alpha"
    When a client lists the work units of a path that does not exist
    Then the call returns an error naming that path
