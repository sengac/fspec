@RPC-442
@rpc
@multi-workspace
Feature: Daemon registers its start workspace
  """
  rust/fspec/src/daemon.rs: after resolving the start workspace, daemon::run opens it through
  codelet_common::workspace_registry so {data_dir}/workspaces.json lists it for every client.
  This is daemon-only startup (build_service stays side-effect free for its unit tests).
  """

  Background: User Story
    As a client choosing a workspace on a daemon
    I want the daemon's own start workspace in the shared list
    So that the workspace dropdown always includes it

  Scenario: The daemon records its start workspace in the registry
    Given a project directory used as the daemon's start workspace
    When the daemon service is built for it
    Then the workspace registry lists that project
