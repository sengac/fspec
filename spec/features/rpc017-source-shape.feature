@done
@work-units
@tui
@rpc
@RPC-017
@rust
@source-shape
Feature: RPC-017 source-shape regression for the priority-reorder persistence port
  """
  RPC-017 introduces six new source artefacts that downstream cards
  (RPC-018+) rely on. This feature pins the file layout + symbol surface
  so that future refactors cannot silently regress the integration
  shape:

  1. `codelet/common/src/file_lock.rs` — new module exposing
  `pub fn with_file_lock<F, T>(lock_dir, f)` lifted from
  `codelet/napi/src/schedule_handler.rs`.
  2. `codelet/napi/src/schedule_handler.rs` — refactored to call
  `codelet_common::file_lock::with_file_lock` (no more inlined
  `acquire_lock` / `release_lock` / `is_lock_stale`).
  3. `codelet/core/src/work_units_write.rs` — new sibling module
  exposing `pub enum Direction { Up, Down }` +
  `pub fn move_work_unit(cwd, id, direction)`. Kept under the
  300 LoC ceiling.
  4. `codelet/rpc/src/lib.rs` — `FspecService` declares
  `async fn move_work_unit_up(id: String) -> Result<(), String>`
  and `_down`. Errors are serialised as String so they cross
  tarpc cleanly.
  5. `codelet/fspec-tui/src/transport/mod.rs` — `FspecBackend`
  trait declares `async fn move_work_unit_up(&self, id: String)`
  and `_down`.
  6. `codelet/napi/src/work_units_watcher.rs` — additive NAPI
  exports `pub fn move_work_unit_up(cwd, id)` and `_down`.

  Existing TS code paths (`src/commands/prioritize-work-unit.ts` +
  `src/tui/components/BoardView.tsx`) are NOT touched.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the RPC-017 source layout to be locked in by a regression test
    So that future cards inheriting the FspecBackend / SharedFspecService surface continue to find the move_work_unit helper where they expect it

  Scenario: codelet_common::file_lock is lifted out of schedule_handler.rs
    Given the codelet workspace after RPC-017 lands
    Then the file codelet/common/src/file_lock.rs exists
    And codelet/common/src/file_lock.rs contains the substring "pub fn with_file_lock"
    And codelet/common/src/lib.rs contains the substring "pub mod file_lock"
    And codelet/napi/src/schedule_handler.rs contains the substring "codelet_common::file_lock"
    And codelet/napi/src/schedule_handler.rs does NOT contain the substring "fn acquire_lock"
    And codelet/napi/src/schedule_handler.rs does NOT contain the substring "fn release_lock"

  Scenario: codelet_core::work_units_write module exists and stays under 300 LoC
    Given the codelet workspace after RPC-017 lands
    Then the file codelet/core/src/work_units_write.rs exists
    And codelet/core/src/work_units_write.rs has fewer than 300 lines
    And codelet/core/src/work_units_write.rs contains the substring "pub fn move_work_unit"
    And codelet/core/src/work_units_write.rs contains the substring "pub enum Direction"
    And codelet/core/src/lib.rs contains the substring "pub mod work_units_write"
    And codelet/core/src/work_units.rs (the read-side module) still exports `pub fn read_snapshot` and `pub struct WorkUnitsWatcher`

  Scenario: FspecService trait gains the move_work_unit_up / _down RPC methods
    Given codelet/rpc/src/lib.rs after RPC-017 lands
    Then the file contains the substring "async fn move_work_unit_up(id: String)"
    And the file contains the substring "async fn move_work_unit_down(id: String)"
    And the FspecServiceImpl body contains the substring "codelet_core::work_units_write::move_work_unit"

  Scenario: FspecBackend trait gains the move_work_unit_up / _down methods
    Given codelet/fspec-tui/src/transport/mod.rs after RPC-017 lands
    Then the file contains the substring "async fn move_work_unit_up"
    And the file contains the substring "async fn move_work_unit_down"
    And both methods take `id: String` and return `Result<()>`

  Scenario: Both transports implement the new FspecBackend methods
    Given the codelet/fspec-tui crate after RPC-017 lands
    Then codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn move_work_unit_up"
    And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn move_work_unit_down"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn move_work_unit_up"
    And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn move_work_unit_down"

  Scenario: Action::ReorderUp / ReorderDown handlers are no longer no-ops
    Given codelet/fspec-tui/src/app/dispatch.rs after RPC-017 lands
    Then the file contains the substring "backend.move_work_unit_up"
    And the file contains the substring "backend.move_work_unit_down"
    And the file does NOT contain the substring "RPC-012 architecture note [1]: persistence is out of scope"

  Scenario: NAPI exports for move_work_unit_up / _down delegate to the shared helper
    Given codelet/napi/src/work_units_watcher.rs after RPC-017 lands
    Then the file contains the substring "pub fn move_work_unit_up"
    And the file contains the substring "pub fn move_work_unit_down"
    And both function bodies contain the substring "codelet_core::work_units_write::move_work_unit"

  Scenario: Existing TS prioritize-work-unit code path is untouched
    Given the project root after RPC-017 lands
    Then the file src/commands/prioritize-work-unit.ts exists
    And src/commands/prioritize-work-unit.ts still exports prioritizeWorkUnit and routes writes through fileManager.transaction
    And src/tui/components/BoardView.tsx still exists at its pre-RPC-017 path
    And src/tui/components/UnifiedBoardLayout.tsx still exists at its pre-RPC-017 path

  Scenario: Views do not directly import codelet_core / napi / tarpc
    Given the directory codelet/fspec-tui/src/views/ after RPC-017 lands
    When a test scans every *.rs file
    Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
    And no file constructs `tokio::runtime::Builder` or `Runtime::new()`
