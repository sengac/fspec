@BUG-582
@done
@BUG-497
@cli
@configuration
@rust
Feature: Project settings and the core board use the resolved project root
  """
  BUG-523 made every fspec command resolve one project root (the directory
  owning the spec/ it reads) for reads and writes. Three paths still used
  <cwd>/spec without walking up: codelet_common::fspec_config::write_config
  and load_config (a project settings save from a subdirectory created
  subdir/spec/fspec-config.json, which later commands run there then used as
  their spec folder), and codelet-core's board snapshot/watcher and
  move_work_unit. resolve_project_root moves to codelet-common
  (rust/common/src/project_root.rs; fspec-core re-exports it) and those paths
  use it.
  Tests: rust/common/tests/bug497_project_settings_use_the_project_root.rs,
  rust/core/tests/bug497_core_board_uses_the_project_root.rs.
  """

  Background: User Story
    As a maintainer running fspec from a subdirectory of the project
    I want to have project settings and the board watcher use the project's own spec folder
    So that a settings save or board move from a subdirectory never creates or reads a stray spec folder

  Scenario: Saving project settings from a subdirectory writes the project's spec folder
    Given a project with a spec folder and a subdirectory without one
    When project settings are saved from the subdirectory
    Then the project's spec/fspec-config.json holds the settings
    And the subdirectory has no spec folder

  Scenario: Loading project settings from a subdirectory reads the project's spec folder
    Given a project whose spec/fspec-config.json sets a value
    When the settings are loaded from a subdirectory
    Then the loaded settings carry that value

  Scenario: Reordering a work unit from a subdirectory updates the project's board
    Given a project whose backlog holds two work units
    When the second work unit is moved up from a subdirectory
    Then the project's spec/work-units.json lists it first in backlog
    And the subdirectory has no spec folder

  Scenario: The board snapshot from a subdirectory reads the project's board
    Given a project whose board has a work unit
    When the board snapshot is read from a subdirectory
    Then the snapshot contains that work unit
