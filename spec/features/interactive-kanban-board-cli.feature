@cli
@feature-management
@high
@tui
@interactive-cli
@ink
@typescript
@BOARD-002
Feature: Interactive Kanban board CLI
  """
  Architecture:
  - Uses Ink + React for terminal UI rendering with cage-style component architecture
  - Components: BoardView (manages focus + detail view), KanbanBoard (renders 7 columns), KanbanColumn (VirtualList for performance), WorkUnitCard (type icons, estimates, priority)
  - Keyboard navigation via Input component (arrow keys + vim style hjkl)
  - Data loaded from Zustand fspecStore.loadData() on mount
  - ViewManager handles navigation to detail view on Enter
  - Detail view: When Enter pressed on board panel, stores selectedWorkUnit in state and switches to detail viewMode
  - Detail rendering: Displays work unit metadata (ID, title, type, status, estimate, epic) + full description + optional sections (rules, examples, questions, attachments)
  - ESC key: Returns from detail view to board, clears selectedWorkUnit state
  - No search/filter in Phase 1 - just display and navigate
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Board must display 7 Kanban columns: backlog, specifying, testing, implementing, validating, done, blocked
  #   2. Arrow keys (←→ or hjkl) navigate between columns, ↑↓ (or jk) navigate work units within column
  #   3. Enter key opens detailed view of selected work unit
  #   4. Work unit cards show: type icon (📖/🐛/⚙️), ID, estimate, priority indicator (🔴🟡🟢), truncated title
  #   5. Selected work unit highlighted with cyan background, focused column has cyan border
  #   6. Board loads work units from Zustand store (fspecStore.loadData() on mount)
  #   7. Empty columns display 'No work units' message
  #   8. Column headers show: status name, count, total story points
  #   9. ESC key returns to previous view or exits board
  #   10. Detail view must display: work unit ID, title, description, type, status, estimate, epic, rules, examples, questions, attachments, dependencies, and 'Press ESC to return' message
  #
  # EXAMPLES:
  #   1. User presses → arrow - focus moves from BACKLOG to SPECIFYING column, first work unit in SPECIFYING is selected
  #   2. User presses ↓ arrow - selection moves from RES-003 to RES-004 within BACKLOG column
  #   3. User presses Enter on selected work unit → Detail view opens showing: ID, title, type, status, description, rules (if any), examples (if any), questions (if any), attachments (if any), 'Press ESC to return'
  #   4. BACKLOG column header shows: 'BACKLOG (26) - 78pts'
  #   5. Work unit ITF-001 (story, 5pt estimate) displays as: 📖 ITF-001 5pt 🟡
  #   6. Empty TESTING column shows 'No work units' message centered in column
  #   7. Board loads on mount by calling fspecStore.loadData() and displaying work units grouped by status
  #   8. Footer displays: '← → Columns | ↑↓ jk Work Units | ↵ Details | ESC Back'
  #   9. User presses Enter on EST-002 (in SPECIFYING column) → Detail view shows: 'EST-002 - AI token usage tracking', 'Type: story', 'Status: specifying', 'Description:', full description text, 'Press ESC to return'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should column navigation wrap around (pressing → on BLOCKED goes to BACKLOG)?
  #   A: true
  #
  #   Q: Should work unit navigation within column wrap around (pressing ↓ on last item goes to first)?
  #   A: true
  #
  #   Q: What happens when navigating to an empty column - skip it or allow focus with no selection?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer managing work units
    I want to navigate and interact with Kanban board using keyboard shortcuts
    So that I can efficiently view, update, and manage work units without leaving the terminal

  Scenario: Navigate to next column with right arrow
    Given the board is displaying with BACKLOG column focused
    And there are work units in SPECIFYING column
    When the user presses the → arrow key
    Then focus should move to SPECIFYING column
    And the first work unit in SPECIFYING should be selected

  Scenario: Navigate down within column
    Given the board is displaying with BACKLOG column focused
    And RES-003 is selected
    And RES-004 is the next work unit in the column
    When the user presses the ↓ arrow key
    Then the selection should move to RES-004

  Scenario: Open detail view with Enter key
    Given the board is displaying with a work unit selected
    When the user presses the Enter key
    Then the detail view should open
    And the detail view should show work unit ID in format "PREFIX-NNN"
    And the detail view should show work unit title
    And the detail view should show type "story"
    And the detail view should show status
    And the detail view should show "Description:" section
    And the detail view should show "Press ESC to return" message

  Scenario: Display column header with count and points
    Given the board is displaying
    And the BACKLOG column has 26 work units
    And the total points in BACKLOG is 78
    When viewing the BACKLOG column header
    Then it should display "BACKLOG (26) - 78pts"

  Scenario: Display work unit card with type icon and estimate
    Given the board is displaying
    And work unit ITF-001 is a story with 5 point estimate
    When viewing the ITF-001 card
    Then it should display type icon 📖
    And it should display "ITF-001"
    And it should display "5pt"
    And it should display priority indicator 🟡

  Scenario: Display empty column message
    Given the board is displaying
    And the TESTING column has no work units
    When viewing the TESTING column
    Then it should display "No work units" message

  Scenario: Load board data on mount
    Given the fspecStore has work units data
    When the board component mounts
    Then it should call fspecStore.loadData()
    And it should display work units grouped by status
    And it should render 7 columns

  Scenario: Display footer with keyboard shortcuts
    Given the board is displaying
    When viewing the footer
    Then it should display "← → Columns | ↑↓ jk Work Units | ↵ Details | ESC Back"
