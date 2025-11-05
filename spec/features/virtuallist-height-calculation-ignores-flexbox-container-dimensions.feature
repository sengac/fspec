@ui-enhancement
@layout
@virtuallist
@tui
@high
@TUI-008
Feature: VirtualList height calculation ignores flexbox container dimensions
  """
  Architecture notes:
  - VirtualList currently uses terminal height for viewport calculation, ignoring flexbox-allocated container space
  - Ink's Yoga layout engine computes actual Box dimensions via measureElement()
  - Fix uses useLayoutEffect to measure container height after flexbox layout completes
  - Measured height takes priority over terminal height calculation (with fallback)
  - Re-measurement triggers on items.length or terminalHeight changes

  Dependencies:
  - Ink's measureElement API (accesses yogaNode.getComputedHeight())
  - React useLayoutEffect hook for post-layout measurement
  - useTerminalSize hook for fallback when measurement unavailable

  Critical implementation requirements:
  - MUST attach ref to VirtualList outer Box for measurement
  - MUST use measured height if available, fallback to terminal height
  - MUST re-measure when items change or terminal resizes
  - MUST handle empty state with same measurement logic
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. VirtualList MUST measure actual container height after flexbox layout using Ink's measureElement API
  #   2. VirtualList MUST use measured height (if available) with fallback to terminal height calculation
  #   3. VirtualList MUST re-measure container height when items change or terminal resizes
  #
  # EXAMPLES:
  #   1. CheckpointViewer with 100 checkpoints in 33% flexGrow container shows only items that fit, checkpoint heading remains visible
  #   2. FileDiffViewer with file list (33% width) and diff pane (67% width) both show correct item counts without overflow
  #   3. Terminal resizes from 80x24 to 120x40, VirtualLists re-measure and adjust item counts automatically
  #
  # ========================================
  Background: User Story
    As a developer using TUI with flexbox layouts
    I want to see correct item counts in VirtualList containers without overflow
    So that headings remain visible and layout respects flexbox space allocation

  Scenario: CheckpointViewer with many items shows correct count in flexbox container
    Given I have a CheckpointViewer with 100 checkpoints
    And the checkpoint list container has flexGrow=1 (33% vertical space)
    And the terminal height is 50 lines
    When VirtualList measures its container height after flexbox layout
    Then the checkpoint list should display approximately 15 items (33% of 50 lines)
    And the checkpoint heading "Checkpoints" should remain visible
    And no items should overflow the container boundaries

  Scenario: FileDiffViewer with dual panes shows correct counts without overflow
    Given I have a FileDiffViewer with file list and diff pane
    And the file list has flexGrow=1 (33% width)
    And the diff pane has flexGrow=2 (67% width)
    And there are 50 files and 200 diff lines
    When both VirtualLists measure their container dimensions
    Then the file list should show items that fit in 33% width allocation
    And the diff pane should show items that fit in 67% width allocation
    And both headings "Files" and "Diff" should remain visible
    And no overflow should occur in either pane

  Scenario: Terminal resize triggers VirtualList re-measurement
    Given I have a CheckpointViewer with VirtualLists rendered
    And the terminal is 80x24
    And VirtualLists have measured their initial container heights
    When the terminal resizes to 120x40
    Then all VirtualLists should re-measure their container heights
    And item counts should adjust automatically to new dimensions
    And no re-render glitches or flickering should occur
