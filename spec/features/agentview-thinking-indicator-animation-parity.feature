@done
@RPC-093
@tui-component
@agent-view
@tui
@rust
Feature: AgentView Thinking indicator animation parity
  """
  TS source of truth: src/tui/components/ThinkingIndicator.tsx (80ms setInterval); src/tui/components/InputTransition.tsx phase machine ('loading'|'paused'|'hiding'|'showing'|'complete') with constants INK_FRAME_TIME_MS=17, CHARS_PER_FRAME=5, ANIMATION_PHASE_DELAY_MS=34. Rust touch points: app/events.rs (drop should_render gate while busy or animating; suppress frame.set_cursor_position when non-Idle), views/agent/input_transition.rs (add Hiding/Showing variants + paint), views/agent.rs (drive transitions in render_with_store). Out of scope: markdown in thinking blocks, HITL placeholder, Action::SpinnerTick (superseded by always-redraw-while-busy).
  """

  Background: User Story
    As a fspec TUI user watching the agent think and finish
    I want to see a smooth braille spinner, a graceful sweep-out/sweep-in finish animation when the agent stops, and no stray cursor block while the spinner is showing
    So that the experience matches the TypeScript Ink reference and feels polished instead of stuttery

  Scenario: Spinner advances at 80ms cadence even when no stream chunks arrive
    Given an AgentView with session s-1 in SessionStatus::Running and spinner_started_at set to the test clock origin
    When the App run loop advances the tokio clock to 80ms, 160ms, 240ms, ..., 960ms in 80ms steps
    Then a render is performed at each tick because the session is busy (should_render gate is bypassed while Running or Compacting)
    And the cell at the input row column 0 cycles through the symbols "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠋", "⠙" in that order across the twelve captured frames

  Scenario: Render loop redraws while busy without waiting on stream chunks
    Given an AgentView with session s-1 in SessionStatus::Running
    When a 16ms RENDER_TICK fires
    Then the terminal is drawn (terminal.draw is called once for that tick)
    And the App should_render flag is false

  Scenario: Render loop stays idle when nothing is busy and no events are pending
    Given an AgentView with session s-1 in SessionStatus::Idle
    When a 16ms RENDER_TICK fires
    Then the terminal is NOT drawn for that tick (terminal.draw is not called)
    And no input-transition finish animation is in progress (phase is Idle)
    And the App should_render flag is false

  Scenario: Render loop continues to tick while the finish animation is mid-Hiding even after session is Idle
    Given an AgentView with session s-1 in SessionStatus::Idle and the InputTransitionState is Hiding with visible_chars=28
    When a 16ms RENDER_TICK fires and App should_render is false
    Then the terminal IS drawn for that tick (terminal.draw is called once)
    And tick_should_draw(false, is_busy=false, is_animating=true) returns true

  Scenario: Render loop continues to tick while the finish animation is mid-Showing even after session is Idle
    Given an AgentView with session s-1 in SessionStatus::Idle and the InputTransitionState is Showing with visible_chars=5
    When a 16ms RENDER_TICK fires and App should_render is false
    Then the terminal IS drawn for that tick (terminal.draw is called once)
    And tick_should_draw(false, is_busy=false, is_animating=true) returns true

  Scenario: Terminal cursor is suppressed while session is Running
    Given an AgentView with session s-1 in SessionStatus::Running rendered into an 80x24 buffer
    When the App performs a render frame
    Then AgentView::is_cursor_visible returns false
    And frame.set_cursor_position is NOT called for that frame

  Scenario: Terminal cursor is suppressed while session is Compacting
    Given an AgentView with session s-1 in SessionStatus::Compacting rendered into an 80x24 buffer
    When the App performs a render frame
    Then AgentView::is_cursor_visible returns false
    And frame.set_cursor_position is NOT called for that frame

  Scenario: Terminal cursor is visible when session is Idle and MultiLineInput is mounted
    Given an AgentView with session s-1 in SessionStatus::Idle and the input-transition phase is Idle
    When the App performs a render frame
    Then AgentView::is_cursor_visible returns true
    And frame.set_cursor_position IS called with the input-area-relative cursor (x, y)

  Scenario: Busy-to-idle transition captures the spinner text and enters Hiding phase
    Given an AgentView whose session has been in SessionStatus::Running for 240ms and the spinner is painting "⠸ Thinking... (Esc to stop)"
    When the session transitions to SessionStatus::Idle at test clock t=240ms
    Then the InputTransitionState becomes Hiding with captured text "⠸ Thinking... (Esc to stop)" and visible_chars equal to the captured text length
    And started_at is set to the current test clock value

  Scenario: Hiding phase advances at 5 chars per 17ms frame and renders captured prefix
    Given an AgentView in InputTransitionState::Hiding with captured "⠸ Thinking... (Esc to stop)" (28 chars) and visible_chars 28 at started_at t0
    When the App run loop advances the test clock to t0+17ms, t0+34ms, t0+51ms, t0+68ms, t0+85ms, t0+102ms
    Then visible_chars at each step equals 23, 18, 13, 8, 3, 0 respectively
    And the rendered input row at each step is the captured text sliced 0..visible_chars, dim-styled, with no other glyphs to its right

  Scenario: Hiding holds for 34ms after visible_chars hits zero before entering Showing
    Given an AgentView in InputTransitionState::Hiding that just reached visible_chars=0 at test clock t1 (hide_completed_at=t1)
    When the App advances the test clock to t1+33ms
    Then the InputTransitionState remains Hiding with visible_chars=0
    When the App advances the test clock to t1+34ms
    Then the InputTransitionState becomes Showing with visible_chars=0 and started_at=t1+34ms and the placeholder string equal to AgentView's INPUT_PLACEHOLDER_HINT

  Scenario: Showing reveals placeholder at 5 chars per 17ms frame then enters Idle
    Given an AgentView in InputTransitionState::Showing with placeholder "Type a message" (14 chars) and visible_chars 0 at started_at t2
    When the App run loop advances the test clock to t2+17ms, t2+34ms, t2+51ms
    Then visible_chars at each step equals 5, 10, 14 respectively (clamped to placeholder length)
    And the rendered input row at each step is the placeholder sliced 0..visible_chars in DarkGray, with no other glyphs to its right
    And after the frame that reaches placeholder length the InputTransitionState transitions to Idle and MultiLineInput is mounted on the next render

  Scenario: Cursor stays suppressed during Hiding and Showing finish phases
    Given an AgentView whose InputTransitionState is Hiding
    When the App performs a render frame
    Then AgentView::is_cursor_visible returns false
    Given the InputTransitionState transitions to Showing
    When the App performs a render frame
    Then AgentView::is_cursor_visible returns false

  Scenario: Running state in the middle of a finish animation aborts the animation and resumes the spinner
    Given an AgentView in InputTransitionState::Hiding with visible_chars=13 at test clock t3
    When the session transitions back to SessionStatus::Running at test clock t3
    Then the InputTransitionState becomes Loading with elapsed_ms=0 and spinner_started_at=t3
    And the next render paints "⠋ Thinking... (Esc to stop)" starting at frame 0

  Scenario: Printable keystroke during Hiding short-circuits to Idle and enters the buffer
    Given an AgentView in InputTransitionState::Hiding with visible_chars=8
    When the user presses the printable key "h"
    Then the InputTransitionState becomes Idle
    And MultiLineInput is mounted and contains the buffer "h" with the cursor positioned after it

  Scenario: Printable keystroke during Showing short-circuits to Idle and enters the buffer
    Given an AgentView in InputTransitionState::Showing with visible_chars=10
    When the user presses the printable key "x"
    Then the InputTransitionState becomes Idle
    And MultiLineInput is mounted and contains the buffer "x" with the cursor positioned after it

  Scenario: Source-shape ceiling stays under 300 LoC for input_transition.rs
    Given the source file rust/fspec-tui/src/views/agent/input_transition.rs after the animation state machine has been implemented
    When the source-shape test reads the file's line count
    Then the line count is strictly less than 300
