@RPC-442
@rpc
@multi-workspace
Feature: Per-workspace daemon state
  """
  rust/rpc/src/workspaces.rs: SharedFspecService::workspace(Option<&Path>) resolves a request's
  workspace. None (or the start workspace's canonical path) returns the start workspace backed by
  the service's existing watcher, git-state watcher and cwd; any other path is opened once through
  codelet_common::workspace_registry and cached by canonical path with its own WorkUnitsWatcher and
  GitStateWatcher. daemon::run registers the start workspace in the registry at startup. RPC-443 threads the
  optional workspace through the RPC surface; RPC-444 moves session paths off the process cwd.
  """

  Background: User Story
    As a client of one daemon serving many workspaces
    I want each workspace's board state kept separately
    So that requests for one project never read another project's work units

  Scenario: A request that names no workspace uses the start workspace
    Given a service started in workspace "alpha" with one work unit
    When a request resolves its workspace without naming one
    Then it is served from "alpha" with alpha's work units and cwd

  Scenario: Naming another workspace opens it once with its own work units
    Given a service started in workspace "alpha" and another project "beta" with a different work unit
    When requests name "beta" twice through different spellings of its path
    Then both are served by one opened "beta" state with beta's work units and cwd
    And the workspace registry remembers "beta"
