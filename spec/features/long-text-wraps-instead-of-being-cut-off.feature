@done
@dialog
@agent-view
@tui
@TUI-131
Feature: Long text wraps instead of being cut off
  """
  fspec-tui only. Dialogs: a wrapped-rows builder in components/dialog_theme_rows.rs (text_wrap::wrap_to_width per line, so line breaks start rows) gives DialogRows for a width taken from the area (dialog_theme sizing caps the width at the area); ErrorDialog and NotificationDialog switch to the SessionStorageDialog fill_rect + scroll pattern when the rows exceed the screen; checkpoint_restore_dialog, add_workspace/role errors and session_worktrees rows use the builder (worktree path on its own rows, list scrolls). Full-screen views: full_screen_shell sizes the footer to its wrapped row count (CHROME_ROWS becomes footer-aware), render_footer_hint wraps; board footer (views/board/footer.rs, render.rs Length(1)) and board_card/frame.rs chrome become dynamic. Panes: blocklist panes.rs and the profile form wrap with wrapped_lines; diff panes wrap diff lines and scroll in wrapped rows; diff_common::truncate_path keeps the tail. stdin_answer_box wraps the notice and errors and counts rows in focused_stdin_box_height. Drafts reuse daemon_form_dialog::scroll_window. Tests: one App-driven test file tui131_long_text_wraps_instead_of_being_cut_off.rs rendering at 80 and 120 columns (helpers from tui126/tui130).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. User request (2026-11-07): text that does not fit wraps onto more rows instead of being cut off at the edge, in every dialog, pane, status row and footer, at 80 and at 120 columns alike; one wrap helper (views/agent/text_wrap, TUI-126) is used everywhere, measuring display width
  #   2. A line break in the text starts a new row; it is never painted into a cell (WR-1)
  #   3. A dialog grows with its wrapped rows up to the screen height; past that its body scrolls with Up/Down/PgUp/PgDn/Home/End and the wheel, and its footer and buttons stay visible (Error, Notification, /worktrees)
  #   4. Full-screen view footers (board card details, changed files, checkpoints, blocklist, model selector) and the board footer wrap onto as many rows as they need; the body gets the rows that are left
  #   5. Status and error rows wrap in full: the board card details attachment error, the model selector write error, the add-workspace error, the checkpoint restore and delete errors, the exec-stdin probe and send errors, and the Provider Settings profile-form status
  #   6. A one-row text field (add-workspace path, role draft) shows the part around the cursor, with … where text is hidden, so the end of a long typed or pasted value is visible
  #   7. A changed-file or checkpoint list row stays one row; a path too long for it is shortened at the start, keeping the file name (…/provider_settings/oauth_login.rs); c copies the full path (TUI-130)
  #   8. Diff lines in the changed-files and checkpoints diff panes wrap; scrolling counts wrapped rows
  #   9. The exec-stdin notice keeps its key hints (Ctrl+O answer · Ctrl+X dismiss) visible whatever the command or probe error length
  #   10. Copy is unchanged: c copies the unwrapped text (TUI-130), never the wrapped rows
  #
  # EXAMPLES:
  #   1. A 300-character API error with two line breaks: at 80x24 and at 120x40 every part of it is on screen and no cell holds a line break
  #   2. Saving new settings for the connected daemon build-server-eu-west: the notification shows '…the next time it connects to build-server-eu-west.' at 80 and at 120
  #   3. Restoring a file whose path is 100 characters long: the confirmation shows 'This overwrites the working copy.' at 80 and at 120
  #   4. /worktrees with three 90-character worktree paths at 80x24: each full path is on screen; with 40 worktrees, Prune and Cancel stay visible and Down reaches the last one
  #   5. A blocklist rule with 200 characters of guidance: the guidance's last word is on screen at 80 and at 120
  #   6. A changed file at 'a/' x30 + 'file_name.rs': its row ends with 'file_name.rs' at 80; a 120-character diff line is on screen whole at 80 and at 120
  #   7. An exec-stdin request for a 70-character command at 80: 'Ctrl+X dismiss' is on screen; with a 150-character probe error both the error and the keys are on screen
  #   8. A 200-character custom-model write error is on screen whole at 80 and at 120
  #   9. Board card details at 80x24: the footer shows 'wheel scroll' and 'y copy id'; an attachment error with a 60-character path and an 80-character message is on screen whole
  #   10. Pasting a 200-character path into Add workspace at 80: its last 20 characters are on screen; a 300-character add error is on screen whole
  #   11. At 80 the blocklist footer shows 'Esc: Close', changed files 'C: Copy Diff', checkpoints 'c: Copy Name' (also at 120), the model selector 'Esc Close', and each list still shows its rows
  #   12. The board footer shows 'ESC Exit' at 80 and at 120 (TUI-129 wording)
  #   13. A 250-character profile-form save error is on screen whole at 80 and at 120
  #   14. A 4000-character API error at 80x24: the dialog fills the screen, Down shows the next rows, and 'c copy · Press ESC to dismiss' stays visible
  #
  # ========================================
  Background: User Story
    As a TUI user reading errors, paths, prompts and key hints
    I want to see every long text whole, wrapped to the screen width, at any terminal width
    So that nothing I need to read or act on is cut off at the screen edge

  Scenario: A long API error with line breaks is shown whole at 80 and at 120 columns
    Given the agent view is open
    When the stream reports a 300-character API error that contains two line breaks
    Then at 80x24 every line of the error is on screen whole, wrapped to the dialog width
    And no cell of the screen holds a line break
    And at 120x40 every line of the error is on screen whole

  Scenario: An API error taller than the screen scrolls and keeps its footer
    Given the agent view is open
    When the stream reports a 4000-character API error
    Then at 80x24 the error dialog fills the screen and "c copy · Press ESC to dismiss" is on screen
    And the error's last word is not on screen
    When the user presses End
    Then the error's last word is on screen and "c copy · Press ESC to dismiss" is still on screen

  Scenario: The daemon switch notification shows its whole sentence
    Given the board is open
    When the user saves new settings for the connected daemon named "build-server-eu-west"
    Then the notification text "the next time it connects to build-server-eu-west." is on screen whole at 80x24
    And it is on screen whole at 120x40

  Scenario: The checkpoint restore confirmation shows its warning for a long path
    Given the checkpoints view lists a checkpoint with a file whose path is 100 characters long
    When the user presses r on that file
    Then the confirmation "Restore <path>? This overwrites the working copy." is on screen whole at 80x24
    And it is on screen whole at 120x40

  Scenario: The worktrees dialog shows each full path and keeps its buttons
    Given the session has three worktrees whose paths are 90 characters long
    When the user opens /worktrees at 80x24
    Then each worktree's full path is on screen whole
    Given the session has 40 worktrees
    When the user opens /worktrees at 80x24
    Then "Prune" and "Cancel" are on screen
    When the user presses End
    Then the last worktree's full path is on screen whole and "Prune" is still on screen

  Scenario: The blocklist detail pane wraps a long guidance
    Given the blocklist view shows a rule whose guidance is 200 characters long
    When the view is drawn at 80x24
    Then the guidance is on screen whole, wrapped to the detail pane
    And it is on screen whole at 120x40

  Scenario: A changed file keeps its file name and a long diff line wraps
    Given the changed files view lists a file whose path is "a/" repeated 30 times then "file_name.rs"
    And its diff has a 120-character added line
    When the view is drawn at 80x24
    Then the file's row ends with "file_name.rs"
    And the 120-character diff line is on screen whole, wrapped to the diff pane
    And at 120x40 the diff line is on screen whole

  Scenario: The exec-stdin notice keeps its key hints for a long command or probe error
    Given the agent view is open
    When a tool asks for stdin for a 70-character command
    Then "Ctrl+O answer · Ctrl+X dismiss" is on screen at 80x24 and at 120x40
    When the probe for that request fails with a 150-character error
    Then the probe error is on screen whole at 80x24
    And "Ctrl+O answer · Ctrl+X dismiss" is still on screen

  Scenario: A long model selector write error is shown whole
    Given the model selector is open
    When saving a custom model fails with a 200-character error
    Then the error is on screen whole at 80x24
    And it is on screen whole at 120x40

  Scenario: The board card details footer and a long attachment error are shown whole
    Given the board card details are open at 80x24
    Then the footer is on screen whole, including "wheel scroll" and "y copy id"
    When opening an attachment with a 60-character path fails with an 80-character message
    Then the error is on screen whole at 80x24
    And at 120x40 the footer and the error are on screen whole

  Scenario: Add workspace shows the end of a long path and the whole error
    Given the Add workspace dialog is open
    When the user pastes a 200-character path
    Then the path's last 20 characters are on screen at 80x24
    When adding the workspace fails with a 300-character error
    Then the error is on screen whole at 80x24
    And it is on screen whole at 120x40

  Scenario: Full-screen view footers wrap and the lists keep their rows
    Given the blocklist, changed files, checkpoints and model selector views each have rows to show
    When each view is drawn at 80x24
    Then the blocklist footer is on screen whole, including "Esc: Close"
    And the changed files footer is on screen whole, including "C: Copy Diff"
    And the checkpoints footer is on screen whole, including "c: Copy Name"
    And the model selector footer is on screen whole, including "Esc Close"
    And each view still shows its first row
    And at 120x40 the checkpoints footer is on screen whole

  Scenario: The board footer is shown whole at 80 and at 120 columns
    Given the board is open
    When the board is drawn at 80x24
    Then the board footer is on screen whole, including "ESC Exit"
    And at 120x40 the board footer is on screen whole

  Scenario: A long profile form status is shown whole
    Given the Provider Settings profile form is open
    When saving the profile fails with a 250-character error
    Then the error is on screen whole at 80x24
    And it is on screen whole at 120x40
