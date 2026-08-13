@done
@RPC-025
@rust
@tui
@agent-view
@command-history
Feature: RPC-025 App::dispatch wires Shift+↑/↓ history recall and fire-and-forget submission persistence
  """
  RPC-025 (App dispatch slice) — Wire the new history primitives into
  App::dispatch:

  - Action::HistoryPrev (already emitted by RPC-019's MultiLineInput
  on Shift+↑) walks backwards through the session's history,
  caching the live draft on entry and clamping at len-1.
  - Action::HistoryNext (Shift+↓) walks forwards, exits recall at
  index 0 and restores the cached_draft.
  - Action::InputSubmitted now fires backend.persistence_add_history
  via tokio::spawn (fire-and-forget) and resets the session's
  HistoryNavState plus cached_history_snapshot.

  Per-session HistoryNavState lives in AgentViewStore.history_state_by_session
  so each open session keeps its own recall position even after the
  user cycles between sessions via RPC-024's Shift+←/→.

  Tests: rust/fspec-tui/tests/app_dispatch_history_rpc025.rs (uses
  a fake FspecBackend with a programmable persistence_get_history
  return).
  """

  Background: User Story
    As a developer using the Rust ratatui TUI with the AgentView open
    I want Shift+↑ to recall my previously-submitted inputs per-session and submitted inputs to be auto-persisted
    So that I can iterate on similar prompts without retyping and so the history matches the TS Ink TUI byte-for-byte

  Scenario: First Shift+↑ caches the live draft, loads a fresh snapshot, and replaces input with history[0]
    Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    And history_state_by_session is empty
    And cached_history_snapshot is empty
    And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return ["third", "second", "first"]
    And the MultiLineInput buffer is "live draft"
    When App::dispatch handles Action::HistoryPrev
    Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    And history_state_by_session[SessionId("s-1")].cached_draft equals Some("live draft")
    And cached_history_snapshot[SessionId("s-1")] equals ["third", "second", "first"]
    And the MultiLineInput buffer equals "third"

  Scenario: Subsequent Shift+↑ walks backwards through the cached snapshot and clamps at len-1
    Given an AgentViewStore where history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(0), cached_draft: Some("live") }
    And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    When App::dispatch handles Action::HistoryPrev
    Then history_state_by_session[SessionId("s-1")].recall_index equals Some(1)
    And the MultiLineInput buffer equals "b"
    When App::dispatch handles Action::HistoryPrev
    Then history_state_by_session[SessionId("s-1")].recall_index equals Some(2)
    And the MultiLineInput buffer equals "c"
    When App::dispatch handles Action::HistoryPrev
    Then history_state_by_session[SessionId("s-1")].recall_index stays at Some(2)
    And the MultiLineInput buffer is still "c"
    And the backend's persistence_get_history was called exactly 0 additional times (snapshot is cached)

  Scenario: Shift+↓ walks forwards and exits recall on the final step restoring the cached_draft
    Given an AgentViewStore where history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(2), cached_draft: Some("live") }
    And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    When App::dispatch handles Action::HistoryNext
    Then history_state_by_session[SessionId("s-1")].recall_index equals Some(1)
    And the MultiLineInput buffer equals "b"
    When App::dispatch handles Action::HistoryNext
    Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    And the MultiLineInput buffer equals "a"
    When App::dispatch handles Action::HistoryNext
    Then history_state_by_session[SessionId("s-1")].recall_index equals None
    And the MultiLineInput buffer equals "live"

  Scenario: Shift+↓ from live-draft mode is a no-op
    Given an AgentViewStore where history_state_by_session[SessionId("s-1")].recall_index is None
    And the MultiLineInput buffer is "current draft"
    When App::dispatch handles Action::HistoryNext
    Then history_state_by_session[SessionId("s-1")].recall_index stays None
    And the MultiLineInput buffer is unchanged at "current draft"

  Scenario: Shift+↑ when history is empty is a no-op
    Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return an empty Vec
    And the MultiLineInput buffer is "untouched"
    When App::dispatch handles Action::HistoryPrev
    Then history_state_by_session[SessionId("s-1")].recall_index stays at None
    And cached_history_snapshot[SessionId("s-1")] is empty
    And the MultiLineInput buffer is unchanged at "untouched"

  Scenario: Each open session keeps an independent recall position across Shift+←/→ cycling
    Given an AgentViewStore with open_sessions [SessionId("s-1"), SessionId("s-2")] and current_session_index == 0
    And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return ["a", "b"]
    And the backend's persistence_get_history(SessionId("s-2"), 100) is stubbed to return ["x", "y", "z"]
    When App::dispatch handles Action::HistoryPrev
    Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    And the MultiLineInput buffer equals "a"
    When App::dispatch handles Action::SessionNext
    Then current_session_index equals 1
    And the MultiLineInput buffer is restored from open_sessions[1].input_draft (NOT from history)
    When App::dispatch handles Action::HistoryPrev
    Then history_state_by_session[SessionId("s-2")].recall_index equals Some(0)
    And the MultiLineInput buffer equals "x"
    When App::dispatch handles Action::SessionPrev
    Then history_state_by_session[SessionId("s-1")].recall_index is preserved at Some(0)
    And history_state_by_session[SessionId("s-2")].recall_index is preserved at Some(0)

  Scenario: Submitting an input fires persistence_add_history via tokio::spawn and resets the session's history state
    Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    And history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(1), cached_draft: Some("live") }
    And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    When App::dispatch handles Action::InputSubmitted("hello") for SessionId("s-1")
    Then backend.send_input(SessionId("s-1"), "hello") is invoked
    And backend.persistence_add_history(SessionId("s-1"), "hello") is invoked via tokio::spawn
    And history_state_by_session[SessionId("s-1")].recall_index is reset to None
    And history_state_by_session[SessionId("s-1")].cached_draft is reset to None
    And cached_history_snapshot[SessionId("s-1")] is cleared
    And the next Action::HistoryPrev triggers a fresh persistence_get_history round-trip

  Scenario: persistence_add_history is fire-and-forget — a slow disk write never blocks input submission
    Given a fake FspecBackend whose persistence_add_history blocks indefinitely on an internal latch
    When App::dispatch handles Action::InputSubmitted("hello") for SessionId("s-1")
    Then the dispatch call returns within 50 milliseconds without awaiting persistence_add_history
    And backend.send_input(SessionId("s-1"), "hello") is invoked exactly once
    And the spawned persistence_add_history task is still pending (the latch has not been released)
