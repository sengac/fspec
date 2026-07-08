@done
@clipboard
@mouse-events
@integration
@rust
@text-selection
@tui
@COPY-011
Feature: A click does not clear the active text selection
  """
  Root cause: recognizer (gesture.rs) emits NO gesture for a quick click (Up from Pressed returns empty, never the Cancel variant), and no wiring site clears the prior selection on a bare click. All four apply_*_gestures handlers already have a Cancel arm that clears.
  Recommended fix: in gesture.rs on_mouse, a left Up while state==Pressed (quick click, no drag/long-press) emits SelectionGesture::Cancel and returns to Idle. This routes through the existing Cancel arms on all four surfaces to clear the selection+highlight. Update the recognizer unit test a_quick_click_produces_no_selection_gesture to expect [Cancel]. Do COPY-011 AFTER COPY-010. See spec/attachments/COPY-011/bug-analysis-click-clears-selection.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A quick click (left button Down then Up with no drag and no long-press) over an active selection clears the selection and its highlight and copies nothing
  #   2. Starting a new drag (Down then Drag then Up) still replaces any old selection with the new one and copies the new text
  #   3. A quick click when there is no active selection stays inert: nothing is selected and nothing is copied
  #   4. The click-to-clear behavior applies identically to all four surfaces: scrollback, input composer, turn-content modal, and board details strip
  #   5. Clearing via a click must not itself write to the clipboard, and must not disturb unrelated click behavior (e.g. board grid focus/select outside the details strip, or Esc-exit-confirmation)
  #
  # EXAMPLES:
  #   1. In the scrollback, the user has an active highlighted selection, then quickly clicks a line without dragging; the highlight disappears and the clipboard is unchanged
  #   2. In the input composer, the user has an active selection, then clicks once without dragging; the selection clears
  #   3. In the turn-content modal, the user has an active selection, then clicks once without dragging; the selection clears and nothing is copied
  #   4. In the board details strip, the user has an active selection, then clicks once inside the strip without dragging; the selection clears
  #   5. The user has an active selection, then presses and drags to select different text; the old selection is replaced and the new text is copied on release
  #   6. With no active selection, the user quickly clicks a line; nothing is selected and nothing is copied
  #
  # ========================================
  Background: User Story
    As a user who has made a text selection with the mouse
    I want to have a plain click clear my existing selection
    So that the stale highlight goes away like in any normal selection system when I click elsewhere without dragging

  Scenario: A quick click clears an active scrollback selection
    Given a scrollback with an active highlighted text selection and mouse capture enabled
    When I quickly click a line without dragging
    Then the scrollback selection is cleared and its highlight is removed
    And nothing new is written to the clipboard by the click

  Scenario: A quick click clears an active input composer selection
    Given an input composer with an active text selection
    When I quickly click without dragging
    Then the composer selection is cleared

  Scenario: A quick click clears an active turn-content modal selection
    Given an open turn-content modal with an active text selection
    When I quickly click in the modal body without dragging
    Then the modal selection is cleared
    And nothing new is written to the clipboard by the click

  Scenario: A quick click clears an active board details strip selection
    Given a board details strip with an active text selection
    When I quickly click inside the strip without dragging
    Then the strip selection is cleared

  Scenario: Starting a new drag replaces the old selection
    Given a scrollback with an active highlighted text selection and mouse capture enabled
    When I press and drag to select different text and release
    Then the old selection is replaced by the new one
    And the newly selected text is written to the clipboard

  Scenario: A quick click with no active selection stays inert
    Given a scrollback with no active text selection and mouse capture enabled
    When I quickly click a line without dragging
    Then nothing is selected
    And nothing is written to the clipboard
