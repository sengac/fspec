@done
@multi-session
@rpc
@history-search
@session-resume
@agent-view
@tui
@RPC-026
Feature: ConfirmDialog — centred floating overlay for destructive confirmations (RPC-021c)
  """
  ConfirmDialog: implemented as a centred floating overlay (this widget IS conceptually a popup — distinct from the mode views). It MAY use tui_popup or a manual centred Block. Layout: Block::default().borders(Borders::ALL).title(title) sized roughly width=max(40, body_width+4) × height=4-6 rows; bottom row hosts up to 3 inline button labels separated by `│`. Focus traversal uses Left/Right keys; Enter activates the focused button; Esc returns Cancel. The dialog does NOT own any backend state — it's a pure widget whose Outcome the resume_view (its parent) maps to Action::ConfirmDeleteSession or no-op.
  """

  # See spec/features/rpc026-* for the broader RPC-026 example-mapping context.
  # This file covers ConfirmDialog cancel-path behaviour (Esc / Cancel button) only.
  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to press /resume or /search (and Ctrl+R) to open full-screen mode views that mirror the TypeScript Ink TUI — listing resumable sessions or filtering submitted-input history with delete confirmation — rather than small floating popups
    So that the Rust frontend's `/resume` and `/search` UX matches the existing TypeScript frontend pixel-for-pixel and feature-for-feature, so habits and integration tests carry across implementations unchanged

  @resume
  @delete
  @dialog
  Scenario: Cancelling the ConfirmDialog leaves the resume view untouched
    Given resume_view.delete_confirm is Some(ConfirmDialog) with Primary focused
    When the user presses Esc on the dialog
    Then resume_view.delete_confirm is None
    And resume_view.sessions is unchanged
    And no backend call has been made
