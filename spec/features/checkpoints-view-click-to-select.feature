@done
@diff-viewer
@tui
@RPC-369
Feature: Click a checkpoint or file row to select it in the Checkpoints view

  """
  handle_mouse in views/checkpoints/keys.rs gains a MouseEventKind::Down(_) arm placed AFTER the existing restore/delete dialog guard (which returns Consumed) and BEFORE the wheel match. It hit-tests via pane_at(col,row), sets focused_pane to the clicked pane, then: Checkpoints pane -> move_checkpoint_selection(clicked - selected_checkpoint); Files pane -> move_file_selection(clicked - selected_file); Diff pane -> Consumed.
  Reuses existing navigation.rs setters move_checkpoint_selection(delta) (emits Action::LoadCheckpointFiles) and move_file_selection(delta) (emits Action::LoadCheckpointFileDiff); both clamp, ensure_visible, and early-return Consumed with no Emit when the clamped index equals the current selection. last_checkpoints_rect/last_files_rect are CONTENT rects so clicked_index = scroll + (ev.row - rect.y), clamped to list length; ignore clicks past the last populated row.
  No App/Navigator changes: Event::Mouse already flows App -> Navigator::handle_event -> handle_checkpoints_event -> view.handle_event -> handle_mouse, and CheckpointsEvent::Emit is relayed onto action_tx by navigator_events.rs. Test via CheckpointsView unit tests in views/checkpoints/ (mirroring existing wheel tests) that seed checkpoints/files + rects and dispatch a synthetic crossterm MouseEvent with kind Down(Left); assert selection index and emitted Action.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A left mouse Down on a checkpoint-name row in the Checkpoints pane selects that checkpoint and requests its changed-files list to load
  #   2. A left mouse Down on a file row in the Files pane selects that file and requests its diff to load
  #   3. The clicked row maps to an index via clicked_index = scroll + (ev.row - rect.y) using the matching pane's cached content rect and scroll offset (checkpoint_scroll/last_checkpoints_rect for checkpoints, file_scroll/last_files_rect for files)
  #   4. A click focuses the pane it lands in (Checkpoints, Files, or Diff); a click in the Diff pane only focuses it without changing any selection
  #   5. While the restore or delete dialog is active it captures the click and no selection change occurs (the dialog guard takes precedence over click handling)
  #   6. Clicking a row whose computed index equals the current selection is a no-op (no files/diff reload), and clicks on empty space below the last row or outside all rects change nothing
  #
  # EXAMPLES:
  #   1. With two checkpoints and the first selected, a left click on the second checkpoint's row sets the selected checkpoint index to 1 and emits Action::LoadCheckpointFiles for it
  #   2. With the selected checkpoint's files listed a.txt then b.txt and a.txt selected, a left click on b.txt's row selects file index 1 and emits Action::LoadCheckpointFileDiff for b.txt
  #   3. Clicking inside the Diff pane focuses the Diff pane and leaves both the checkpoint and file selection unchanged
  #   4. While the restore dialog is open, a left click on a checkpoint row is swallowed by the dialog and the selection does not change
  #
  # ========================================

  Background: User Story
    As a fspec user browsing the three-pane Checkpoints view
    I want to click a checkpoint name row or a file row to select it
    So that I can pick a checkpoint or file with the mouse instead of only arrow keys or the wheel

  Scenario: Clicking a checkpoint name row selects it and loads its files
    Given a Checkpoints view with two checkpoints and the first selected
    When the user left-clicks the second checkpoint's row
    Then the selected checkpoint index becomes 1
    And the view emits Action::LoadCheckpointFiles for the second checkpoint
    And the focused pane is the Checkpoints pane

  Scenario: Clicking a file row selects it and loads its diff
    Given a Checkpoints view whose selected checkpoint lists files a.txt then b.txt with a.txt selected
    When the user left-clicks the file row for b.txt
    Then the selected file index becomes 1
    And the view emits Action::LoadCheckpointFileDiff for b.txt
    And the focused pane is the Files pane

  Scenario: Clicking inside the diff pane focuses it without changing any selection
    Given a Checkpoints view with two checkpoints and the first selected
    When the user left-clicks inside the Diff pane
    Then the focused pane is the Diff pane
    And the selected checkpoint index is still 0
    And the view does not emit a checkpoint files or diff reload

  Scenario: A click is swallowed while the restore dialog is open
    Given a Checkpoints view with two checkpoints and the first selected and the restore dialog open
    When the user left-clicks the second checkpoint's row
    Then the selected checkpoint index is still 0
    And the view does not emit a checkpoint files or diff reload

  Scenario: Clicking the already-selected checkpoint row changes nothing
    Given a Checkpoints view with two checkpoints and the first selected
    When the user left-clicks the first checkpoint's row
    Then the selected checkpoint index is still 0
    And the view does not emit a checkpoint files or diff reload

  Scenario: Clicking empty space below the last checkpoint row changes nothing
    Given a Checkpoints view with two checkpoints and the first selected
    When the user left-clicks the empty area below the last checkpoint row
    Then the selected checkpoint index is still 0
    And the view does not emit a checkpoint files or diff reload
