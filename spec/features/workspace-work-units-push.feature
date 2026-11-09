@BUG-582
@RPC-445
@rpc
@multi-workspace
Feature: Workspace-keyed work-unit pushes
  """
  codelet-rpc-types gains WorkspaceWorkUnits { workspace, units }. SharedFspecService keeps a
  broadcast::Sender<WorkspaceWorkUnits> (workspace_work_units_rx()); every WorkspaceState opened
  through SharedFspecService::workspace (RPC-442) spawns a forwarder from its WorkUnitsWatcher
  into that sender, tagging each snapshot with the workspace's canonical path. codelet-rpc-server
  appends Envelope::WorkspaceWorkUnitsUpdate(WorkspaceWorkUnits) (bincode indices stay stable),
  fans it out to every connection from a per-connection task (workspace_fanout.rs), and the
  client exposes FspecWsClient::workspace_work_units_rx(). The start workspace keeps using
  Envelope::WorkUnitsUpdate.
  """

  Background: User Story
    As an fspec client showing a workspace other than the daemon's start workspace
    I want that workspace's work-unit changes pushed to me
    So that the board stays live without polling

  Scenario: An opened workspace's changes reach the service stream tagged with its path
    Given a daemon service started in "alpha" that has opened "beta"
    When beta's work units change on disk
    Then the service's workspace stream delivers beta's new work units tagged with beta's canonical path

  Scenario: An opened workspace's changes are pushed to WebSocket clients
    Given a WebSocket client connected to a daemon started in "alpha"
    And the client has read the work units of "beta"
    When beta's work units change on disk
    Then the client receives beta's new work units tagged with beta's canonical path
