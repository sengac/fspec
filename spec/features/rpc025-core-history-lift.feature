@done
@RPC-025
@rust
@persistence
@command-history
Feature: RPC-025 HistoryStore lifted from codelet_napi into codelet_core
  """
  RPC-025 (core history lift slice) — Move the HistoryStore +
  HistoryEntry surface from rust/napi/src/persistence/{history.rs,
  types.rs} into rust/core/src/persistence/{history.rs, mod.rs} so
  codelet_rpc can delegate to it without re-introducing a rpc → napi
  dep. Make the existing NAPI helpers (add_history_entry, get_history,
  search_history) and the #[napi] exports (persistence_add_history,
  persistence_get_history, persistence_search_history) one-line
  delegates to the lifted core helpers.

  The on-disk JSONL file at codelet_common::get_data_dir().join(
  "history.jsonl") is the SINGLE source of truth — both the TS Ink TUI
  (via the existing NAPI exports) and the Rust ratatui TUI (via the
  new RPC methods landing in this card) read from and write to this
  same file.

  Tests: rust/fspec-tui/tests/core_history_lift_rpc025.rs (uses a
  temp HOME via std::env::set_var to redirect get_data_dir without
  touching the user's real history).
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the HistoryStore and HistoryEntry types to live in codelet_core::persistence
    So that codelet_rpc can call into them without depending on codelet_napi and both the TS Ink TUI and the Rust ratatui TUI converge on the same JSONL store

  Scenario: codelet_core::persistence::history::add writes to the same on-disk JSONL file the NAPI surface uses
    Given a temporary data directory configured via codelet_common::set_data_dir
    When codelet_core::persistence::history::add is called with a HistoryEntry whose display is "hello core"
    Then a single JSONL line is appended to <data_dir>/history.jsonl whose "display" field equals "hello core"
    And codelet_core::persistence::history::get(None, None) returns a Vec containing that entry as its only element

  Scenario: codelet_core::persistence::history::get returns entries newest-first, optionally filtered by project
    Given a temporary data directory with no existing history file
    And HistoryEntries are added in order entry-a (project /p1), entry-b (project /p1), entry-c (project /p2)
    When codelet_core::persistence::history::get(None, None) is called
    Then the returned Vec is ordered ["entry-c", "entry-b", "entry-a"] (newest-first)
    When codelet_core::persistence::history::get(Some("/p1"), None) is called
    Then the returned Vec is ordered ["entry-b", "entry-a"]
    When codelet_core::persistence::history::get(Some("/p1"), Some(1)) is called
    Then the returned Vec is ordered ["entry-b"]

  Scenario: codelet_core::persistence::history::search is case-insensitive substring on display and respects project filter
    Given a temporary data directory with no existing history file
    And HistoryEntries with display ["foobar" (/p1), "baz" (/p1), "FOOZ" (/p2)] are added
    When codelet_core::persistence::history::search("foo", None) is called
    Then the returned displays are exactly ["FOOZ", "foobar"] in newest-first order
    When codelet_core::persistence::history::search("foo", Some("/p1")) is called
    Then the returned displays are exactly ["foobar"]
    When codelet_core::persistence::history::search("missing", None) is called
    Then the returned Vec is empty

  Scenario: rust/napi/src/persistence/mod.rs::add_history_entry is now a one-line delegate to codelet_core::persistence::history::add
    Given a temporary data directory configured for the test process
    When codelet_napi::persistence::add_history_entry is called with a HistoryEntry whose display is "hello napi"
    Then codelet_core::persistence::history::get(None, Some(1)) returns a Vec whose first display is "hello napi"
    And the on-disk <data_dir>/history.jsonl contains exactly one line whose display is "hello napi"

  Scenario: rust/napi/src/persistence/mod.rs::get_history is now a one-line delegate to codelet_core::persistence::history::get
    Given a temporary data directory containing two HistoryEntries written via codelet_core::persistence::history::add
    When codelet_napi::persistence::get_history(None, Some(2)) is called
    Then the returned Vec<HistoryEntry> is identical (same displays in the same order) to codelet_core::persistence::history::get(None, Some(2))

  Scenario: rust/napi/src/persistence/mod.rs::search_history is now a one-line delegate to codelet_core::persistence::history::search
    Given a temporary data directory containing HistoryEntries via codelet_core::persistence::history::add
    When codelet_napi::persistence::search_history("query", None) is called
    Then the returned Vec<HistoryEntry> is identical to codelet_core::persistence::history::search("query", None)

  Scenario: HistoryEntry::to_history_match converts a core HistoryEntry into the transport-portable HistoryMatch
    Given a HistoryEntry with display "submitted text", session_id Uuid("...-1"), and a known timestamp
    When HistoryEntry::to_history_match() is called
    Then the returned HistoryMatch.text equals "submitted text"
    And HistoryMatch.session_id equals SessionId from the entry's session_id Uuid
    And HistoryMatch.timestamp_iso equals the entry.timestamp formatted via to_rfc3339()

  Scenario: The TS Ink TUI persistence path stays byte-identical via the kept NAPI exports
    Given the existing #[napi] persistence_add_history / persistence_get_history / persistence_search_history exports
    Then their JS-facing parameter lists are unchanged
    And their return types (NapiHistoryEntry) are unchanged
    And invoking persistence_add_history("hi", "/cwd", "uuid") with the lifted core under it produces the same observable effect (one JSONL line appended) as before the lift
