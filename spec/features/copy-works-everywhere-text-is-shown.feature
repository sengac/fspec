@done
@tui
@clipboard
@dialog
@agent-view
@TUI-130
Feature: Copy works everywhere text is shown
  """
  Implementation: one copy path. Component::copy_text() (default None) lets a dialog offer its whole text; App::handle_event copies it on c or Ctrl+C when the topmost active compositor layer offers text (app/dialog_copy.rs), so Error, Session error, Notification and board card dialogs copy without their own senders. CopyNotice (app/copy_notice.rs) words the OSC 52 fact. Scrollback: text_wrap::wrap_rows reports how each row ends (line break, wrap at a removed space, wrap inside a token); chunk_wrap stores it as RenderedChunk::row_ends and the turn modal derives it from the same wrap, and scrollback_copy/turn_modal_select join wrapped rows with what the wrap removed. The first row of a prefixed chunk wraps at the width minus the prefix. Board strip copies emit Action::CopyToClipboard (BoardView's own Osc52Clipboard removed) and the App ticks the board recognizer so a long press works there. Views (changed files, checkpoints, blocklist, model selector, /worktrees) emit CopyToClipboard from their keys.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. One copy path: every copy in the TUI (the copy keys of dialogs and views, scrollback, turn-modal and composer selections, and the board details strip) goes through Action::CopyToClipboard to the App's one OSC 52 writer, so every copy is confirmed the same way. The board view's own clipboard writer is removed.
  #   2. The confirmation says only what the App knows (TUI-126 note: OSC 52 has no acknowledgement, so it must not claim a success it cannot know): a written copy shows 'Sent N characters to the terminal clipboard (OSC 52)' for 2 seconds, and a failed write shows 'Copy failed: <reason>'. This replaces TUI-126's 'Copied N characters' wording.
  #   3. User request (2026-11-07): text in a dialog is copied with the copy wrapper the same way as in Provider Settings. The Error, Session error and Notification dialogs copy their whole message with c or Ctrl+C (never only the rows on screen), and their footer lists the copy key.
  #   4. Board card details: c or Ctrl+C copies the open page's whole text (the card id and title, then every body row, including rows below the fold); y copies the card id.
  #   5. Board details strip: a long press on the title row copies 'ID: title' in full, and a long press on a description row copies the whole description; a drag copies what it covers, as before.
  #   6. User request (2026-11-07): copying from the agent scrollback or the turn modal does not insert line breaks where the text wrapped. Rows the wrap split are joined with what the wrap removed (one space between words, nothing inside a long token such as a URL), so a wrapped URL or command pastes as one line; real line breaks are kept.
  #   7. The turn modal: c copies the whole turn. In SELECT mode c copies the selected turn's whole text.
  #   8. Agent view Ctrl+C: while a scrollback or composer selection is active, Ctrl+C copies the selection and clears it, so the next Ctrl+C interrupts; with no selection Ctrl+C interrupts as before (audit CP-15).
  #   9. Changed files: c or Ctrl+C copies the selected file's full path, and C copies the shown diff. Checkpoints: c or Ctrl+C copies the selected checkpoint's name.
  #   10. The /worktrees dialog: Up and Down choose a worktree row, and c or Ctrl+C copies that worktree's full path.
  #   11. Model selector: outside the filter and the custom-model form, c or Ctrl+C copies the focused model as 'provider/model-id'. A paste in the add or edit custom-model form goes into the focused field.
  #   12. Blocklist: c or Ctrl+C copies the focused rule's pattern, and C copies the whole detail block (ID, pattern, reason, guidance).
  #   13. When the attachment or FOUNDATION viewer cannot be opened in a browser, the error names the viewer URL, so it can be copied from the error dialog and opened by hand.
  #   14. A successful worktree merge leaves a Notification dialog on the board with the merge commit sha (it stays until dismissed), so the sha can be read and copied with c.
  #   15. Out of scope here: the HITL, pause and exec-stdin prompt headers (audit CP-13). AMGR-019 changes where sub-agent questions are shown (the root view, headed with who asks), so their copy is left to that work; listed as a follow-up.
  #
  # EXAMPLES:
  #   1. Copying 'abc' shows 'Sent 3 characters to the terminal clipboard (OSC 52)' for 2 seconds; a clipboard write that fails with 'broken pipe' shows 'Copy failed: broken pipe'
  #   2. An API Error dialog with a 300-character error: c puts 'API Error: ' and the whole error on the clipboard, and Ctrl+C does the same
  #   3. A Session error dialog whose message is 300 characters and a long path: c copies the whole message, including the rows scrolled out of view
  #   4. The daemon-saved Notification dialog: c copies its whole sentence
  #   5. Board card details for a card with a 500-character description: c copies the id, title and the whole description; y copies the card id
  #   6. Long press on the board strip's description row copies the whole 500-character description, not the two rows shown; the copy reaches the App clipboard and is confirmed
  #   7. A 200-character URL in the scrollback wraps over 3 rows at 80 columns; dragging over all its rows copies the URL with no line break
  #   8. An assistant reply 'first line\nsecond line that wraps' keeps exactly one line break when copied; the wrapped words are joined by one space
  #   9. In the open turn modal, c copies the whole turn
  #   10. In the agent view with a scrollback selection, Ctrl+C copies it and does not interrupt; a second Ctrl+C interrupts
  #   11. Changed files with a 120-character path: c copies the path; C copies the diff lines
  #   12. /worktrees with two 90-character paths: Down then c copies the second path
  #   13. Model selector focused on anthropic/claude-x: c copies 'anthropic/claude-x'; a paste of 'gpt-x' in the custom-model form goes into its Model ID field
  #   14. Blocklist focused on a rule: c copies its pattern; C copies ID, pattern, reason and guidance
  #   15. The FOUNDATION viewer launch fails with 'no display': the error names the 'http://127.0.0.1:' viewer URL
  #   16. A merge that returns commit 'abc123' leaves a Notification dialog on the board containing 'abc123'; c copies the notice
  #
  # ========================================
  Background: User Story
    As a user of the fspec TUI
    I want to copy any text a dialog or view shows, the same way as in Provider Settings, and get the whole unwrapped text
    So that I can paste error messages, paths, ids, commit shas and long URLs elsewhere without retyping them or fixing line breaks

  Scenario: Every copy is confirmed with what the App knows
    Given the App's clipboard writer accepts writes
    When "abc" is copied
    Then the clipboard receives exactly "abc"
    And the screen shows "Sent 3 characters to the terminal clipboard (OSC 52)"
    And the confirmation is gone 2 seconds later
    When the clipboard writer fails with "broken pipe" and "abc" is copied
    Then the screen shows "Copy failed: broken pipe"

  Scenario: The API Error dialog copies its whole message
    Given the API Error dialog shows a 300-character error
    When I press c
    Then the clipboard receives "API Error: " followed by the whole error
    When I press Ctrl+C
    Then the clipboard receives the same text again
    And the dialog footer lists "c copy"

  Scenario: The Session error dialog copies rows scrolled out of view
    Given the Session error dialog shows a message longer than the dialog
    When I press c
    Then the clipboard receives the whole message

  Scenario: The Notification dialog copies its whole sentence
    Given the daemon-saved Notification dialog is shown
    When I press c
    Then the clipboard receives its whole sentence

  Scenario: Board card details copy the whole page and the card id
    Given the board card details are open for a card with a 500-character description
    When I press c
    Then the clipboard receives the card id, its title and the whole description
    When I press y
    Then the clipboard receives the card id

  Scenario: A long press on the board strip copies the whole field through the App
    Given the board details strip shows a card with a 500-character description
    When I long-press the strip's description row
    Then the App clipboard receives the whole description
    And the screen shows "Sent 500 characters to the terminal clipboard (OSC 52)"

  Scenario: A wrapped URL copied from the scrollback has no line breaks
    Given the scrollback shows a 200-character URL that wraps over 3 rows at 80 columns
    When I drag over all its rows
    Then the clipboard receives the URL with no line break

  Scenario: Wrapped words are joined by a space and real line breaks are kept
    Given the scrollback shows an assistant reply whose second line wraps
    When I drag over all its rows
    Then the clipboard holds exactly one line break, where the reply had one
    And the wrapped words are joined by one space

  Scenario: The turn modal copies the whole turn
    Given the turn modal is open on a long assistant reply
    When I press c
    Then the clipboard receives the whole turn text

  Scenario: Ctrl+C copies an agent view selection instead of interrupting
    Given the agent view has a scrollback selection
    When I press Ctrl+C
    Then the clipboard receives the selection
    And no interrupt is sent
    When I press Ctrl+C again
    Then an interrupt is sent

  Scenario: Changed files copy the full path and the diff
    Given the changed files view lists a file with a 120-character path
    When I press c
    Then the clipboard receives the full path
    When I press C
    Then the clipboard receives the shown diff lines

  Scenario: Checkpoints copy the selected checkpoint name
    Given the checkpoints view lists a checkpoint
    When I press c
    Then the clipboard receives the checkpoint's name

  Scenario: The worktrees dialog copies the chosen worktree path
    Given the /worktrees dialog lists two worktrees with 90-character paths
    When I press Down and then c
    Then the clipboard receives the second worktree's path

  Scenario: The model selector copies the focused model and takes a paste in its form
    Given the model selector is focused on a model of a provider
    When I press c
    Then the clipboard receives "provider/model-id" for that model
    When I open the add custom model form and paste "gpt-x"
    Then the focused form field holds "gpt-x"

  Scenario: The blocklist copies the rule pattern and the whole detail
    Given the blocklist is focused on a rule
    When I press c
    Then the clipboard receives the rule's pattern
    When I press C
    Then the clipboard receives the rule's ID, pattern, reason and guidance

  Scenario: A viewer that cannot open names its URL
    Given the browser launcher fails with "no display"
    When I open the FOUNDATION viewer
    Then the error shown names the "http://127.0.0.1:" viewer URL

  Scenario: The merge commit sha stays on the board and can be copied
    Given a worktree merge returns commit "abc123"
    When the session returns to the board
    Then a Notification dialog on the board shows "abc123"
    When I press c
    Then the clipboard receives the notification text
