@done
@visual-design
@high
@tui
@ui
@BOARD-019
Feature: Add fspec logo to TUI header

  """
  HYBRID APPROACH: Combines table-style borders (Text with box-drawing chars) with component-based content (Ink Box components). Uses buildBorderRow() for table borders (┌┬┐ ├┼┤ └┴┘), keeps border Text rows, replaces content rows with Box components using flexbox layout. Logo component uses Box width=11 with 3 Text lines for ASCII art. Content components (GitStashesPanel, ChangedFilesPanel, StatusColumn, WorkUnitCard) return Box with NO borders - borders added by UnifiedBoardLayout. Preserves existing state management, scroll handling, shimmer animation, and keyboard input. Removes logo injection hack and centerText utility.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Logo MUST be ASCII art positioned in top-left corner of TUI
  #   2. Logo container MUST be separate from Git Stashes/Changed Files container
  #   3. Git Stashes and Changed Files panels MUST be positioned to the right of the logo
  #   4. Logo format: 3 lines of ASCII art: ┏┓┏┓┏┓┏┓┏┓ / ┣ ┗┓┃┃┣ ┃ / ┻ ┗┛┣┛┗┛┗┛
  #   5. Component structure: Logo (left fixed width) | GitPanels (right flex) | Board (below, full width)
  #   6. UnifiedBoardLayout MUST keep state management: scroll offsets per column, shimmer animation state, terminal dimensions
  #   7. UnifiedBoardLayout MUST keep computed values: column width calculation, viewport height, work unit grouping by status, last changed work unit
  #   8. UnifiedBoardLayout MUST keep effects: auto-scroll to keep selection visible (lines 281-350), shimmer animation interval (lines 258-277)
  #   9. UnifiedBoardLayout MUST keep keyboard input handling: arrow keys, Page Up/Down, Enter, bracket keys (lines 353-402)
  #   10. HYBRID APPROACH: KEEP buildBorderRow() (lines 168-183) for table borders, KEEP border Text rows, REPLACE content rows with Box components
  #   11. KEEP fitToWidth() (lines 94-99) for border alignment, REMOVE centerText() and logo injection hack (lines 593-615)
  #   12. Content components (GitStashesPanel, ChangedFilesPanel, WorkUnitDetailsPanel, StatusColumn, WorkUnitCard, FooterPanel) return Box or Text with NO borders - borders added by UnifiedBoardLayout
  #
  # EXAMPLES:
  #   1. TUI displays logo in left container (3 lines tall), Git Stashes panel in right container beside it
  #   2. Logo takes up minimal width (approx 11 characters), leaving maximum space for git panels
  #   3. TUI layout: [Logo Container | Git Info Container] where Git Info contains stashes + changed files vertically stacked
  #   4. TUI uses flexDirection='row' for Logo|GitPanels horizontal layout, flexDirection='column' for vertical stacking
  #   5. Logo component: Box with width=11, contains 3 Text elements (logo lines)
  #   6. WorkUnitCards: Individual Box components per work unit, not pre-rendered text strings
  #   7. Terminal width handling: Use <Box width={terminalWidth}> as root container, Ink handles responsive layout automatically
  #   8. StatusColumn receives props: status (string), units (WorkUnit[]), scrollOffset (number), focusedColumnIndex, selectedWorkUnitIndex, shimmerPosition, lastChangedWorkUnit
  #   9. WorkUnitCard receives props: workUnit (WorkUnit), isSelected (boolean), isLastChanged (boolean), shimmerPosition (number) for character-by-character animation
  #   10. GitStashesPanel receives props: stashes (array), displays first 3 stashes with checkpoint name and relative time (e.g., '2 days ago')
  #   11. ChangedFilesPanel receives props: stagedFiles (string[]), unstagedFiles (string[]), displays first 3 files with '+' for staged and 'M' for unstaged
  #   12. WorkUnitDetailsPanel receives props: selectedWorkUnit (WorkUnit | null), displays title, description (first line), metadata (epic, estimate, status)
  #   13. UnifiedBoardLayout renders hybrid: Text rows for borders (┌─┬─┐, ├─┼─┤, └─┴─┘), Box components for content between borders
  #   14. Content components render content ONLY with │ characters for side borders (if needed), no top/bottom borders - those are rendered by UnifiedBoardLayout as separate Text rows
  #   15. Column width calculation (lines 59-67) MUST be kept - used by buildBorderRow() for correct table border alignment and passed to StatusColumn for content width
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should StatusColumn manage its own scroll state internally, or should scroll state be kept in UnifiedBoardLayout and passed down as props?
  #   A: true
  #
  #   Q: Should shimmer animation continue using character-by-character chalk colors, or use Ink color props?
  #   A: true
  #
  #   Q: Should column width be calculated and passed to StatusColumn props, or let Ink flex={1} distribute space automatically?
  #   A: true
  #
  #   Q: Should work unit selection state (focusedColumnIndex + selectedWorkUnitIndex) remain in UnifiedBoardLayout, or pass isSelected boolean to each WorkUnitCard?
  #   A: true
  #
  #   Q: Should we keep the existing buildBorderRow() system and render borders as Text lines, with Box components BETWEEN the border rows? Or do you want a completely different approach for borders with flexbox?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to see the fspec logo in the TUI header
    So that I have clear visual branding and better understand which tool I'm using

  Scenario: Logo displays in header with correct ASCII art
    Given I have a fspec project with work units
    When I launch the fspec TUI
    Then the logo line 1 should display "┏┓┏┓┏┓┏┓┏┓"
    And the logo line 2 should display "┣ ┗┓┃┃┣ ┃"
    And the logo line 3 should display "┻ ┗┛┣┛┗┛┗┛"


  Scenario: TUI uses hybrid rendering with table borders and Box content
    Given I have a fspec project with work units
    When I launch the fspec TUI
    Then table borders should be rendered as Text rows with box-drawing characters
    And content between borders should use Ink Box components
    And the Logo component should be a Box with width 11

