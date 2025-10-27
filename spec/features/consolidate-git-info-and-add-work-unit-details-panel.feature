@high
@tui
@interactive-cli
@BOARD-007
Feature: Consolidate Git info and add work unit details panel
  """
  Modify UnifiedBoardLayout.tsx to restructure top panels. Combine Git Stashes and Changed Files into single Git Context panel (lines 181-223). Replace old Changed Files section with new Work Unit Details panel that displays selected work unit metadata (type icon, title, description truncated to 3 lines, dependencies, epic, estimate, status). When no work unit selected, show 'No work unit selected' message. Pass selected work unit data via props to BoardView component.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Git Stashes and Changed Files must be combined into single 'Git Context' panel
  #   2. Changed Files section must be replaced with Work Unit Details panel
  #   3. Work Unit Details panel shows type icon, title, description, dependencies, and other metadata
  #   4. Description must be truncated after few lines with indicator to press Enter for full details
  #   5. When no work unit selected, display user-friendly message like 'No work unit selected'
  #
  # EXAMPLES:
  #   1. Git Context panel shows 'Git Stashes (2)' followed by stash list, then 'Changed Files (3 staged, 1 unstaged)' with file list
  #   2. Work unit BOARD-001 selected: shows 📖 story icon, title, truncated description, dependencies list
  #   3. No work unit selected in empty column: shows 'No work unit selected' message
  #
  # ========================================
  Background: User Story
    As a developer using TUI board
    I want to see git context and selected work unit details in one view
    So that I can understand my current work context without switching views

  Scenario: Git Context panel combines stashes and changed files
    Given UnifiedBoardLayout component renders the TUI board
    And there are 2 git stashes
    And there are 3 staged files and 1 unstaged file
    When the Git Context panel is rendered
    Then it should display "Git Stashes (2)" as the first section
    And it should list the stash names below the header
    And it should display "Changed Files (3 staged, 1 unstaged)" below the stashes
    And it should list the changed files with staged/unstaged indicators
    And both sections should be in the same panel box

  Scenario: Work Unit Details panel shows selected work unit metadata
    Given UnifiedBoardLayout component renders the TUI board
    And work unit BOARD-001 is selected
    And BOARD-001 is a story with title "Test Feature"
    And BOARD-001 has description "This is a longer description that spans multiple lines and needs to be truncated for display"
    And BOARD-001 has dependencies ["BOARD-002", "BOARD-003"]
    When the Work Unit Details panel is rendered
    Then it should display the story icon 📖
    And it should display "BOARD-001: Test Feature" as the title
    And it should display the first 3 lines of the description
    And it should display "Press ↵ to view full details" indicator
    And it should display "Dependencies: BOARD-002, BOARD-003"
    And it should display epic, estimate, and status metadata

  Scenario: Work Unit Details panel shows empty state when nothing selected
    Given UnifiedBoardLayout component renders the TUI board
    And no work unit is selected (focused column is empty)
    When the Work Unit Details panel is rendered
    Then it should display "No work unit selected"
    And the message should be user-friendly and centered
    And no metadata fields should be displayed
