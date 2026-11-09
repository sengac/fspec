@BUG-582
@done
@refactor
@rust
@ui-refinement
@tui
@dialog
@rpc
@RPC-027
Feature: RPC-027 — HelpDialog, DisconnectDialog and ThinkingLevelDialog migration
  """
  RPC-027 Sections B (HelpDialog), C (DisconnectDialog), D (ThinkingLevelDialog).

  The shared dialog_theme.rs renderer (see rpc027-dialog-theme.feature)
  is the only thing these dialogs paint with — tui_popup::Popup is
  removed from every render() impl.

  ThinkingLevelDialog gains the missing 'D Set Default' keybinding
  from the TypeScript reference (ThinkingLevelDialog.tsx lines 93–96).
  TUI-132 (2026-11-09): Section C is gone with the DisconnectDialog, which nothing showed since RPC-416.
  """

  Background: User Story
    As a developer maintaining the rust/fspec-tui Rust ratatui frontend
    I want HelpDialog DisconnectDialog and ThinkingLevelDialog to render with the canonical theme
    So that they look identical to their TypeScript Ink counterparts

  # ============================================================
  # Section B — HelpDialog
  # ============================================================
  Scenario: HelpDialog renders with the cyan accent and inner-title body
    Given an isolated HelpDialog component
    When I render it onto an 80x24 TestBackend buffer
    Then the border cells use foreground color Color::Cyan
    And the body's first non-padding row contains the text "Help"
    And the "Help" text cells have foreground color Color::Cyan with BOLD modifier
    And the top border row does NOT contain the text "Help"

  Scenario: HelpDialog body lists the board keybindings
    Given an isolated HelpDialog component
    When I render it onto an 80x24 TestBackend buffer
    Then the rendered buffer contains "Navigate"
    And the rendered buffer contains "New Agent"
    And the rendered buffer contains "Reorder"
    And the rendered buffer contains "Enter"
    And the rendered buffer does NOT contain the old "q       Quit" line

  Scenario: HelpDialog no longer imports tui_popup
    Given the source file rust/fspec-tui/src/components/help_dialog.rs
    Then the source does not contain the substring "tui_popup::Popup"
    And the source does not contain "Popup::new("
    And the source imports dialog_theme::render_dialog

  # ============================================================
  # Section D — ThinkingLevelDialog
  # ============================================================
  Scenario: ThinkingLevelDialog renders with the yellow accent and inner-title body
    Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    When I render it onto an 80x24 TestBackend buffer
    Then the border cells use foreground color Color::Yellow
    And the body's first non-padding row contains the text "Thinking Level"
    And the "Thinking Level" text cells have foreground color Color::Yellow with BOLD modifier

  Scenario: ThinkingLevelDialog highlights the current level with the inverse style
    Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    When I render it onto an 80x24 TestBackend buffer
    Then the "Off" row has background color Color::Yellow
    And the "Off" row has foreground color Color::Black with BOLD modifier
    And the "Off" row begins with the two-character marker "▸ "
    And the "low", "medium", and "high" rows begin with the two-character marker "  "
    And the description text for unselected rows carries Modifier::DIM

  Scenario: ThinkingLevelDialog footer documents the four key bindings
    Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    When I render it onto an 80x24 TestBackend buffer
    Then the last body row contains the substring "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"
    And the footer text carries Modifier::DIM
    And the footer text is horizontally centered

  Scenario: Pressing D in ThinkingLevelDialog emits Action::SetThinkingLevelDefault and keeps the dialog open
    Given a ThinkingLevelDialog seeded with ThinkingLevel::Off and currently highlighting "medium"
    When I send a KeyCode::Char('d') event
    Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::Effort("medium"))
    And the dialog returns EventResult::Consumed without a remove-callback
    And the dialog is still mounted on the compositor

  Scenario: Pressing uppercase D in ThinkingLevelDialog behaves identically to lowercase d
    Given a ThinkingLevelDialog seeded with ThinkingLevel::Effort("high")
    When I send a KeyCode::Char('D') event
    Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::Effort("high"))
    And the dialog is still mounted on the compositor

  Scenario: SetThinkingLevelDefault is wired through the backend trait stack
    Given the codelet_rpc_types::Action enum
    Then it contains the variant SetThinkingLevelDefault(SessionId, ThinkingLevel)
    Given the SessionManagerHandle trait
    Then it declares set_thinking_level_default with a default no-op implementation returning Ok(())
    Given the FspecBackend trait
    Then it declares set_thinking_level_default on both transports
    Then dispatch_model_thinking_dialogs.rs routes Action::SetThinkingLevelDefault to backend.set_thinking_level_default
