@done
@RPC-450
@multi-workspace
@rpc
@board
Feature: Workspace-aware work unit details and reordering
  """
  get_work_unit_details, move_work_unit_up and move_work_unit_down read and wrote only the daemon's start workspace (inner.cwd()), so a board showing another workspace (RPC-445 dropdown, Paseo) could not open or reorder its cards. Each now takes a required workspace path as its first argument, resolved with SharedFspecService::workspace(Some(path)) exactly like list_workspace_work_units (an unregistered path is opened through the workspace registry); the start workspace is passed by its path like any other and an unknown or unopenable path is an error. There is no implicit start-workspace default. The fspec TUI passes its selected board workspace.
  """

  Background: User Story
    As a client showing any workspace's board
    I want card details and reordering to act on the workspace I am showing
    So that every workspace board behaves like the start workspace's

  Scenario: Details and reordering act on the named workspace
    Given a daemon started in workspace "alpha" and a second workspace "beta" with cards "BETA-1" and "BETA-2" in backlog
    When a client reads the details of "BETA-1" in "beta" and moves "BETA-2" up in "beta" and then down in "beta"
    Then the details are beta's "BETA-1" record
    And beta's backlog order changes and alpha's work-units.json is unchanged

  Scenario: The start workspace is addressed by its path
    Given a daemon started in workspace "alpha" with cards "ALPHA-1" and "ALPHA-2" in backlog
    When a client moves "ALPHA-2" up in "alpha"
    Then alpha's backlog order is "ALPHA-2", "ALPHA-1"

  Scenario: An unknown workspace is refused
    Given a daemon started in workspace "alpha"
    When a client reads card details or moves a card in a workspace path that does not exist
    Then each call returns an error naming that path

  Scenario: The TUI acts on its selected board workspace
    Given the TUI board has workspace "beta" selected
    When I open the details of a card and move a card up and down
    Then each request names workspace "beta"
