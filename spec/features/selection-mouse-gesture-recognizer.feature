@done
@rust
@mouse-events
@text-selection
@tui
@COPY-003
Feature: Mouse gesture recognizer for selection (drag + long-press)
  """
  New module rust/fspec-tui/src/mouse/gesture.rs. Type SelectionRecognizer with an internal state enum (Idle, Pressed{cell, at}, Selecting). Public enum SelectionGesture { Begin(Cell), Extend(Cell), Commit, Cancel }. Reuses Cell from selection.rs (COPY-002).
  crossterm mapping: Down(Left) records Pressed{cell, at:now}; Drag(Left) transitions Pressed to Selecting and returns Begin(press_cell) on the first drag, else returns Extend(cell); Up(Left) returns Commit when Selecting else clears with no gesture. Non-left buttons and wheel return None.
  Long-press threshold const HOLD = Duration::from_millis(400). tick(now) fires Begin(press_cell) exactly once when state is Pressed and now minus at is greater-or-equal HOLD, transitioning to Selecting. Threshold constant documented and tunable. Mirrors the debounce-timer pattern in mouse/toggle.rs but for press-hold detection.
  Testing: feed a scripted sequence of MouseEvent plus Instant into on_mouse and tick, assert the exact Vec of SelectionGesture. Fake clock is a base Instant plus Duration offsets. No real time, no terminal. Consumer is COPY-006 which maps gestures onto the live Selection and clipboard write.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A left-button Down followed by one or more Drag events begins and extends a selection; the first Drag after Down starts it
  #   2. A left-button Down with no movement, held past the long-press threshold (~400ms), begins a selection anchored at the press cell
  #   3. A quick left click (Down then Up before the threshold and without dragging) is NOT a selection and produces no selection gesture
  #   4. A left-button Up while a selection is active emits Commit; Up with no active selection emits nothing
  #   5. The recognizer emits high-level gestures: Begin{row,col}, Extend{row,col}, Commit, Cancel — never raw mouse events
  #   6. The long-press threshold decision is made against injected timestamps so behaviour is deterministic and testable without real time
  #   7. Mouse wheel (ScrollUp/ScrollDown) events are ignored by the recognizer and left to bubble to scroll handling
  #   8. Yes. The recognizer exposes fn tick(&mut self, now: Instant) -> Option<SelectionGesture> that the App run loop calls on the existing 16ms render tick (App::run in app/events.rs already has a tick arm). While a button is held stationary, tick compares now against the recorded press time and emits Begin once REEnable threshold (~400ms) passes. Mouse events feed fn on_mouse(ev, now). Both paths take an injected 'now' so tests use a fake clock.
  #
  # EXAMPLES:
  #   1. User presses left button at (5,3) then drags to (5,8); recognizer emits Begin(5,3) then Extend(5,8)
  #   2. User presses at (5,3) and holds 500ms without moving; recognizer emits Begin(5,3) once the threshold passes
  #   3. User clicks: Down at (5,3) then Up at (5,3) after 100ms with no drag; recognizer emits no gesture (bubbles as a normal click)
  #   4. After a drag selection, user releases the button; recognizer emits Commit
  #   5. During an active selection a ScrollUp arrives; recognizer ignores it and does not emit a selection gesture
  #   6. Long-press then drag: Down at (5,3), 500ms hold emits Begin(5,3), then Drag to (7,2) emits Extend(7,2), then Up emits Commit
  #
  # QUESTIONS (ANSWERED):
  #   Q: Long-press with no mouse movement produces no crossterm event to trigger Begin. Should the recognizer expose a poll/tick(now) method the event loop calls (e.g. on the 16ms render tick) to fire Begin once the hold threshold passes, rather than relying on a mouse event?
  #   A: Yes. The recognizer exposes fn tick(&mut self, now: Instant) -> Option<SelectionGesture> that the App run loop calls on the existing 16ms render tick (App::run in app/events.rs already has a tick arm). While a button is held stationary, tick compares now against the recorded press time and emits Begin once REEnable threshold (~400ms) passes. Mouse events feed fn on_mouse(ev, now). Both paths take an injected 'now' so tests use a fake clock.
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to start a text selection by dragging or by pressing and holding the mouse over the transcript
    So that I can select text with an intuitive gesture without a plain click accidentally selecting anything

  Scenario: Drag begins and extends a selection
    Given a fresh selection recognizer
    When the left button is pressed at row 5 column 3 at time 0ms
    And the mouse is dragged to row 5 column 8 at time 30ms
    Then the recognizer emits Begin at row 5 column 3 then Extend at row 5 column 8

  Scenario: A stationary long-press begins a selection at the press cell
    Given a fresh selection recognizer
    When the left button is pressed at row 5 column 3 at time 0ms
    And the recognizer is ticked at time 500ms
    Then the recognizer emits Begin at row 5 column 3

  Scenario: A quick click produces no selection gesture
    Given a fresh selection recognizer
    When the left button is pressed at row 5 column 3 at time 0ms
    And the left button is released at row 5 column 3 at time 100ms with no drag in between
    Then the recognizer emits no gesture

  Scenario: Releasing an active selection commits it
    Given a recognizer with an active drag selection
    When the left button is released
    Then the recognizer emits Commit

  Scenario: Wheel events are ignored during an active selection
    Given a recognizer with an active drag selection
    When a mouse wheel scroll-up event arrives
    Then the recognizer emits no selection gesture

  Scenario: Long-press then drag begins, extends, and commits
    Given a fresh selection recognizer
    When the left button is pressed at row 5 column 3 at time 0ms
    And the recognizer is ticked at time 500ms
    And the mouse is dragged to row 7 column 2
    And the left button is released
    Then the recognizer emits Begin at row 5 column 3, then Extend at row 7 column 2, then Commit
