@done
@RPC-017
@rust
@rpc
@persistence
@work-units
Feature: RPC-017 work-units write helper (codelet_core::work_units_write::move_work_unit)
  """
  RPC-017 (slice 1 of 3) — codelet_core gains a write-side sibling
  module `codelet_core::work_units_write` that exposes:

  pub enum Direction { Up, Down }
  pub fn move_work_unit(cwd: &Path, id: &str, direction: Direction) -> Result<()>

  Semantics mirror `src/commands/prioritize-work-unit.ts`:
  - Reorder within `states[<column>]` only — never across columns.
  - Done column refuses reorders.
  - Boundary moves are no-ops (Up at index 0; Down at index len-1).
  - Persistence uses an atomic temp + rename + proper-lockfile-
  compatible mkdir lock on `spec/work-units.json.lock` so concurrent
  TS `fspec prioritize-work-unit` commands cooperate.

  The mkdir-lock helper is the lifted `with_file_lock` from
  `rust/common/src/file_lock.rs` (extracted from the inlined copy
  in `rust/napi/src/schedule_handler.rs`).
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a pure-Rust `move_work_unit(cwd, id, direction)` helper in codelet_core that mirrors the TS prioritize-work-unit semantics and persists via the same proper-lockfile-compatible inter-process lock as the existing TS CLI
    So that both the Rust ratatui TUI and the TS CLI cooperate on writes to spec/work-units.json without corrupting the file

  Scenario: move_work_unit Up swaps the target with its predecessor inside states[<column>]
    Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    When move_work_unit(cwd, "C-003", Direction::Up) is called
    Then the call returns Ok(())
    And spec/work-units.json on disk now has states.backlog == ["A-001", "C-003", "B-002"]
    And no other column array in the file is modified

  Scenario: move_work_unit Down swaps the target with its successor inside states[<column>]
    Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    When move_work_unit(cwd, "A-001", Direction::Down) is called
    Then the call returns Ok(())
    And spec/work-units.json on disk now has states.backlog == ["B-002", "A-001", "C-003"]

  Scenario: move_work_unit Up at the top boundary is a no-op
    Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    When move_work_unit(cwd, "A-001", Direction::Up) is called
    Then the call returns Ok(())
    And spec/work-units.json on disk still has states.backlog == ["A-001", "B-002", "C-003"]

  Scenario: move_work_unit Down at the bottom boundary is a no-op
    Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    When move_work_unit(cwd, "C-003", Direction::Down) is called
    Then the call returns Ok(())
    And spec/work-units.json on disk still has states.backlog == ["A-001", "B-002", "C-003"]

  Scenario: move_work_unit refuses to reorder a done-column unit
    Given a workspace whose spec/work-units.json has states.done == ["DONE-001", "DONE-002"]
    When move_work_unit(cwd, "DONE-001", Direction::Down) is called
    Then the call returns Err
    And the error message contains the substring "done column"
    And spec/work-units.json on disk is unchanged

  Scenario: move_work_unit returns Err for an unknown work unit id
    Given a workspace whose spec/work-units.json has states.backlog == ["A-001"]
    When move_work_unit(cwd, "MISSING-999", Direction::Up) is called
    Then the call returns Err
    And the error message contains the substring "MISSING-999"

  Scenario: move_work_unit updates meta.lastUpdated on every persisting write
    Given a workspace whose spec/work-units.json has meta.lastUpdated == "2026-01-01T00:00:00.000Z"
    And states.backlog == ["A-001", "B-002"]
    When move_work_unit(cwd, "B-002", Direction::Up) is called
    Then the call returns Ok(())
    And spec/work-units.json on disk has a meta.lastUpdated strictly greater than "2026-01-01T00:00:00.000Z"

  Scenario: Concurrent move_work_unit calls serialize via the inter-process lock
    Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    When two threads call move_work_unit("C-003", Up) and move_work_unit("A-001", Down) concurrently
    Then both calls return Ok(())
    And spec/work-units.json on disk is valid JSON
    And the post-state states.backlog has length 3 and is a permutation of ["A-001", "B-002", "C-003"]
