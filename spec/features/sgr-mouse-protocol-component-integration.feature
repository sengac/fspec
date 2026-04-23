@BUG-131
@tui
@mouse-events
@virtuallist
Feature: SGR mouse protocol component integration
  """
  VirtualList and other TUI components must use SGR mouse protocol for scroll and text selection.
  SGR events survive ink 6.8.0's CSI input parser intact, unlike X10 raw bytes.
  Text selection (TUI-078) uses button-down/release with SGR terminators M/m.
  """

  Background: 
    Given the TUI is running with ink 6.8.0

  Scenario: VirtualList scroll-up via SGR mouse event
    Given a VirtualList component is rendered in scroll mode
    When the user sends an SGR scroll-up event
    Then the VirtualList should scroll up by the configured scroll amount

  Scenario: VirtualList scroll-down via SGR mouse event
    Given a VirtualList component is rendered in scroll mode
    When the user sends an SGR scroll-down event
    Then the VirtualList should scroll down by the configured scroll amount

  Scenario: Text selection disables mouse tracking on button-down
    Given a VirtualList component has mouse tracking enabled
    When the user sends an SGR left-click press event with terminator "M"
    Then the component should disable mouse tracking to allow native text selection

  Scenario: Text selection re-enables mouse tracking on button-release
    Given a VirtualList component has mouse tracking disabled for text selection
    When the user sends an SGR left-click release event with terminator "m"
    Then the component should re-enable mouse tracking

  Scenario: Board view scroll in UnifiedBoardLayout
    Given the UnifiedBoardLayout is rendered with board columns
    When the user sends an SGR scroll-down event over a board column
    Then the board column should scroll down
