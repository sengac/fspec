@done
@interactive-cli
@cli
@board-visualization
@high
@tui
@interactive
@kanban
@layout
@ITF-004
Feature: Fix TUI Kanban column layout to match table style
  """
  Add scroll indicators (↑ ↓) based on scrollOffset and viewport bounds calculations
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Columns must be part of a unified table with box-drawing characters (┌┬┐ ├┼┤ └┴┘), not individual bordered boxes
  #   2. Column headers must show: STATUS (count) - Xpts format
  #   3. Work units within columns must show: typeIcon ID Xpt priorityIcon
  #   4. The table layout must adapt to terminal width like fspec board does
  #   5. Focused column must be highlighted with cyan color
  #   6. Selected work unit within focused column must be highlighted
  #   7. All panels (Git Stashes, Changed Files, Kanban columns, footer) should be integrated into a single unified table layout to avoid duplicate borders
  #   8. Footer should be integrated into the unified table layout
  #   9. Support scrolling through ALL items with Page Up/Down. Show up arrow indicator above first visible item if scrolled past start. Show down arrow indicator below last visible item if more items exist beyond viewport. No artificial limit.
  #
  # EXAMPLES:
  #   1. User opens TUI, sees table with ┌┬┐ top border connecting all columns
  #   2. User navigates right with arrow key, sees focused column header turn cyan while others stay gray
  #   3. User navigates down within a column, sees selected work unit highlighted with cyan background
  #   4. User resizes terminal, table columns adjust width proportionally like fspec board
  #   5. Column shows AUTH-001 3pt 🟡 format with type icon and priority indicator
  #   6. User has 20 work units in backlog, presses Page Down, sees viewport scroll showing items 11-20 with up arrow indicator at top
  #   7. User scrolls to middle of long column, sees both up arrow at top and down arrow at bottom indicating more items in both directions
  #   8. User presses Page Up at top of column, nothing happens (already at start, no up arrow shown)
  #   9. Git Stashes panel integrated as table row above Kanban columns with proper ├┼┤ junction characters
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the Git Stashes and Changed Files panels also use table layout, or keep their current bordered box style?
  #   A: true
  #
  #   Q: Should the footer with keyboard shortcuts also use table borders, or keep its current style?
  #   A: true
  #
  #   Q: Do you want the same column limit behavior (showing first 5 items + overflow count) as fspec board?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using the interactive TUI
    I want to view the Kanban board with a consistent table layout
    So that the interface is clean, familiar, and matches the fspec board command style

  Scenario: Display unified table layout with box-drawing characters
    Given I am using the interactive TUI
    When I open the Kanban board view
    Then I should see a unified table with box-drawing characters (┌┬┐ ├┼┤ └┴┘)
    And all columns should be connected with a continuous top border
    And Git Stashes panel should be integrated as a table row with ├┼┤ junction characters
    And Changed Files panel should be integrated as a table row with ├┼┤ junction characters
    And footer should be integrated at the bottom with table borders

  Scenario: Navigate between columns with arrow keys
    Given I am viewing the Kanban board
    And the backlog column is focused
    When I press the right arrow key
    Then the specifying column should be focused
    And the focused column header should be displayed in cyan color
    And all other column headers should be displayed in gray color

  Scenario: Navigate within column with arrow down
    Given I am viewing a column with multiple work units
    And the first work unit is selected
    When I press the arrow down key
    Then the selection should move down by 1 work unit
    And the newly selected work unit should be highlighted with cyan background

  Scenario: Scroll by page with Page Down key
    Given I am viewing a column with 30 work units
    And the viewport shows 10 items at a time
    And I am at the top of the column
    When I press Page Down
    Then the viewport should jump to show items 11-20
    And an up arrow indicator should appear above the first visible item
    And a down arrow indicator should appear below the last visible item

  Scenario: Scroll indicator when scrolled past start
    Given I am viewing a column with 20 work units
    And I have scrolled down past the first item
    When I view the column
    Then I should see an up arrow indicator above the first visible item
    And I should see a down arrow indicator below the last visible item if more items exist

  Scenario: No action when Page Up pressed at top
    Given I am viewing a column
    And I am at the top of the column
    When I press Page Up
    Then the viewport should not change
    And no up arrow indicator should be shown

  Scenario: Display work unit with type icon and priority
    Given I am viewing a column containing work unit AUTH-001
    And AUTH-001 has estimate of 3 points
    And AUTH-001 is a story type
    When I view the work unit in the column
    Then I should see "📖 AUTH-001 3pt 🟡" format
    And the type icon should indicate story type
    And the priority icon should reflect the estimate

  Scenario: Adapt column width to terminal size
    Given I am viewing the Kanban board
    When I resize the terminal window
    Then the table columns should adjust width proportionally
    And the layout should match the behavior of fspec board command

  Scenario: Column header shows status, count, and points
    Given I am viewing a column with 5 work units
    And the total estimate for the column is 15 points
    When I view the column header
    Then it should display "STATUS (5) - 15pts" format

  Scenario: Use direct stdout.columns for terminal width
    Given the Kanban board component is rendering
    When it needs to determine terminal width
    Then it should use useStdout hook from Ink
    And read stdout.columns directly (same pattern as BoardDisplay)
    And fallback to 80 if stdout.columns is undefined
    And NOT use a custom useTerminalSize hook with state

  Scenario: Column width calculation with useMemo dependency
    Given the component is using useStdout to get terminalWidth
    When terminalWidth changes from 100 to 140 columns
    Then colWidth should be wrapped in useMemo with terminalWidth dependency
    And colWidth should recalculate from 12 to 18 characters automatically
    And component should re-render with new column widths

  Scenario: Table borders stay aligned after terminal resize
    Given I am viewing the Kanban board at 120 columns wide
    And the table borders are properly aligned
    When I resize the terminal to 80 columns wide
    Then the table should instantly reflow with new column widths
    And all box-drawing characters should remain properly connected
    And top border should show continuous ┌─┬─┐ pattern
    And junction rows should show continuous ├─┼─┤ pattern
    And bottom border should show continuous └─┴─┘ pattern
