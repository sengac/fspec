@done
@multi-session
@rpc
@history-search
@session-resume
@agent-view
@tui
@RPC-026
Feature: ResumeSessionView — full-screen resume session list (RPC-021c)
  """
  RPC-021c depends on RPC-024 (multi-session AgentViewStore — gives us open_sessions / append_session / cycle_session / session_index) and RPC-025 (persistence_search_history RPC method + HistoryMatch type). RPC-026 does NOT introduce new RPC methods or new shared types — only consumes them.
  Render shape: ResumeSessionView::render(area, buf, store) and SearchHistoryView::render(area, buf, store) take the FULL area Rect. Their first statement is `Clear.render(area, buf)` so the underlying AgentView pixels are fully overwritten. They then split `area` vertically with Layout::default().constraints([Length(1), Length(1), Min(0), Length(1)]) for title row, separator, scrollable list, and footer hint — mirroring TS AgentView.tsx:5053-5191. Pattern reference inside this repo: see existing full-screen views like BoardView (rust/fspec-tui/src/views/board.rs) — they paint directly into the supplied Rect, NOT into a sub-rect.
  Mode-view ownership stays on AgentView (NOT Navigator) because: (a) they only make sense inside Agent mode (Navigator already ensures that); (b) they consume the SAME action_tx and backend the AgentView wires to its input; (c) the AgentView's existing input/popup state must survive the mode view (Esc returns to AgentView with input intact). The render gate is a simple `if let Some(v) = self.resume_view { v.render(area, buf, store); return; }` early-return at the TOP of AgentView::render_with_store, BEFORE the layout split.
  Scroll machinery: both mode views compute `visible_rows = area.height.saturating_sub(chrome_rows)` where chrome_rows accounts for title + separator + footer (3 rows). Selection movement updates both selected_index AND scroll_offset using the standard formula: if selected_index < scroll_offset → scroll_offset = selected_index; if selected_index >= scroll_offset + visible_rows → scroll_offset = selected_index - visible_rows + 1. This mirrors the TS resume mode at AgentView.tsx:1347 (`resumeVisibleHeight = max(1, terminalHeight - 5)`).
  """

  # See spec/features/rpc026-* for the broader RPC-026 example-mapping context.
  # This file covers ResumeSessionView scrolling, delete-confirm flow, and empty-state placeholder.
  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to press /resume or /search (and Ctrl+R) to open full-screen mode views that mirror the TypeScript Ink TUI — listing resumable sessions or filtering submitted-input history with delete confirmation — rather than small floating popups
    So that the Rust frontend's `/resume` and `/search` UX matches the existing TypeScript frontend pixel-for-pixel and feature-for-feature, so habits and integration tests carry across implementations unchanged

  @resume
  @scrolling
  Scenario: ResumeSessionView scrolls beyond 10 rows using terminal height
    Given resume_view has 40 SessionInfo values
    And the render area height is 24
    When the user presses ↓ twenty times
    Then resume_view.selected_index equals 20
    And resume_view.scroll_offset has advanced so row 20 falls inside the visible window
    And the rendered list shows the row at index 20

  @resume
  @delete
  @dialog
  Scenario: D opens the ConfirmDialog and Enter on Primary deletes the session
    Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    And resume_view.selected_index is 1
    When the user presses D
    Then Action::RequestDeleteSession("s-2") is dispatched
    And resume_view.delete_confirm is Some(ConfirmDialog) with primary_label "Delete"
    And no backend call has been made
    When the user presses Enter while the ConfirmDialog has Primary focused
    Then Action::ConfirmDeleteSession("s-2") is dispatched
    And a tokio task spawns backend.persistence_delete_session("s-2")
    And on completion a follow-up backend.list_sessions() is fetched
    And Action::SessionListLoaded(["s-1", "s-3"]) is dispatched
    And resume_view.sessions equals ["s-1", "s-3"]
    And resume_view.delete_confirm is None

  @resume
  @placeholder
  Scenario: Empty session list renders the no-sessions placeholder
    Given resume_view is open
    When Action::SessionListLoaded with an empty Vec is folded in
    Then resume_view.sessions is empty
    And the next render shows the centred placeholder "(no sessions to resume)"
    And pressing Enter is a no-op
    And pressing D is a no-op
    And pressing Esc still dispatches Action::CloseResumeView
