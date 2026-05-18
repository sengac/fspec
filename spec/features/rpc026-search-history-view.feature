@done
@multi-session
@rpc
@history-search
@session-resume
@agent-view
@tui
@RPC-026
Feature: SearchHistoryView — full-screen history typeahead palette (RPC-021c)

  """
  RPC-021c depends on RPC-024 (multi-session AgentViewStore) and RPC-025 (persistence_search_history RPC method + HistoryMatch type). RPC-026 does NOT introduce new RPC methods or new shared types — only consumes them.
  Render shape: SearchHistoryView::render(area, buf, store) takes the FULL area Rect. Its first statement is `Clear.render(area, buf)` so the underlying AgentView pixels are fully overwritten. It splits `area` vertically with Layout::default().constraints([Length(1), Length(1), Min(0), Length(1)]) for title row, separator, scrollable list, and footer hint — mirroring TS AgentView.tsx:5053-5191.
  Scroll machinery: visible_rows = area.height.saturating_sub(chrome_rows) where chrome_rows accounts for title + separator + footer (3 rows). Selection movement updates both selected_index AND scroll_offset using the standard formula: if selected_index < scroll_offset → scroll_offset = selected_index; if selected_index >= scroll_offset + visible_rows → scroll_offset = selected_index - visible_rows + 1.
  """

  # See spec/features/rpc026-* for the broader RPC-026 example-mapping context.
  # This file covers SearchHistoryView typeahead, empty-result placeholder, and scrolling.

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to press /resume or /search (and Ctrl+R) to open full-screen mode views that mirror the TypeScript Ink TUI — listing resumable sessions or filtering submitted-input history with delete confirmation — rather than small floating popups
    So that the Rust frontend's `/resume` and `/search` UX matches the existing TypeScript frontend pixel-for-pixel and feature-for-feature, so habits and integration tests carry across implementations unchanged

  @search @typeahead
  Scenario: Typing characters triggers persistence_search_history and folds results
    Given search_view is open with an empty query
    When the user types "g" then "i" then "t"
    Then search_view.query becomes "git"
    And three Action::SearchHistory dispatches were emitted in order ("g", "gi", "git")
    And backend.persistence_search_history was invoked with "git"
    And the backend returned two HistoryMatch values
    When Action::HistorySearchResults is folded into search_view
    Then search_view.matches has length 2
    And search_view.selected_index equals 0
    And the rendered list shows both rows with the first highlighted

  @search @placeholder
  Scenario: Non-empty query with zero matches renders the no-match placeholder
    Given search_view is open with query "xyzzy"
    And backend.persistence_search_history("xyzzy") returned an empty Vec
    When AgentView.render_with_store paints
    Then the body shows the placeholder "(no history matches \"xyzzy\")"
    When the user presses Enter
    Then the keystroke is ignored
    When the user presses Esc
    Then Action::CloseSearchView is dispatched
    And AgentView.search_view is None

  @search @scrolling
  Scenario: SearchHistoryView scrolls past 10 rows using terminal height
    Given search_view has 40 HistoryMatch values
    And the render area height is 24
    When the user presses ↓ fifteen times
    Then search_view.selected_index equals 15
    And search_view.scroll_offset has advanced so row 15 falls inside the visible window
    When the user presses ↓ until selection wraps past index 39
    Then search_view.selected_index equals 0
    And search_view.scroll_offset resets to 0
