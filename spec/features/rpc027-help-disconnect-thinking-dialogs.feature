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

  Scenario: HelpDialog body lists every RPC-009 keybinding
    Given an isolated HelpDialog component
    When I render it onto an 80x24 TestBackend buffer
    Then the rendered buffer contains "j/k"
    And the rendered buffer contains "Tab"
    And the rendered buffer contains "?"
    And the rendered buffer contains "q"
    And the rendered buffer contains "Enter"
    And the rendered buffer contains "Ctrl+C"
    And the rendered buffer contains "ESC"

  Scenario: HelpDialog no longer imports tui_popup
    Given the source file rust/fspec-tui/src/components/help_dialog.rs
    Then the source does not contain the substring "tui_popup::Popup"
    And the source does not contain "Popup::new("
    And the source imports dialog_theme::render_dialog

  # ============================================================
  # Section C — DisconnectDialog
  # ============================================================
  Scenario: DisconnectDialog renders with the red accent and the "Disconnected" inner title
    Given a fresh DisconnectDialog with no Reconnecting action applied
    When I render it onto an 80x24 TestBackend buffer
    Then the border cells use foreground color Color::Red
    And the body's first non-padding row contains the text "Disconnected"
    And the "Disconnected" text cells have foreground color Color::Red with BOLD modifier
    And the body contains the line "daemon disconnected"
    And the body contains the line "q to quit"
    And the body contains the line "r to reconnect"

  Scenario: DisconnectDialog updates the body inline on Action::Reconnecting(N)
    Given a fresh DisconnectDialog
    When I dispatch Action::Reconnecting(3)
    And I render it onto an 80x24 TestBackend buffer
    Then the body contains the substring "auto-reconnecting (attempt 3)"
    And the border cells still use foreground color Color::Red
    And the "Disconnected" title is still painted with BOLD red foreground

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
    And the "Low", "Medium", and "High" rows begin with the two-character marker "  "
    And the description text for unselected rows carries Modifier::DIM

  Scenario: ThinkingLevelDialog footer documents the four key bindings
    Given a ThinkingLevelDialog seeded with ThinkingLevel::Off
    When I render it onto an 80x24 TestBackend buffer
    Then the last body row contains the substring "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"
    And the footer text carries Modifier::DIM
    And the footer text is horizontally centered

  Scenario: Pressing D in ThinkingLevelDialog emits Action::SetThinkingLevelDefault and keeps the dialog open
    Given a ThinkingLevelDialog seeded with ThinkingLevel::Off and currently highlighting "Medium"
    When I send a KeyCode::Char('d') event
    Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::Medium)
    And the dialog returns EventResult::Consumed without a remove-callback
    And the dialog is still mounted on the compositor

  Scenario: Pressing uppercase D in ThinkingLevelDialog behaves identically to lowercase d
    Given a ThinkingLevelDialog seeded with ThinkingLevel::High
    When I send a KeyCode::Char('D') event
    Then the dialog emits Action::SetThinkingLevelDefault(session_id, ThinkingLevel::High)
    And the dialog is still mounted on the compositor

  Scenario: SetThinkingLevelDefault is wired through the backend trait stack
    Given the codelet_rpc_types::Action enum
    Then it contains the variant SetThinkingLevelDefault(SessionId, ThinkingLevel)
    Given the SessionManagerHandle trait
    Then it declares set_thinking_level_default with a default no-op implementation returning Ok(())
    Given the FspecBackend trait
    Then it declares set_thinking_level_default on both transports
    Then dispatch_model_thinking_dialogs.rs routes Action::SetThinkingLevelDefault to backend.set_thinking_level_default
