@done
@RPC-025
@rust
@rpc
@persistence
@parity
Feature: RPC-025 FspecService persistence_*_history RPCs over both EmbeddedFspecBackend and WebSocketFspecBackend
  """
  RPC-025 (RPC methods slice) — Add three new RPC methods to FspecService
  in codelet/rpc/src/lib.rs and to the FspecBackend trait in
  codelet/fspec-tui/src/transport/mod.rs:
  - persistence_add_history(session: SessionId, text: String) -> Result<()>
  - persistence_get_history(session: SessionId, limit: u32) -> Result<Vec<String>>
  - persistence_search_history(query: String) -> Result<Vec<HistoryMatch>>

  EmbeddedFspecBackend forwards via its in-process FspecServiceImpl;
  WebSocketFspecBackend forwards via tarpc over WS. Both must produce
  identical observable behaviour against the same on-disk JSONL store
  (cross-transport-parity invariant from RPC-009/011/012).

  HistoryMatch lands in codelet/rpc-types/src/lib.rs as a new shared
  type gated on the napi feature, with three fields: session_id,
  text, timestamp_iso (RFC3339 string).

  Tests: codelet/fspec-tui/tests/rpc_persistence_history_rpc025.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want both transports to expose persistence_add_history / persistence_get_history / persistence_search_history with identical semantics
    So that the AgentView can call the backend without knowing whether it is in-process or remote, and the TS Ink TUI's NAPI persistence path stays unchanged

  Scenario: FspecService trait declares the three new history RPC methods
    Given the codelet/rpc crate
    Then the FspecService trait declares "async fn persistence_add_history(session: SessionId, text: String) -> Result<(), String>"
    And the FspecService trait declares "async fn persistence_get_history(session: SessionId, limit: u32) -> Result<Vec<String>, String>"
    And the FspecService trait declares "async fn persistence_search_history(query: String) -> Result<Vec<HistoryMatch>, String>"

  Scenario: FspecBackend trait declares the three new history methods with matching signatures
    Given the codelet/fspec-tui crate
    Then the FspecBackend trait in transport/mod.rs declares "async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>"
    And the FspecBackend trait declares "async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>>"
    And the FspecBackend trait declares "async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>"

  Scenario: EmbeddedFspecBackend.persistence_add_history persists into the lifted core store via FspecServiceImpl
    Given an EmbeddedFspecBackend bound to a workspace cwd "/tmp/parity"
    When EmbeddedFspecBackend.persistence_add_history(SessionId("s-1"), "embedded hello") is awaited
    Then codelet_core::persistence::history::get(Some("/tmp/parity"), Some(1))[0].display equals "embedded hello"

  Scenario: EmbeddedFspecBackend.persistence_get_history returns the texts of the most recent entries newest-first
    Given an EmbeddedFspecBackend bound to a workspace cwd "/tmp/parity"
    And persistence_add_history is called for SessionId("s-1") with texts "a", "b", "c" in order
    When EmbeddedFspecBackend.persistence_get_history(SessionId("s-1"), 10) is awaited
    Then the returned Vec<String> equals ["c", "b", "a"]

  Scenario: EmbeddedFspecBackend.persistence_search_history returns HistoryMatch values with text, session_id, and ISO timestamp
    Given an EmbeddedFspecBackend bound to a workspace cwd "/tmp/parity"
    And persistence_add_history is called for SessionId("s-1") with texts "foobar", "baz", "FOOZ"
    When EmbeddedFspecBackend.persistence_search_history("foo") is awaited
    Then the returned Vec<HistoryMatch> has length 2
    And each HistoryMatch.session_id equals SessionId("s-1")
    And the HistoryMatch.text values are exactly ["FOOZ", "foobar"] in newest-first order
    And each HistoryMatch.timestamp_iso is a valid RFC3339 string

  Scenario: WebSocketFspecBackend round-trips persistence_add_history over tarpc to the same core store
    Given a WebSocketFspecBackend connected to a local FspecService bound to cwd "/tmp/parity"
    When WebSocketFspecBackend.persistence_add_history(SessionId("s-2"), "ws hello") is awaited
    Then codelet_core::persistence::history::get(Some("/tmp/parity"), Some(1))[0].display equals "ws hello"

  Scenario: Cross-transport-parity for persistence_get_history
    Given an EmbeddedFspecBackend and a WebSocketFspecBackend both bound to the same workspace cwd "/tmp/parity"
    And persistence_add_history is called via the EmbeddedFspecBackend for SessionId("s-1") with text "shared"
    When persistence_get_history(SessionId("s-1"), 10) is awaited on BOTH backends
    Then both return Vec<String> == ["shared"]

  Scenario: Cross-transport-parity for persistence_search_history
    Given an EmbeddedFspecBackend and a WebSocketFspecBackend both bound to the same workspace cwd "/tmp/parity"
    And persistence_add_history is called via the WebSocketFspecBackend for SessionId("s-1") with texts "alpha", "beta"
    When persistence_search_history("alp") is awaited on BOTH backends
    Then both return Vec<HistoryMatch> whose text values equal ["alpha"]
    And the timestamp_iso values are equal between the two backends for the same entry

  Scenario: FspecServiceImpl falls back to a None project filter when the shared service has no workspace cwd attached
    Given a FspecServiceImpl with workspace_cwd == None
    And persistence_add_history is called for SessionId("s-1") with text "global hello"
    When persistence_get_history(SessionId("s-1"), 10) is awaited
    Then the returned Vec<String> contains "global hello" regardless of project
