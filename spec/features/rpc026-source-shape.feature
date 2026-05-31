@done
@multi-session
@rpc
@history-search
@session-resume
@agent-view
@tui
@RPC-026
Feature: Source-shape regression for RPC-026 mode-view files (RPC-021c)
  """
  Source-shape regressions guard the architectural decisions: new mode-view files must exist with the required structure (each < 300 LoC, first render statement is Clear, no tui_popup/popup_body imports), and the old popup files (resume_picker.rs, search_palette.rs) plus their identifiers (ResumePicker, SearchPalette) must be fully removed.
  """

  # See spec/features/rpc026-* for the broader RPC-026 example-mapping context.
  # This file covers source-shape regressions: file existence, line-count caps,
  # forbidden imports, and removal of the old popup implementations.
  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to press /resume or /search (and Ctrl+R) to open full-screen mode views that mirror the TypeScript Ink TUI — listing resumable sessions or filtering submitted-input history with delete confirmation — rather than small floating popups
    So that the Rust frontend's `/resume` and `/search` UX matches the existing TypeScript frontend pixel-for-pixel and feature-for-feature, so habits and integration tests carry across implementations unchanged

  @source-shape
  Scenario: New view files exist with the required shape and forbidden imports absent
    Given the repository is at the RPC-026 implementing snapshot
    When the source-shape regression test runs
    Then codelet/fspec-tui/src/views/agent/resume_session_view.rs exists with line count < 300
    And codelet/fspec-tui/src/views/agent/search_history_view.rs exists with line count < 300
    And codelet/fspec-tui/src/views/agent/confirm_dialog.rs exists with line count < 300
    And resume_session_view.rs contains no occurrences of "tui_popup" or "popup_body"
    And search_history_view.rs contains no occurrences of "tui_popup" or "popup_body"
    And the first non-attribute statement in each mode view's render fn is "Clear.render(area, buf)" or "frame.render_widget(Clear, area)"
    And codelet/fspec-tui/src/views/agent.rs line count is < 300
    And codelet/fspec-tui/src/app/dispatch_rpc026.rs line count is < 300

  @source-shape
  Scenario: Old popup files are removed and their identifiers no longer appear
    Given the repository is at the RPC-026 implementing snapshot
    When ripgrep searches codelet/fspec-tui/src/ for "ResumePicker"
    Then zero matches are returned
    When ripgrep searches codelet/fspec-tui/src/ for "SearchPalette"
    Then zero matches are returned
    And codelet/fspec-tui/src/views/agent/resume_picker.rs does NOT exist
    And codelet/fspec-tui/src/views/agent/search_palette.rs does NOT exist
