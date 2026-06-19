@done
@RPC-025
@rust
@tui
@persistence
@source-shape
@bootstrap
Feature: RPC-025 fspec binary persistence bootstrap initialises the global data directory
  """
  RPC-025 (binary bootstrap slice) — the `fspec` binary MUST call
  `codelet_common::set_data_directory(~/.fspec)` at startup BEFORE
  any persistence_*_history RPC method can be invoked.

  Without this call, `HistoryStore::new()` returns
  `Err("Data directory not initialized")`, and the swallowing
  `if let Ok(snapshot) = backend.persistence_get_history(...)` arm
  in `dispatch_history_recall.rs` silently drops the error — making
  Shift+↑/↓ appear to do nothing in the live binary even though
  the unit/integration tests pass (because the tests call
  `codelet_common::set_data_directory(temp.path())` themselves).

  `common::build_service` is the single chokepoint for both
  `combined::run` (TUI + WS server in one process) and
  `daemon::run` (headless WS server). `client::run` does NOT call
  `build_service` because it constructs a `WebSocketFspecBackend`
  pointed at a remote daemon — client mode inherits the daemon's
  data directory through tarpc round-trips, so a single call to
  `codelet_common::set_data_directory(home_fspec_dir()?)` inside
  `build_service` covers both modes that actually own a
  `FspecServiceImpl`.

  Tests: codelet/fspec/src/common.rs::tests.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the fspec binary to initialise the global data directory at startup
    So that Shift+↑/↓ history recall actually walks ~/.fspec/history.jsonl in the live binary instead of silently doing nothing

  Scenario: build_service initialises the global data directory before exposing persistence RPCs
    Given the codelet/fspec binary crate after RPC-025 lands
    When common::build_service is invoked against a tempdir workspace
    Then codelet_common::get_data_dir() returns Ok with a path ending in ".fspec"
    And codelet/fspec/src/common.rs contains the substring "codelet_common::set_data_directory"
    And the set_data_directory call appears BEFORE the WorkUnitsWatcher::new(workspace) call in build_service
