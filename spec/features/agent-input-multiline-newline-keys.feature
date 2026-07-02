@done
@rust
@agent-view
@tui
@RPC-402
Feature: Shift+Enter newline unreachable in agent input — keyboard enhancement flags never enabled
  """
  Gate flag push on crossterm supports_keyboard_enhancement() called AFTER raw mode is enabled; store pushed=true process-wide; Pop in the same teardown path (incl. Drop/panic restore) BEFORE LeaveAlternateScreen; teardown is best-effort (each command attempted independently)
  Enter dispositions live in multiline_input_enter.rs handle_enter: plain Enter submits (suppress_enter-gated); ANY modifier-carrying Enter (Shift/Alt/Ctrl/combos) inserts a newline, gated by block_edits — this closes the accidental tui-textarea fallthrough completely
  KeyEventKind::Press filter enforced on the REAL dispatch path — dispatch.rs handle_event drops Release/Repeat key events before any branch (shortcuts, chords, input); multiline_input.rs handle_event_gated keeps the same filter as defense-in-depth for direct widget callers
  multiline_input.rs stays <300 LoC via the multiline_input_enter.rs / multiline_input_paste.rs submodules; dispatch.rs stays <300 LoC via dispatch_popups.rs; terminal.rs changes must remain best-effort (never fail init if push fails)
  INPUT_PLACEHOLDER_HINT (views/agent.rs) leads with the 'Shift+Enter' newline hint so it survives 80-column truncation; help_dialog for_agent() documents Shift+Enter/Alt+Enter newline
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Terminal init pushes kitty keyboard enhancement flags (DISAMBIGUATE_ESCAPE_CODES) only when the terminal reports support, and pops them on all teardown paths
  #   2. Plain Enter (no modifiers) always submits the buffer; Shift+Enter and Alt+Enter insert a newline at the cursor
  #   3. Only KeyEventKind::Press key events are processed by the input; Release/Repeat events delivered under the enhancement protocol are ignored
  #   4. Newline insertion grows the input area by one row per logical line up to 6 visible rows
  #   5. While the session is Compacting (block_edits/suppress_enter gate), Shift+Enter and Alt+Enter must not modify the buffer
  #
  # EXAMPLES:
  #   1. User on kitty/WezTerm presses Shift+Enter mid-word: line splits at cursor, cursor moves to start of new line, input grows to 2 rows
  #   2. User on a legacy xterm presses Alt+Enter (delivered as ESC CR → Enter+ALT): a newline is inserted, not a submit
  #   3. User types 3 lines via Shift+Enter then presses plain Enter: the full 3-line buffer is submitted joined by \n and the input resets to 1 row
  #   4. A Shift+Enter KeyEvent with kind=Release arrives: it is ignored, no newline inserted
  #   5. Session is Compacting: Shift+Enter is swallowed (Continued), buffer unchanged
  #   6. TerminalGuard init on a terminal without keyboard-enhancement support: flags are not pushed, teardown does not pop, app works as before
  #   7. TerminalGuard init on a supporting terminal: flags pushed after raw mode; on quit, PopKeyboardEnhancementFlags issued before leaving alternate screen
  #
  # ========================================
  Background: User Story
    As a TUI user composing a message in the agent view
    I want to insert newlines with Shift+Enter (or Alt+Enter on legacy terminals) while Enter still submits
    So that I can compose multi-line messages instead of being limited to a single line

  Scenario: Shift+Enter mid-word splits the line and grows the input to 2 rows
    Given the agent input contains "hello world" with the cursor between "hello " and "world"
    When I press Shift+Enter
    Then the input buffer contains "hello " on the first line and "world" on the second line
    And the cursor is at the start of the second line
    And the input area reports 2 visible rows

  Scenario: Alt+Enter inserts a newline instead of submitting
    Given the agent input contains "first line" with the cursor at the end
    When I press Alt+Enter
    Then a newline is inserted at the cursor
    And the buffer is not submitted
    And the input area reports 2 visible rows

  Scenario: Plain Enter submits the full multi-line buffer and resets the input
    Given the agent input contains 3 lines composed with Shift+Enter
    When I press plain Enter with no modifiers
    Then the submitted value is the 3 lines joined by newline characters
    And the input buffer is empty
    And the input area reports 1 visible row

  Scenario: Key release events are ignored by the input
    Given the agent input contains "draft"
    When a Shift+Enter key event with kind Release arrives
    Then no newline is inserted
    And the input buffer still contains "draft"

  Scenario: Shift+Enter is swallowed while the session is compacting
    Given the agent input contains "draft" and the compacting edit gate is active
    When I press Shift+Enter
    Then the key is consumed without submitting
    And the input buffer still contains "draft"
    And the input area reports 1 visible row
