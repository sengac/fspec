@done
@tui
@rust
@interaction
@TUI-129
Feature: Advertised keys do what their hints say
  """
  codelet-fspec-tui only. components/menu_bar/items.rs gains the key lookups (a row for a key in one category; the board shortcut for a key across all categories); views/board/keys.rs, the open-dropdown arms of views/board/menu_keys.rs and views/multiplex/menu_keys.rs, components/menu_bar_help.rs and components/help_content.rs read them, and the separate '/', 's', 'w' open arms go. Notices reuse Action::SessionOperationFailed. Agent End gets Action::ScrollbackEnd (jump_to_bottom), the twin of ScrollbackHome. The mux Daemons/Workspaces rows are routed in app/dispatch_daemons.rs and app/dispatch_workspaces.rs through one helper that focuses the Board pane. Tests drive App over MockBackend (AppTestHarness) and read the rendered buffer.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The menu registry (components/menu_bar/items.rs) is the one table of board shortcut keys: the board key handler, the board help and the menu key column all read it, so a key shown in any of them opens the same thing as its menu row. Letter keys work in either case; Ctrl and Alt chords are never shortcuts
  #   2. While a menu dropdown is open on the board or the mux bar, pressing a key shown in one of its rows runs that row; arrows, Enter and Esc keep their meaning (Esc closes the dropdown)
  #   3. In the menu-bar help dialog (u), pressing a key shown in a row runs that row; Esc closes the dialog
  #   4. The Help row says what it opens: the board keybindings
  #   5. The board footer names what Esc and Enter do: ESC Exit (the Exit confirmation) and Enter chooses agent or details, in the wide and the compact footer
  #   6. A board key that cannot act says why with a notice: a with no selected card or a card without attachments, s when the TUI has no daemon directory, [ and ] when the board's workspace is not known yet or the move fails
  #   7. In the agent view, Home and End on an empty input jump to the top and the bottom of the scrollback; with text in the input they move the cursor
  #   8. The agent help describes each key as it behaves: Alt+Enter adds a newline and queues the draft while the agent is busy; Esc stops a run, clears the draft, or asks to close the agent; Shift+Left/Right cycles sessions and moves pane focus in the mux
  #   9. A collapsed tool card's hint names the keys that open it in full: Tab selects a turn, then Enter opens it
  #   10. The empty-answer hint of a question names what Esc does there: back to the options after Other, cancel the question when it has no options
  #   11. In the mux, Settings > Daemons and Settings > Workspaces focus the Board pane so the dropdown shows and takes its keys; a layout with no Board pane shows a notice instead of an invisible dropdown
  #
  # EXAMPLES:
  #   1. On the board, pressing each single-key row of the menu registry (. / D A b F C M P s w ?) emits that row's action, in lower and upper case; Ctrl+F opens nothing
  #   2. The board help lists every single-key menu row with the same key and label as the menu (D FOUNDATION.md, A Attachments, b New Bot), and u Menu bar help
  #   3. The board's Tools dropdown is open; the user presses F: the Changed Files view opens and the dropdown closes
  #   4. The mux bar's Settings dropdown is open; the user presses p: the Provider Settings view opens
  #   5. The menu-bar help dialog is open; the user presses C: the dialog closes and the Checkpoints view opens
  #   6. The Help row reads '? Help - Show the board keybindings'
  #   7. The board footer at width 200 and at width 120 shows 'ESC Exit' and never 'ESC Back'; the compact one no longer says 'Work Agent'
  #   8. The selected card has no attachments; pressing a shows 'No attachments on TUI-001' and opens no picker
  #   9. The board has no workspace yet; pressing ] shows that the card cannot move yet; when the backend refuses the move, the notice names the card and the error
  #   10. 200 rows of scrollback, scrolled up five pages, empty input: End jumps to the bottom (sticks to the bottom); with 'abc' typed and the cursor at the start, End moves the cursor to the end and does not scroll
  #   11. The agent help shows 'Home / End    Top / Bottom (empty input)', 'Alt+Enter     Newline (queues the draft while busy)', an Esc row naming stop, clear and close, and no 'ESC  Back'
  #   12. A tool card with more than 8 output lines ends with '(Tab to select, Enter to view full)'
  #   13. A freeform-only question: Enter on an empty answer shows 'press Esc to cancel'; after choosing Other on an options question it shows 'press Esc to go back'
  #   14. Mux Board|Agent with the agent pane focused: Settings > Workspaces focuses the Board pane and the open dropdown takes Esc; mux Agent|Agent: Settings > Daemons shows a notice and opens nothing
  #
  # ========================================
  Background: User Story
    As a fspec TUI user
    I want to see only key hints that match what the key really does
    So that pressing an advertised key always does what its hint says

  Scenario: Every single-key menu row's key does the same thing on the board
    Given the board is shown with a selected card
    When the user presses the key of each single-key menu row, in lower and in upper case
    Then each key emits the same action as its menu row
    And Ctrl+F opens nothing

  Scenario: The board help lists the menu keys exactly as the menu shows them
    When the user opens the board help
    Then it lists "D" with "FOUNDATION.md", "A" with "Attachments" and "b" with "New Bot"
    And every single-key menu row appears in it with the same key and label
    And it lists "u" with "Menu bar help"

  Scenario: A row's key runs that row while a board dropdown is open
    Given the board's Tools dropdown is open
    When the user presses "F"
    Then the Changed Files view is shown
    And no dropdown is open

  Scenario: A row's key runs that row while a mux bar dropdown is open
    Given the mux is shown and its Settings dropdown is open
    When the user presses "p"
    Then the Provider Settings view is shown

  Scenario: A row's key runs that row in the menu-bar help dialog
    Given the menu-bar help dialog is open over the board
    When the user presses "C"
    Then the menu-bar help dialog is closed
    And the Checkpoints view is shown

  Scenario: The Help row says it shows the board keybindings
    When the menu-bar help dialog is rendered
    Then it shows "? Help - Show the board keybindings"

  Scenario: The board footer names Exit for Esc and the chooser for Enter
    When the board is rendered at width 200 and at width 120
    Then each footer shows "ESC Exit" and not "ESC Back"
    And neither footer says "Work Agent"

  Scenario: Pressing a on a card without attachments says so
    Given the selected card TUI-001 has no attachments
    When the user presses "a"
    Then the notice "No attachments on TUI-001" is shown
    And no attachment picker opens

  Scenario: A card that cannot move says why
    Given the board has no workspace yet
    When the user presses "]"
    Then a notice says the card cannot move until the workspace is known
    Given the board shows workspace "/ws" and the backend refuses to move TUI-001
    When the user presses "["
    Then a notice names TUI-001 and the backend's error

  Scenario: End on an empty input jumps to the bottom of the scrollback
    Given an agent view with 200 rows of scrollback scrolled up five pages and an empty input
    When the user presses End
    Then the scrollback is at the bottom and sticks to it
    Given the input holds "abc" with the cursor at its start and the scrollback scrolled up
    When the user presses End
    Then the cursor is at the end of "abc" and the scrollback has not moved

  Scenario: The agent help describes Home, End, Alt+Enter and Esc as they behave
    When the agent help is opened
    Then it shows "Top / Bottom (empty input)" for Home and End
    And it shows "Newline (queues the draft while busy)" for Alt+Enter
    And its Esc row names stopping a run, clearing the draft and closing the agent
    And it does not show "ESC           Back"

  Scenario: A collapsed tool card names Tab and Enter
    When a tool card with more than 8 output lines is rendered collapsed
    Then it ends with "(Tab to select, Enter to view full)"

  Scenario: The empty-answer hint names what Esc does
    Given a question with no options is shown
    When the user presses Enter on an empty answer
    Then the hint says "press Esc to cancel"
    Given a question with options where the user chose Other
    When the user presses Enter on an empty answer
    Then the hint says "press Esc to go back"

  Scenario: Mux Daemons and Workspaces rows show their dropdown on the Board pane
    Given the mux shows Board and Agent with the Agent pane focused
    When the user runs Settings > Workspaces
    Then the Board pane is focused and the workspace dropdown is open
    And Esc closes the workspace dropdown
    Given the mux shows two Agent panes and no Board pane
    When the user runs Settings > Daemons
    Then a notice says the Board pane is needed
    And no daemon switcher is open
