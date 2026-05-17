@done
@session-management
@parity
@RPC-026
@rust
@tui
@rpc
Feature: RPC-026 Cross-transport parity — list_sessions and persistence_search_history behave identically via EmbeddedFspecBackend and WebSocketFspecBackend

  """
  RPC-026 (cross-transport parity slice) — preserves the RPC-002
  invariant that every new behaviour is exercised against BOTH
  EmbeddedFspecBackend AND WebSocketFspecBackend with byte-identical
  observable outcomes (matches the RPC-020 search_files parity test
  and the RPC-025 persistence_*_history parity test pattern).

  RPC-026 introduces NO new FspecBackend methods — only consumes
  `list_sessions` (already in RPC-007 / RPC-008) and
  `persistence_search_history` (already lifted in RPC-025) at the
  popup / dispatch layer. This parity feature therefore proves the
  two backends RPC-026 depends on still produce byte-identical
  observable outcomes for the SAME SharedFspecService, so the resume
  picker / search palette UX is transport-agnostic.

  End-to-end App-level behaviour (open /resume, pick a session,
  AttachToSession dispatch, popup state) is exercised against a
  programmable mock backend in `app_dispatch_resume_search_rpc026.rs`,
  not duplicated here.

  Tests: codelet/fspec-tui/tests/rpc026_cross_transport_parity.rs.
  Uses the same DATA_DIRECTORY serialization pattern from
  rpc_persistence_history_rpc025.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the /resume and /search popups to consume backend RPC methods that produce identical outcomes against the embedded and the WebSocket backends
    So that the daemon-attached fspec TUI's session picker and history typeahead feel exactly like the embedded fspec TUI's

  Scenario: Embedded and WebSocket backends return byte-identical SessionInfo lists for the same SharedFspecService
    Given a SharedFspecService bound to a workspace cwd (no SessionManager attached)
    When EmbeddedFspecBackend.list_sessions().await is called against the service
    And WebSocketFspecBackend.list_sessions().await is called against the same service over a loopback daemon
    Then both backends return Vec<SessionInfo> of the same length with the same id field in the same order

  Scenario: Embedded and WebSocket backends return byte-identical HistoryMatch lists for the same query
    Given a SharedFspecService whose persistence store contains submitted inputs "git status", "git push", "fspec board" under SessionId("s-1")
    When EmbeddedFspecBackend.persistence_search_history("git").await is called
    And WebSocketFspecBackend.persistence_search_history("git").await is called against the same service over a loopback daemon
    Then both backends return Vec<HistoryMatch> with the same length and the same text field in the same order
