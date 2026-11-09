@done
@RPC-457
@rpc
@multi-workspace
@rust
Feature: Add and remove workspaces through RPC and the CLI
  """
  The workspace registry ({data_dir}/workspaces.json) only grew implicitly
  (daemon start, any workspace-aware RPC) and never shrank. Now
  workspace_registry::add records a directory that contains spec/, and
  workspace_registry::remove forgets one. The daemon exposes them as the
  appended RPCs add_workspace and remove_workspace (RPC-449); remove refuses
  the start workspace and drops the cached workspace scope with its
  watchers. fspec workspaces add|list|remove edits the same registry.
  Tests: rust/fspec/tests/rpc457_add_and_remove_workspaces.rs.
  """

  Background: User Story
    As a user who keeps several fspec projects
    I want to add and remove workspaces explicitly through the daemon or the CLI
    So that the workspace list holds exactly the projects I want, without stale entries

  Scenario: Adding a directory without spec is refused
    Given a directory that has no spec folder
    When it is added as a workspace
    Then it is refused as not an fspec project
    And the registry does not list it

  Scenario: The daemon adds and removes a workspace
    Given a daemon started in workspace "alpha" and a project "beta"
    When "beta" is added over RPC and its work units are read
    Then the workspace list includes "beta"
    When "beta" is removed over RPC
    Then the workspace list no longer includes "beta"
    And the daemon no longer holds "beta" open

  Scenario: The daemon refuses to remove its start workspace or an unknown one
    Given a daemon started in workspace "alpha"
    When "alpha" or an unregistered directory is removed over RPC
    Then each removal is refused

  Scenario: The CLI adds, lists and removes a workspace
    Given a project "beta"
    When "fspec workspaces add" names it
    Then "fspec workspaces list" shows it
    When "fspec workspaces remove" names it
    Then "fspec workspaces list" no longer shows it
