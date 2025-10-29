@ui-refinement
@tui
@git
@flexbox
@layout
@GIT-006
Feature: Pure flexbox layout for checkpoint and changed files viewers
  """
  VirtualList uses useTerminalSize hook to dynamically calculate visible height. CheckpointViewer and ChangedFilesViewer use Ink's flexbox model (flexGrow, flexShrink, minWidth) for responsive layout. ChangedFilesViewer uses getFileDiff() from src/git/diff.ts for real git diffs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. No percentage-based widths or heights (e.g., no width='30%', no height='100%')
  #   2. Use flexGrow exclusively for dynamic sizing
  #   3. Fixed widths must be character-based minimums only (e.g., 30 characters for file list)
  #   4. VirtualList height prop must be optional, calculate from useTerminalSize if not provided
  #   5. ChangedFilesViewer must load actual git diffs using getFileDiff function (not mock data)
  #   6. Show loading state while diff is being fetched to prevent flickering
  #   7. No, VirtualList should NOT accept height prop at all. It must be pure flexbox - use flexGrow to fill container completely and calculate visible items from useTerminalSize internally.
  #   8. File list should use flexbox proportional sizing (e.g., flexBasis with flexGrow/flexShrink) to take approximately 1/4 of container width, with 30 characters as minimum. Use minWidth for character minimum.
  #   9. CheckpointViewer diff view must also load REAL git diffs from checkpoint stash data, not placeholder text.
  #
  # EXAMPLES:
  #   1. VirtualList with 100 items in 80x24 terminal: renders with flexGrow=1, calculates visibleHeight from terminalHeight, shows only items that fit
  #   2. CheckpointViewer in 120 column terminal: file list takes ~30 columns (1/4 with minWidth=30), diff pane takes remaining ~90 columns with flexGrow=1
  #   3. ChangedFilesViewer selects 'src/auth.ts': shows 'Loading diff...' immediately, then displays real git diff with +/- lines from getFileDiff()
  #   4. Terminal resizes from 80x24 to 120x40: all components adjust automatically using flexbox, no re-render glitches
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should VirtualList still accept an explicit height prop for cases where the parent wants to control it?
  #   A: true
  #
  #   Q: Should the file list width be exactly 30 characters, or is that just a minimum with flexShrink?
  #   A: true
  #
  #   Q: Does the diff view in CheckpointViewer also need real git diff loading, or can it stay as placeholder for now?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using checkpoint/changed files viewers
    I want to have all layout use pure flexbox without any hardcoded dimensions
    So that views resize properly and work correctly on any terminal size

  Scenario: VirtualList uses flexbox without hardcoded heights
    Given VirtualList has 100 items
    And the terminal is 80x24
    When VirtualList renders
    Then it should use flexGrow=1 to fill container
    And it should calculate visibleHeight from useTerminalSize internally
    And it should display only items that fit in available height
    And it should NOT accept a height prop

  Scenario: CheckpointViewer uses flexbox proportional sizing
    Given CheckpointViewer is rendered in 120 column terminal
    When the layout renders
    Then the file list should take approximately 1/4 of container width
    And the file list should have minWidth of 30 characters
    And the diff pane should use flexGrow=1 to fill remaining space
    And NO percentage-based widths should be used

  Scenario: ChangedFilesViewer loads real git diffs
    Given ChangedFilesViewer has selected file 'src/auth.ts'
    When the diff pane renders
    Then it should show 'Loading diff...' immediately
    And it should call getFileDiff() to fetch real git diff
    And it should display actual +/- diff lines
    And it should NOT show placeholder or mock data

  Scenario: Terminal resize causes automatic layout adjustment
    Given any viewer component is rendered in 80x24 terminal
    When terminal resizes to 120x40
    Then all components should adjust automatically using flexbox
    And there should be NO re-render glitches or flickering
    And proportional sizing should recalculate correctly
