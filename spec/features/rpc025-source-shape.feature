@done
@RPC-025
@rust
@tui
@persistence
@source-shape
Feature: RPC-025 source-shape regressions for the history-store lift and per-session history state
  """
  RPC-025 (source-shape slice) — pin the file layout invariants for
  the history-store lift and the AgentViewStore per-session history
  state additions:
  - rust/core/src/persistence/mod.rs and history.rs exist and are
  under their respective ceilings (mod.rs < 100 LoC, history.rs <
  300 LoC).
  - rust/fspec-tui/src/store/agent_view/history_state.rs exists
  and is under 100 LoC; HistoryNavState lives there so
  agent_view.rs stays under 300 LoC after the new fields are added.
  - rust/napi/src/persistence/history.rs and the helpers in
  rust/napi/src/persistence/mod.rs (add_history_entry,
  get_history, search_history) become one-line delegates to the
  lifted core helpers (no business logic remains in napi).
  - rust/rpc-types/src/lib.rs declares HistoryMatch with the
  expected three fields (session_id, text, timestamp_iso) and is
  gated on the existing napi feature alongside other shared types.
  - No file under rust/fspec-tui/src/views/ imports forbidden crates
  (codelet_core, codelet_napi, tarpc, tokio_tungstenite).

  Tests: rust/fspec-tui/tests/source_shape_rpc025.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the RPC-002 source-shape invariants to keep holding after the history lift
    So that core owns the persistence module, napi stays a thin binding shim, and agent_view.rs does not bloat past 300 LoC

  Scenario: The lifted history module lives under codelet_core with the right file shape
    Given the rust/core crate
    Then a file exists at rust/core/src/persistence/mod.rs
    And that file is under 100 lines
    And a file exists at rust/core/src/persistence/history.rs
    And rust/core/src/persistence/history.rs is under 300 lines

  Scenario: codelet_core::persistence re-exports the public history surface
    Given the rust/core crate
    Then rust/core/src/persistence/mod.rs declares "pub mod history"
    And rust/core/src/persistence/mod.rs re-exports "HistoryStore"
    And rust/core/src/persistence/mod.rs re-exports "HistoryEntry"
    And rust/core/src/persistence/history.rs declares "pub fn add"
    And rust/core/src/persistence/history.rs declares "pub fn get"
    And rust/core/src/persistence/history.rs declares "pub fn search"

  Scenario: HistoryNavState lives in its own sub-module under the 100-LoC ceiling
    Given the rust/fspec-tui crate
    Then a file exists at rust/fspec-tui/src/store/agent_view/history_state.rs
    And that file is under 100 lines
    And the file declares "pub struct HistoryNavState"
    And the file declares the "recall_index" field
    And the file declares the "cached_draft" field

  Scenario: AgentViewStore stays under 300 LoC after the per-session history fields are added
    Given rust/fspec-tui/src/store/agent_view.rs after RPC-025 lands
    Then the file is under 300 lines
    And the file declares a "history_state_by_session" field
    And the file declares a "cached_history_snapshot" field
    And the file declares "pub fn history_state_for"
    And the file declares "pub fn reset_history_state"
    And the file declares "pub fn set_history_snapshot"

  Scenario: The NAPI persistence surface becomes a thin delegate layer
    Given rust/napi/src/persistence/mod.rs after the lift
    Then rust/napi/src/persistence/history.rs does NOT exist (persistence types live in codelet_core)
    And rust/napi/src/persistence/mod.rs flat re-exports codelet_core::persistence
    And rust/napi/src/persistence/napi_bindings.rs::persistence_add_history delegates to history::add
    And rust/napi/src/persistence/napi_bindings.rs::persistence_get_history delegates to history::get
    And rust/napi/src/persistence/napi_bindings.rs::persistence_search_history delegates to history::search
    And the existing #[napi] persistence_add_history / persistence_get_history / persistence_search_history exports keep their JS-facing signatures byte-identical

  Scenario: HistoryMatch is declared in rust/rpc-types with the expected fields
    Given the rust/rpc-types crate
    Then rust/rpc-types/src/lib.rs declares "pub struct HistoryMatch"
    And HistoryMatch declares the "session_id" field of type SessionId
    And HistoryMatch declares the "text" field of type String
    And HistoryMatch declares the "timestamp_iso" field of type String
    And HistoryMatch is gated on the existing "napi" feature alongside other shared types

  Scenario: No view file imports forbidden crates
    Given the rust/fspec-tui crate
    Then no file under rust/fspec-tui/src/views/ imports codelet_core
    And no file under rust/fspec-tui/src/views/ imports codelet_napi
    And no file under rust/fspec-tui/src/views/ imports tarpc
    And no file under rust/fspec-tui/src/views/ imports tokio_tungstenite
