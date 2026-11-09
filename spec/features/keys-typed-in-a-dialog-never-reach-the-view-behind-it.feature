@done
@tui
@rust
@interaction
@TUI-128
Feature: Keys typed in a dialog never reach the view behind it
  """
  codelet-fspec-tui only. App::route_event (app/events.rs): after Stage 2, when the compositor did not consume a key press or paste and its topmost layer is Foreground or Critical, the event stops (consumed) before the mux Shift+Left/Right intercept, Stage 3 (Navigator) and Stage 4 (app shortcuts); Ctrl+D still quits. route_paste applies the same rule. Every compositor layer is Foreground or Critical, so this covers every dialog; mouse routing is unchanged. Tests drive App::handle_event / handle_paste over MockBackend with the dialogs opened through their real actions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. While a dialog (a Foreground or Critical compositor layer) is on top, every key press and paste goes only to the dialogs: a key or paste no dialog uses is dropped and never reaches the view behind (the board, an agent view, the mux, its Shift+Left/Right pane keys) or the board shortcuts (? and Esc)
  #   2. Ctrl+D quits fspec from anywhere, including while a dialog is open, unless the dialog uses Ctrl+D itself (the role dialog clears the role)
  #   3. Mouse events are out of scope: a click outside a dialog keeps its current routing (some dialogs pass it to the board on purpose)
  #   4. The disconnect dialog keeps its own rule: only q and r act
  #
  # EXAMPLES:
  #   1. An API Error dialog covers the agent view of s-1; the user types 'ok' and presses Enter: nothing is sent to s-1, its composer is empty and the dialog is still open; after Esc closes it, typing 'ok' fills the composer
  #   2. The board help is open; the user presses p, f and c: the board stays on the board, help stays open, and Provider Settings, Changed Files and Checkpoints do not open
  #   3. The 'Close Agent?' dialog is open over s-1 with s-2 also open; the user presses Tab and Shift+Right: s-1 stays focused and turn-select mode is not entered
  #   4. The 'Merge Worktree' dialog is open over s-1; the user types 'x', presses ? and pastes 'hello': the composer stays empty and no help opens
  #   5. A notification is showing over s-1; the user types 'hi': the composer stays empty
  #   6. The 'Merge Worktree' dialog is open; the user presses Ctrl+D: fspec quits
  #
  # QUESTIONS (ANSWERED):
  #   Q: the auto-dismissing notification ("Closing in Ns... (ESC to dismiss)") is drawn as a dialog; should keys typed while it shows be dropped like every other dialog, or go to the view behind it? Conservative option taken: it is a dialog, so keys go to it only (Esc dismisses; other keys are dropped)
  #   A: Conservative option taken (supervisor instruction, 2026-11-07, pending product owner review): the notification is a Critical dialog like the error dialog, so while it shows a key or paste reaches nothing behind it; Esc dismisses it and it closes by itself after its countdown.
  #
  # ========================================
  Background: User Story
    As a user with a dialog open in the TUI
    I want to have every key and paste go to the dialog on top and nowhere else
    So that typing into or dismissing a dialog never sends a message, opens a view or changes a session hidden behind it

  Scenario: Typing under an API Error dialog sends nothing to the session behind it
    Given session s-1 is focused on the agent view
    And an API Error dialog is open over it
    When the user types "ok" and presses Enter
    Then nothing is sent to s-1
    And the composer of s-1 is empty
    And the API Error dialog is still open
    When the user presses Esc
    And types "ok"
    Then the API Error dialog is closed
    And the composer of s-1 holds "ok"

  Scenario: Board keys pressed while the board help is open open nothing behind it
    Given the board is shown
    And the user has opened the board help with "?"
    When the user presses "p", "f" and "c"
    Then the board help is still open
    And the board is still shown

  Scenario: Tab and Shift+Right under the close dialog change nothing behind it
    Given sessions s-1 and s-2 are open and s-1 is focused on the agent view
    And the "Close Agent?" dialog is open over s-1
    When the user presses Tab and Shift+Right
    Then s-1 is still focused
    And turn-select mode is off
    And the "Close Agent?" dialog is still open

  Scenario: Keys, ? and a paste under the merge dialog reach nothing behind it
    Given session s-1 is focused and its worktree has changes
    And the "Merge Worktree" dialog is open over s-1
    When the user types "x", presses "?" and pastes "hello"
    Then the composer of s-1 is empty
    And no help dialog is open
    And the "Merge Worktree" dialog is still open

  Scenario: Typing while a notification shows reaches nothing behind it
    Given no session is open and the agent view is shown
    And the notification from /debug is showing
    When the user types "hi"
    Then the composer is empty
    When the user presses Esc
    And types "hi"
    Then the composer holds "hi"

  Scenario: Ctrl+D quits while a dialog is open
    Given session s-1 is focused and its worktree has changes
    And the "Merge Worktree" dialog is open over s-1
    When the user presses Ctrl+D
    Then fspec quits
