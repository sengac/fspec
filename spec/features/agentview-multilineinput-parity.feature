@done
@agent-view
@input
@rpc
@RPC-095
Feature: AgentView MultiLineInput parity: spinner/busy, placeholder, blocking, and Esc cascade
  """
  Rust render loop is currently event-driven only — no idle ticks. Spinner needs an 80ms timer that fires only while is_loading || is_compacting.
  Existing pattern: scrollback_paint.rs (RPC-094 polish) wrote a manual painter rather than using ratatui's built-in widget. Reuse the same approach for spinner — write a single-line painter that takes (area, buf, frame_index, message, hint).
  InputGate { block_edits, suppress_enter } threads from agent.rs (state read) → multiline_input::handle_key. Esc/cursor moves bypass the gate (cf. TS MultiLineInput.tsx:294-296).
  Esc-cascade extension lives in app/dispatch_esc_cascade.rs — add input-clear branch BEFORE the BackToBoard fallback. Requires plumbing 'input_is_nonempty' through Action::AgentEscPressed or a store query.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. While a session is Running, the input row shows a braille-dot spinner cycling through ['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏'] every 80ms with the message 'Thinking... (Esc to stop)' dim-styled
  #   2. While a session is Compacting, the input row shows the same braille-dot spinner with the message 'Compacting... (Esc to stop)' dim-styled
  #   3. While Compacting, the input buffer is frozen: printable characters, Backspace, Delete, and forward-delete are swallowed without changing the buffer
  #   4. While Compacting, pressing Enter does not submit the input — the keystroke is swallowed
  #   5. During Running or Compacting, cursor moves (arrow keys), Shift-arrow navigation, and Esc still function normally
  #   6. Pressing Esc when Running or Compacting interrupts the session and stays on AgentView (existing behaviour preserved)
  #   7. Pressing Esc when idle with a non-empty input buffer (after trim) clears the buffer and stays on AgentView — does NOT navigate to Board
  #   8. Pressing Esc when idle with an empty input buffer navigates back to the Board (L7 exit-confirmation deferred to a follow-up card)
  #   9. The SessionHeader's is_loading flag is wired from session status (Running == is_loading), no longer hard-coded to false in agent.rs
  #   10. The idle placeholder string remains verbatim: "Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)"
  #   11. The dangling unused constant PLACEHOLDER_FOOTER_HINTS in agent.rs:72 is removed
  #   12. All new modules (spinner.rs, input_transition.rs) stay under the 300-LoC source-shape ceiling
  #   13. Defer L7 exit-confirmation to follow-up card. When idle + empty input, Esc → BackToBoard (current behaviour).
  #   14. Keep Rust's footer chip; do not duplicate compaction text into the input placeholder. The placeholder during compaction remains the idle Type-a-message hint (but input is gated).
  #   15. Wire is_loading from session status now. tokens_per_second stays None until RPC-086 lands — header tok/s chip remains hidden when None.
  #   16. Defer HITL placeholder/options UI to a separate HITL-dedicated card. Add the HITL_PLACEHOLDER constant only if it costs nothing.
  #   17. Spawn a tokio::time::interval(80ms) tick task while a session is Running or Compacting; emit Action::SpinnerTick. Cancel/stop when the session leaves those states. Pattern mirrors existing background tasks under app/.
  #
  # EXAMPLES:
  #   1. Session enters Running state with elapsed=0ms — input row paints '⠋ Thinking... (Esc to stop)' in dim style
  #   2. After 240ms of Running, spinner cell shows '⠸' (frame index 3) because 240/80 = 3
  #   3. Session enters Compacting state — input row paints '⠋ Compacting... (Esc to stop)' in dim style
  #   4. While Compacting, user presses 'a' then Backspace — buffer remains exactly as it was before Compacting started
  #   5. While Compacting, user presses Enter — no submit_action is dispatched; the keystroke is swallowed
  #   6. While Running, user presses Esc — Action::AgentEscPressed → backend.interrupt(session_id) is invoked; view stays on AgentView
  #   7. Session is Idle and input buffer is 'hello world'. User presses Esc — buffer becomes empty string; view stays on AgentView; NO BackToBoard action emitted
  #   8. Session is Idle and input buffer contains only whitespace '   '. User presses Esc — buffer cleared (trim non-empty short-circuited), view stays on AgentView. NOTE: TS uses inputValue.trim() !== '' so pure whitespace is treated as empty → BackToBoard
  #   9. Session is Idle and input buffer is empty. User presses Esc — Action::BackToBoard emitted (L7 exit-confirmation deferred)
  #   10. During Running, user presses Right arrow — cursor moves right in the buffer; spinner continues animating
  #   11. Session is Running and tokens_per_second is None — header shows no tok/s chip
  #   12. Session is Running and tokens_per_second is Some(42.5) — header shows magenta '42.5 tok/s' chip
  #   13. Source-shape test confirms views/agent/spinner.rs and views/agent/input_transition.rs are both under 300 LoC
  #   14. agent.rs no longer contains the constant PLACEHOLDER_FOOTER_HINTS
  #
  # QUESTIONS (ANSWERED):
  #   Q: The TS Esc cascade L7 shows an exit-confirmation dialog before navigating to Board. Should RPC-095 implement that dialog or defer to a follow-up card? Defaulting to: defer (jump straight to Board when input is empty).
  #   A: Defer L7 exit-confirmation to follow-up card. When idle + empty input, Esc → BackToBoard (current behaviour).
  #
  #   Q: TS shows compaction progress text inside the input placeholder. Rust currently shows it in the footer chip. Keep Rust footer chip OR add placeholder text too? Recommending: keep footer-only.
  #   A: Keep Rust's footer chip; do not duplicate compaction text into the input placeholder. The placeholder during compaction remains the idle Type-a-message hint (but input is gated).
  #
  #   Q: tokens_per_second source — is the value already available via session store, or does this depend on the unfinished RPC-086 (token tracking)? If the latter, scenario 11 is gated and we ship is_loading wiring only.
  #   A: Wire is_loading from session status now. tokens_per_second stays None until RPC-086 lands — header tok/s chip remains hidden when None.
  #
  #   Q: HITL placeholder 'Type your answer...' and HITL options UI — include here or defer to a HITL-dedicated card? Recommending: defer; touch the placeholder constant only if cheap.
  #   A: Defer HITL placeholder/options UI to a separate HITL-dedicated card. Add the HITL_PLACEHOLDER constant only if it costs nothing.
  #
  #   Q: Spinner animation drive — current Rust render loop is event-driven; need a periodic 80ms tick to advance frames. Acceptable to spawn a tokio interval that emits Action::SpinnerTick when a session is Running/Compacting, and cancels otherwise?
  #   A: Spawn a tokio::time::interval(80ms) tick task while a session is Running or Compacting; emit Action::SpinnerTick. Cancel/stop when the session leaves those states. Pattern mirrors existing background tasks under app/.
  #
  # ========================================
  Background: User Story
    As a fspec user running the Rust ratatui AgentView
    I want to see a spinner while the agent is thinking or compacting, have my input safely locked during compaction, and have a sensible Esc cascade that clears text before navigating away
    So that I get the same trustworthy busy/blocking/Esc behaviour the original TypeScript Ink TUI had and don't lose typed text or race the backend

  Scenario: Running session paints Thinking spinner on first frame
    Given I am viewing the AgentView for a session whose status has just become Running
    And the spinner elapsed-time counter is zero
    When the AgentView renders the input row
    Then the input row shows the text "⠋ Thinking... (Esc to stop)"
    And the entire line is rendered with the DIM style modifier

  Scenario: Running session advances spinner frame at 80ms cadence
    Given I am viewing the AgentView for a Running session
    When 240 milliseconds have elapsed since the spinner started
    Then the spinner glyph in the input row is "⠸"
    And the spinner message remains "Thinking... (Esc to stop)"

  Scenario: Compacting session paints Compacting spinner on first frame
    Given I am viewing the AgentView for a session whose status has just become Compacting
    And the spinner elapsed-time counter is zero
    When the AgentView renders the input row
    Then the input row shows the text "⠋ Compacting... (Esc to stop)"
    And the entire line is rendered with the DIM style modifier

  Scenario: Compacting blocks printable character insertion
    Given I am viewing the AgentView for a Compacting session
    And the input buffer contains the text "hello"
    When I press the printable character "a"
    Then the input buffer still contains exactly "hello"
    And no submit action is dispatched

  Scenario: Compacting blocks Backspace
    Given I am viewing the AgentView for a Compacting session
    And the input buffer contains the text "hello"
    When I press Backspace
    Then the input buffer still contains exactly "hello"

  Scenario: Compacting blocks Delete and forward-delete
    Given I am viewing the AgentView for a Compacting session
    And the input buffer contains the text "hello"
    And the cursor is at position 0
    When I press Delete
    Then the input buffer still contains exactly "hello"

  Scenario: Compacting swallows Enter so input is not submitted
    Given I am viewing the AgentView for a Compacting session
    And the input buffer contains the text "hello"
    When I press Enter
    Then no submit action is dispatched
    And the input buffer still contains exactly "hello"

  Scenario: Cursor movement still works during Running
    Given I am viewing the AgentView for a Running session
    And the input buffer contains the text "hello"
    And the cursor is at position 0
    When I press the Right arrow key
    Then the cursor moves to position 1
    And the spinner continues to animate

  Scenario: Idle placeholder text is verbatim
    Given I am viewing the AgentView for an Idle session
    And the input buffer is empty
    When the AgentView renders the input row
    Then the input row shows the placeholder text "Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)"

  Scenario: New modules stay under the 300-LoC source-shape ceiling
    Given codelet/fspec-tui/src/views/agent/spinner.rs exists
    And codelet/fspec-tui/src/views/agent/input_transition.rs exists
    When the source-shape test runs
    Then both files have fewer than 300 lines of code
