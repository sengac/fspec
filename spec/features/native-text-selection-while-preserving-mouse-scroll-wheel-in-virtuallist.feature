@TUI-078
Feature: Native text selection while preserving mouse scroll wheel in VirtualList
  """
  Architecture notes:
  - Timer ref (reEnableMouseRef) must be a MutableRefObject<ReturnType<typeof setTimeout> | null> to track pending re-enable timeout
  - File: VirtualList.tsx — only the main conversation VirtualList needs this change; AgentView and BoardView have their own ?1000h instances for modal interaction that must remain unchanged
  - Button byte detection via input.charCodeAt(2): 32=left, 33=middle, 34=right (clicks), 35=release, 96=scroll up, 97=scroll down (scrolls)
  - X10 mouse protocol escape sequence format: ESC [ M <btn> <x> <y> where <btn> is at charCodeAt(2)
  - The 5000ms timeout is a generous window for selection/copy operations as a debounce fallback
  - Button release event (byte 35) immediately re-enables mouse tracking, so users can scroll right after finishing selection
  - Each button-down event restarts the debounce timer, extending the selection mode window while the user is still clicking/selecting
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When ?1000h (X10 mouse tracking) is active, the terminal hands all mouse button events to the app — this prevents native terminal text selection entirely
  #   2. Mouse button-down events (buttonByte 32–34), button-release events (35), and scroll wheel events (96–99) are distinguishable in the ?1000h raw escape sequence at charCodeAt(2)
  #   3. When a button-down event (32–34) arrives, ?1000l must be written to stdout immediately so the terminal regains control for native click-and-drag selection
  #   4. After button-down, ?1000h is re-enabled either when button-release (35) arrives OR after a 5000ms debounce timeout, whichever comes first
  #   5. Each button-down event restarts the debounce timer (extends selection mode while user is actively clicking/dragging)
  #   6. When button-release (35) arrives, mouse tracking is immediately re-enabled and any pending timer is cancelled
  #   7. On VirtualList unmount or when isFocused becomes false, any pending re-enable timer must be cleared before ?1000l is written
  #   8. Scroll wheel events (96–99) are always handled normally and never trigger ?1000l — the 5000ms window only applies to button-down events
  #
  # EXAMPLES:
  #   1. Button byte 96 = scroll up, 97 = scroll down — both should be handled normally without ?1000l
  #   2. User scrolls with mouse wheel → conversation scrolls, no disable/re-enable cycle occurs
  #   3. User clicks and drags text → mouse tracking is disabled, terminal handles native text selection
  #   4. When user releases mouse button, mouse tracking is immediately re-enabled → scroll wheel works right away
  #   5. If button-release event is missed, the 5000ms debounce timeout ensures scroll wheel eventually works again
  #   6. User clicks repeatedly → timer is reset each time, mouse tracking stays disabled while actively selecting
  #   7. User navigates away from conversation view while timer is pending → timer is cleared, mouse tracking disabled cleanly
  #
  # ========================================
  Background: User Story
    As a TUI user reading AI output
    I want to click and drag to select text in the conversation view
    So that copy AI responses, code snippets, or any TUI content to the clipboard using my terminal's native Ctrl+C

  @happy-path
  Scenario: User scrolls with mouse wheel in conversation view
    Given the TUI is showing the conversation view with AI output
    And mouse tracking is enabled (?1000h)
    When the user scrolls the mouse wheel up or down
    Then the conversation content should scroll in the corresponding direction
    And mouse tracking should remain enabled throughout

  @happy-path
  Scenario: User clicks and drags to select text
    Given the TUI is showing the conversation view with AI output
    And mouse tracking is enabled (?1000h)
    When the user clicks and drags to select text
    Then mouse tracking should be temporarily disabled (?1000l)
    And the terminal should handle the text selection natively
    And the user should be able to copy the selected text with Ctrl+C

  @timer
  Scenario: Scroll wheel works again after selection timeout (fallback)
    Given the TUI is showing the conversation view
    And the user has just finished clicking/dragging to select text
    And mouse tracking was temporarily disabled
    And the button-release event was not captured
    When 5 seconds have passed since the last click
    Then mouse tracking should be re-enabled (?1000h)
    And the user should be able to scroll with the mouse wheel again

  @timer
  @button-release
  Scenario: Button release immediately re-enables mouse tracking
    Given the TUI is showing the conversation view
    And the user has clicked to select text
    And mouse tracking was temporarily disabled
    When the user releases the mouse button
    Then mouse tracking should be immediately re-enabled (?1000h)
    And any pending timer should be cleared
    And the user should be able to scroll with the mouse wheel right away

  @timer
  @debounce
  Scenario: Rapid clicks extend selection mode (debounce behavior)
    Given the TUI is showing the conversation view
    And mouse tracking is enabled (?1000h)
    When the user clicks once (first click)
    And 3 seconds later clicks again (second click)
    Then the re-enable timer should be reset
    And mouse tracking should stay disabled for 5 seconds from the second click
    And mouse tracking should not be re-enabled at 5 seconds from the first click

  @cleanup
  Scenario: Timer cleaned up when navigating away
    Given the TUI is showing the conversation view
    And the user has clicked to select text (triggering the disable timer)
    And the re-enable timer is pending
    When the user navigates away from the conversation view
    Then the pending re-enable timer should be cleared
    And mouse tracking should be cleanly disabled (?1000l)
