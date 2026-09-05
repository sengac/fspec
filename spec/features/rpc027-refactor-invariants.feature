@done
@refactor
@rust
@ui-refinement
@tui
@dialog
@rpc
@RPC-027
Feature: RPC-027 — Refactor structural invariants and snapshot regeneration
  """
  RPC-027 Sections I (structural invariants) and J (snapshot regeneration).

  These scenarios pin the cross-cutting constraints of the refactor:
  popup_body.rs is deleted, no dialog imports tui_popup, every dialog
  file stays under 300 LoC, and the TypeScript Ink reference files are
  not modified. Every migrated dialog gets a fresh insta snapshot.
  """

  Background: User Story
    As a developer maintaining the rust/fspec-tui Rust ratatui frontend
    I want the refactor's cross-cutting invariants pinned by tests
    So that no future change can silently regress the canonical look

  # ============================================================
  # Section I — Structural invariants
  # ============================================================
  Scenario: popup_body.rs is deleted from the codebase
    Given the rust/fspec-tui crate
    Then the file rust/fspec-tui/src/views/agent/popup_body.rs does not exist
    And no source file references "mod popup_body"
    And no source file imports "popup_body::PopupBody"

  Scenario: No dialog module imports tui_popup::Popup
    Given the seven refactored dialog source files
    Then none of them contains the substring "tui_popup::Popup"
    And none of them contains the substring "Popup::new("
    And every dialog's render() method calls dialog_theme::render_dialog instead

  Scenario: Every refactored dialog file remains under 300 lines
    Given the dialog source files listed in rule [11]
    Then each file has fewer than 300 source lines (excluding tests and comments)
    And dialog_theme.rs itself has fewer than 300 source lines

  # ============================================================
  # Section J — Snapshot regeneration
  # ============================================================
  Scenario: Insta snapshot for HelpDialog is regenerated against the new theme
    Given the insta snapshot help_dialog__centered_popup_80x24
    When I render the migrated HelpDialog onto an 80x24 TestBackend buffer
    Then the snapshot row containing "Help" shows the title inside the body (not in the top border)
    And the top border row contains "╭" then horizontal box-drawing characters then "╮" with no title text

  Scenario: A new insta snapshot exists for every migrated dialog
    Given the rust/fspec-tui/src/components/snapshots/ directory
    Then there is a snapshot named help_dialog__centered_popup_80x24
    And there is a snapshot named disconnect_dialog__centered_popup_80x24
    And there is a snapshot named thinking_level_dialog__centered_popup_80x24
    And there is a snapshot named model_selector_dialog__centered_popup_80x24
    And there is a snapshot named confirm_dialog__centered_popup_80x24
    And there is a snapshot named slash_command_popup__centered_popup_80x24
    And there is a snapshot named file_search_popup__centered_popup_80x24
