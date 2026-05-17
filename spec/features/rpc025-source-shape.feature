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
    - codelet/core/src/persistence/mod.rs and history.rs exist and are
      under their respective ceilings (mod.rs < 100 LoC, history.rs <
      300 LoC).
    - codelet/fspec-tui/src/store/agent_view/history_state.rs exists
      and is under 100 LoC; HistoryNavState lives there so
      agent_view.rs stays under 300 LoC after the new fields are added.
    - codelet/napi/src/persistence/history.rs and the helpers in
      codelet/napi/src/persistence/mod.rs (add_history_entry,
      get_history, search_history) become one-line delegates to the
      lifted core helpers (no business logic remains in napi).
    - codelet/rpc-types/src/lib.rs declares HistoryMatch with the
      expected three fields (session_id, text, timestamp_iso) and is
      gated on the existing napi feature alongside other shared types.
    - No file under codelet/fspec-tui/src/views/ imports forbidden crates
      (codelet_core, codelet_napi, tarpc, tokio_tungstenite).

  Tests: codelet/fspec-tui/tests/source_shape_rpc025.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the RPC-002 source-shape invariants to keep holding after the history lift
    So that core owns the persistence module, napi stays a thin binding shim, and agent_view.rs does not bloat past 300 LoC

  Scenario: The lifted history module lives under codelet_core with the right file shape
    Given the codelet/core crate
    Then a file exists at codelet/core/src/persistence/mod.rs
    And that file is under 100 lines
    And a file exists at codelet/core/src/persistence/history.rs
    And codelet/core/src/persistence/history.rs is under 300 lines

  Scenario: codelet_core::persistence re-exports the public history surface
    Given the codelet/core crate
    Then codelet/core/src/persistence/mod.rs declares "pub mod history"
    And codelet/core/src/persistence/mod.rs re-exports "HistoryStore"
    And codelet/core/src/persistence/mod.rs re-exports "HistoryEntry"
    And codelet/core/src/persistence/history.rs declares "pub fn add"
    And codelet/core/src/persistence/history.rs declares "pub fn get"
    And codelet/core/src/persistence/history.rs declares "pub fn search"

  Scenario: HistoryNavState lives in its own sub-module under the 100-LoC ceiling
    Given the codelet/fspec-tui crate
    Then a file exists at codelet/fspec-tui/src/store/agent_view/history_state.rs
    And that file is under 100 lines
    And the file declares "pub struct HistoryNavState"
    And the file declares the "recall_index" field
    And the file declares the "cached_draft" field

  Scenario: AgentViewStore stays under 300 LoC after the per-session history fields are added
    Given codelet/fspec-tui/src/store/agent_view.rs after RPC-025 lands
    Then the file is under 300 lines
    And the file declares a "history_state_by_session" field
    And the file declares a "cached_history_snapshot" field
    And the file declares "pub fn history_state_for"
    And the file declares "pub fn reset_history_state"
    And the file declares "pub fn set_history_snapshot"

  Scenario: The NAPI persistence surface becomes a thin delegate layer
    Given codelet/napi/src/persistence/history.rs after the lift
    Then the file does NOT declare its own struct HistoryStore (re-exports from codelet_core instead)
    And codelet/napi/src/persistence/mod.rs::add_history_entry is a one-line delegate to codelet_core::persistence::history::add
    And codelet/napi/src/persistence/mod.rs::get_history is a one-line delegate to codelet_core::persistence::history::get
    And codelet/napi/src/persistence/mod.rs::search_history is a one-line delegate to codelet_core::persistence::history::search
    And the existing #[napi] persistence_add_history / persistence_get_history / persistence_search_history exports keep their JS-facing signatures byte-identical

  Scenario: HistoryMatch is declared in codelet/rpc-types with the expected fields
    Given the codelet/rpc-types crate
    Then codelet/rpc-types/src/lib.rs declares "pub struct HistoryMatch"
    And HistoryMatch declares the "session_id" field of type SessionId
    And HistoryMatch declares the "text" field of type String
    And HistoryMatch declares the "timestamp_iso" field of type String
    And HistoryMatch is gated on the existing "napi" feature alongside other shared types

  Scenario: No view file imports forbidden crates
    Given the codelet/fspec-tui crate
    Then no file under codelet/fspec-tui/src/views/ imports codelet_core
    And no file under codelet/fspec-tui/src/views/ imports codelet_napi
    And no file under codelet/fspec-tui/src/views/ imports tarpc
    And no file under codelet/fspec-tui/src/views/ imports tokio_tungstenite
