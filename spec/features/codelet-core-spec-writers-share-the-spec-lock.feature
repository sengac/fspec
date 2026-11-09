@done
@infrastructure
@file-ops
@concurrency
@scheduler
@rust
@bug-fix
@critical
@BUG-477
Feature: Codelet core spec writers share the spec lock
  """
  Architecture notes:
  - codelet-core work_units_write::move_work_unit (the TUI work-unit reorder) and the scheduler (crud, trigger::update_last_run, catch_up) plus codelet-agent-loop schedule_handler write spec/work-units.json and spec/schedules.json outside fspec-core.
  - They switch from the mkdir '<file>.lock' protocol and plain tokio::fs::write to codelet_common::spec_lock::SpecLock on the spec directory plus replace_file_atomically, so they serialise with fspec commands on spec/.fspec-write.lock.
  """

  Background: User Story
    As a developer or agent running several fspec commands at the same time
    I want to have the TUI reorder and the scheduler take the same spec lock as fspec commands
    So that a board or schedule write from the app never interleaves with a command's read-modify-write

  Scenario: The TUI reorder and the scheduler use the same spec lock
    Given another process holds the spec lock and does not release it
    When the TUI moves a work unit up and the scheduler records a run
    Then both fail with "Failed to acquire file lock"
    And spec/work-units.json and spec/schedules.json are byte-for-byte unchanged
