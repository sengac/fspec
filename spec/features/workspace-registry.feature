@RPC-441
@rpc
@persistence
@multi-workspace
@RPC-442
Feature: Persisted workspace registry
  """
  rust/common/src/workspace_registry.rs: open(path) canonicalizes the path, derives the display
  name from its final component, and records the workspace in {data_dir}/workspaces.json
  ({"version":1,"workspaces":[{"path","name"}]}) under the shared file lock; list() reads that file
  (missing file = empty). The daemon (RPC-442) opens workspaces through this registry and the
  board header dropdown (RPC-445) lists them.
  """

  Background: User Story
    As a client of a daemon that serves many workspaces
    I want opened workspaces identified by their canonical path and remembered
    So that every client sees the same workspace list across daemon restarts

  Scenario: Different spellings of a path open one workspace
    Given a project directory named "proj" reachable through a relative path, a trailing slash and a symlink
    When each spelling is opened through the workspace registry
    Then every open returns the same canonical path and the display name "proj"
    And the registry lists exactly one workspace

  Scenario: Opened workspaces are remembered across restarts
    Given two project directories have been opened through the workspace registry
    When the registry is read again from the same data directory
    Then both workspaces are listed in the order they were first opened

  Scenario: A path that is not a directory is rejected
    Given a path to a regular file and a path that does not exist
    When each is opened through the workspace registry
    Then each open fails with a not-a-directory error
    And nothing is recorded in the registry

  Scenario: Workspaces whose directory was removed are not listed
    Given two opened workspaces
    When one workspace directory is deleted
    Then the registry lists only the remaining workspace
