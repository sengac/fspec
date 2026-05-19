@done
@refactor
@rust
@ui-refinement
@tui
@dialog
@rpc
@RPC-027
Feature: RPC-027 — SlashCommandPopup and FileSearchPopup migration

  """
  RPC-027 Sections G (SlashCommandPopup) and H (FileSearchPopup).
  Both replace the old popup_body.rs adapter with the shared
  dialog_theme.rs renderer. The inverse highlight switches from
  bg=Blue fg=White to the canonical bg=Cyan fg=Black. The single-char
  "▸" / " " markers become two-char "▸ " / "  " markers.
  """

  Background: User Story
    As a developer maintaining the codelet/fspec-tui Rust ratatui frontend
    I want the slash and file search popups to use the same canonical theme
    So that the inverse highlight and marker alignment match every other dialog

  # ============================================================
  # Section G — SlashCommandPopup
  # ============================================================

  Scenario: SlashCommandPopup renders with the cyan accent and "Slash Commands" inner title
    Given a SlashCommandPopup with at least one matching command
    When I render it onto an 80x24 TestBackend buffer
    Then the border cells use foreground color Color::Cyan
    And the body's first non-padding row contains the text "Slash Commands"
    And the title cells have foreground color Color::Cyan with BOLD modifier

  Scenario: SlashCommandPopup uses the two-character marker on every match row
    Given a SlashCommandPopup with three matching commands and selected_index = 1
    When I render it onto an 80x24 TestBackend buffer
    Then the row at index 0 begins with the two-character marker "  "
    And the row at index 1 begins with the two-character marker "▸ "
    And the row at index 2 begins with the two-character marker "  "

  Scenario: SlashCommandPopup highlights the selected match with the inverse cyan/black style
    Given a SlashCommandPopup with three matching commands and selected_index = 1
    When I render it onto an 80x24 TestBackend buffer
    Then the row at index 1 has background Color::Cyan and foreground Color::Black with BOLD modifier
    And no other row carries the inverse highlight

  Scenario: SlashCommandPopup footer documents Tab/Enter Select
    Given a SlashCommandPopup with matches
    When I render it onto an 80x24 TestBackend buffer
    Then the footer contains "↑↓ Navigate │ Tab/Enter Select │ Esc Close"
    And the footer carries Modifier::DIM

  # ============================================================
  # Section H — FileSearchPopup
  # ============================================================

  Scenario: FileSearchPopup renders with the cyan accent and "File Search" inner title
    Given a FileSearchPopup with at least one match
    When I render it onto an 80x24 TestBackend buffer
    Then the border cells use foreground color Color::Cyan
    And the body's first non-padding row contains the text "File Search"
    And the title cells have foreground color Color::Cyan with BOLD modifier

  Scenario: FileSearchPopup uses the two-character marker on every match row
    Given a FileSearchPopup with three matches and selected_index = 0
    When I render it onto an 80x24 TestBackend buffer
    Then the row at index 0 begins with the two-character marker "▸ "
    And rows 1 and 2 begin with the two-character marker "  "

  Scenario: FileSearchPopup empty-state literals render in plain text
    Given a FileSearchPopup with no matches and an empty filter
    When I render it onto an 80x24 TestBackend buffer
    Then the body contains the literal "(type to search files)"
    And the literal carries no inverse highlight

  Scenario: FileSearchPopup no-match state renders with the filter quoted
    Given a FileSearchPopup with filter "zzz" and zero matches
    When I render it onto an 80x24 TestBackend buffer
    Then the body contains the literal "(no files match \"zzz\")"
    And the literal carries no inverse highlight
