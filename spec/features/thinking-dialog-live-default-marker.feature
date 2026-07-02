@done
@ui-refinement
@dialog
@rust
@tui
@TUI-095
Feature: Live-update (default) marker when D pressed in Rust /thinking dialog
  """
  The D-key handler in ThinkingLevelDialog::handle_event (codelet/fspec-tui/src/components/thinking_level_dialog.rs) updates self.default_index = Some(self.selected_index) so render() marks the new default row live. The marker rides the dimmable description span via label_description_default_row; after D the new default row is also the selected (non-dim) row. Pressing D still emits Action::SetThinkingLevelDefault(session_id, selected_level) and returns EventResult::Consumed without a remove callback so the dialog stays open. Mirrors TS ThinkingLevelDialog.tsx onSetDefault + AgentView defaultLevel state re-render.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing D updates the dialog's default_index to the currently selected row so the (default) marker moves to that row on the next render with no reopen
  #   2. Pressing D still emits Action::SetThinkingLevelDefault(session_id, selected_level) and keeps the dialog open
  #   3. After D, the previously-default row no longer shows (default); only the newly selected row shows it
  #   4. Navigating with arrows after D moves only the selection highlight; the (default) marker stays on the row chosen by the last D press until D is pressed again
  #   5. Pressing D on a row that is already the default is idempotent: the marker stays on that row
  #
  # EXAMPLES:
  #   1. Default is Medium; user navigates to High and presses D; the High row now reads (default) and Medium no longer does, in a single render with the dialog still open
  #   2. No default set; user selects Low and presses D; the Low row shows (default)
  #   3. Default is High and High is selected; user presses D; High still shows (default) (idempotent)
  #   4. After pressing D on High, user presses Down to Off; Off is highlighted but High still carries (default)
  #   5. Pressing D emits Action::SetThinkingLevelDefault and handle_event returns Consumed so the dialog stays open
  #
  # ========================================
  Background: User Story
    As a Rust TUI user
    I want to see the (default) marker move to the level I just pressed D on, immediately, without reopening the dialog
    So that the dialog gives instant feedback that my default changed, matching the TypeScript dialog

  Scenario: Pressing D moves the (default) marker to the selected row live
    Given a ThinkingLevelDialog seeded with current level Off and default level Medium
    And I navigate the selection down to the High row
    When I send a KeyCode::Char('d') event
    And I render it onto an 80x24 TestBackend buffer
    Then the High row reads "(default)"
    And the Medium row no longer reads "(default)"

  Scenario: Pressing D sets the marker when no default was previously set
    Given a ThinkingLevelDialog seeded with current level Off and default level None
    And I navigate the selection down to the Low row
    When I send a KeyCode::Char('d') event
    And I render it onto an 80x24 TestBackend buffer
    Then the Low row reads "(default)"

  Scenario: Pressing D on the row that is already default is idempotent
    Given a ThinkingLevelDialog seeded with current level High and default level High
    When I send a KeyCode::Char('d') event
    And I render it onto an 80x24 TestBackend buffer
    Then the High row reads "(default)"
    And no other row reads "(default)"

  Scenario: Navigating after D keeps the marker on the chosen row while only the highlight moves
    Given a ThinkingLevelDialog seeded with current level Off and default level Medium
    And I navigate the selection down to the High row
    And I send a KeyCode::Char('d') event
    When I send a KeyCode::Down event to move the selection to the Off row
    And I render it onto an 80x24 TestBackend buffer
    Then the Off row begins with the "▸" selection marker
    And the High row still reads "(default)"

  Scenario: Pressing D emits SetThinkingLevelDefault and keeps the dialog open
    Given a ThinkingLevelDialog seeded with current level Off and default level None
    And I navigate the selection down to the Medium row
    When I send a KeyCode::Char('d') event
    Then the dialog emits Action::SetThinkingLevelDefault with the Medium level
    And handle_event returns EventResult::Consumed without closing the dialog
