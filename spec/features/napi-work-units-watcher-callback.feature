@done
@integration-test
@p1
@critical
@napi
@rust
@RPC-006
Feature: NAPI work-units watcher callback compatibility after the lift
  """
  Architecture

  RPC-006 moves the cross-platform `notify`-based watcher logic out of
  `codelet/napi/src/work_units_watcher.rs` and into a new pure-Rust
  module at `codelet/core/src/work_units.rs`. The existing NAPI export
  surface MUST be preserved bit-for-bit so that the Ink/TS frontend
  keeps working unchanged:

    startWorkUnitsWatcher(projectRoot: string, callback: (chunk: StreamChunk) => void): void
    stopWorkUnitsWatcher(): void

  Internally, the NAPI shim wraps a `codelet_core::work_units::WorkUnitsWatcher`,
  drains its `subscribe()` broadcast receiver, and forwards each
  `Vec<WorkUnitInfo>` payload as a new
  `StreamChunk::work_units_update(...)` chunk into the existing
  `ThreadsafeFunction` callback.

  This Vitest smoke codifies the invariant: any future regression that
  silently breaks the Node-side callback fires loudly in CI.

  References: spec/attachments/RPC-006/plan.md (Step 2);
              RPC-005 architecture rule [9].
  """

  Background: User Story
    As a TypeScript developer maintaining the existing Ink frontend
    I want the existing NAPI startWorkUnitsWatcher callback to keep firing on file mutation with the same WorkUnitInfo[] payload shape
    So that the watcher lift is invisible from the JS side and no TS code needs to change

  Scenario: NAPI startWorkUnitsWatcher callback continues to fire after the lift
    Given the existing NAPI export startWorkUnitsWatcher implemented as a thin shim over codelet_core::work_units::WorkUnitsWatcher and a temporary workspace observed by that shim
    When the Vitest smoke test mutates spec/work-units.json once and waits up to two seconds on the registered ThreadsafeFunction callback
    Then the callback is invoked at least once with a WorkUnitInfo[] payload whose shape (id, title, workType, status, description, estimate, epic) is unchanged from RPC-005
