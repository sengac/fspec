@done
@ui-refinement
@dialog
@tui
@rust
@TUI-094
Feature: Render (default) indicator in Rust /thinking dialog for TS parity
  """
  Threads the persisted default (load_default_thinking_level_opt -> Option<ThinkingLevel>) into ThinkingLevelDialog via a with_default_level builder; new(session_id, current_level) stays stable.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The row whose level equals the persisted default thinking level renders a ' (default)' marker appended to its description
  #   2. When no default is persisted (None), no row shows the ' (default)' marker (parity with TS defaultLevel === null)
  #   3. The (default) marker and the selection highlight are independent: the default row shows (default) whether or not it is highlighted, and the highlighted row shows the marker regardless of which row is default
  #   4. The (default) text rides on the description span (dimmed when its row is not selected), matching TS dimColor={!isSelected}
  #   5. The persisted default is loaded via load_default_thinking_level_opt and threaded into the dialog at open time (handle_open_thinking_dialog); existing behavior (selection, D keybinding, Enter/Esc/navigation) is unchanged
  #
  # EXAMPLES:
  #   1. Default is High; user opens /thinking; the High row reads 'High - ~32K tokens, deep reasoning (default)'
  #   2. Default is Medium and current selection is Off; user opens /thinking; the Off row is highlighted with the marker, and the Medium row (not highlighted) shows (default)
  #   3. No default has ever been set; user opens /thinking; none of the four rows show (default)
  #   4. Default is High and current selection is also High; user opens /thinking; the High row shows both the selection highlight marker and (default)
  #
  # ========================================
  Background: User Story
    As a Rust TUI user
    I want to see a (default) marker on the thinking level that is currently my persisted default when I open the /thinking dialog
    So that I can tell which level is the default at a glance, matching the TypeScript dialog

  Scenario: Default row shows the (default) marker appended to its description
    Given a ThinkingLevelDialog seeded with current level Off and default level High
    When I render it onto an 80x24 TestBackend buffer
    Then the High row reads "High - ~32K tokens, deep reasoning (default)"

  Scenario: No default persisted shows no (default) marker on any row
    Given a ThinkingLevelDialog seeded with current level Off and default level None
    When I render it onto an 80x24 TestBackend buffer
    Then no row in the rendered buffer contains the text "(default)"

  Scenario: Default marker is independent of the selection highlight
    Given a ThinkingLevelDialog seeded with current level Off and default level Medium
    When I render it onto an 80x24 TestBackend buffer
    Then the Off row is highlighted with the "▸" marker and shows no "(default)" text
    Then the Medium row is not highlighted and reads "(default)"

  Scenario: Default equals current selection shows both the highlight and the (default) marker
    Given a ThinkingLevelDialog seeded with current level High and default level High
    When I render it onto an 80x24 TestBackend buffer
    Then the High row begins with the "▸" selection marker
    Then the High row also reads "(default)"

  Scenario: Default marker on an unselected row is dimmed on the description span
    Given a ThinkingLevelDialog seeded with current level Off and default level High
    When I render it onto an 80x24 TestBackend buffer
    Then the "(default)" cells on the unselected High row carry the Modifier::DIM style
