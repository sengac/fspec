@done
@multi-session
@rpc
@history-search
@session-resume
@agent-view
@tui
@RPC-026
Feature: AgentView mode-view routing and Ctrl+R keybinding (RPC-021c)

  """
  Mode-view ownership stays on AgentView (NOT Navigator) because: (a) they only make sense inside Agent mode (Navigator already ensures that); (b) they consume the SAME action_tx and backend the AgentView wires to its input; (c) the AgentView's existing input/popup state must survive the mode view (Esc returns to AgentView with input intact). The render gate is a simple `if let Some(v) = self.resume_view { v.render(area, buf, store); return; }` early-return at the TOP of AgentView::render_with_store, BEFORE the layout split.
  Ctrl+R routing: views/agent/dispatch.rs::handle_event is amended with a guard `if event_is_ctrl_r(event) && self.search_view.is_none() && self.resume_view.is_none() && self.slash_popup.is_none() && self.file_popup.is_none() { return EventResult::Action(Action::OpenSearchView); }` placed BEFORE the existing slash_popup / file_popup routing. The chord is detected by both crossterm's parsed key (KeyModifiers::CONTROL + KeyCode::Char('r')) AND, for terminals that send a raw byte (^R = 0x12), the equivalent literal byte sequence — mirroring the dual-detection pattern RPC-019 uses for Shift+arrow chords.
  """

  # See spec/features/rpc026-* for the broader RPC-026 example-mapping context.
  # This file covers AgentView render-time mode-view early-return and the Ctrl+R chord routing.

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to press /resume or /search (and Ctrl+R) to open full-screen mode views that mirror the TypeScript Ink TUI — listing resumable sessions or filtering submitted-input history with delete confirmation — rather than small floating popups
    So that the Rust frontend's `/resume` and `/search` UX matches the existing TypeScript frontend pixel-for-pixel and feature-for-feature, so habits and integration tests carry across implementations unchanged

  @resume @render
  Scenario: ResumeSessionView paints full-screen and hides the normal AgentView layout
    Given resume_view is open with 3 SessionInfo values
    When AgentView.render_with_store is called with area Rect(0, 0, 120, 24)
    Then the buffer contains a row whose text is "Resume Session (3 available)"
    And every cell inside the 120×24 area is overwritten by ResumeSessionView (Clear was painted)
    And no "Agent —" scrollback title row appears in the buffer
    And the footer row contains "Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"

  @search @keybinding
  Scenario: Ctrl+R opens the same search view as /search
    Given AgentView has no popups or mode views open
    When the user presses Ctrl+R
    Then Action::OpenSearchView is dispatched
    And AgentView.search_view is Some(default SearchHistoryView)
    When the user presses Ctrl+R again
    Then the chord is forwarded to the search_view which returns Ignored
    And search_view stays open with unchanged query and matches
