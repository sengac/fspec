@ui-refinement
@tui
@BOARD-015
Feature: Fix work unit details panel to be static 4 lines high
  """
  Static 4-line layout prevents dynamic height changes that cause metadata line to disappear. UnifiedBoardLayout calculates viewport height with fixedRows=17 to account for all header sections including the 5-line Work Unit Details panel (1 header + 4 content), plus footer sections (bottom separator + footer + bottom border = 3 rows).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Work Unit Details panel must always be exactly 4 content lines high (plus 1 header line)
  #   2. Line 1 must show: work unit ID and title
  #   3. Line 2 must show: first line of description (or empty if no description)
  #   4. Line 3 must show: metadata (Epic, Estimate, Status separated by |)
  #   5. Line 4 must be: empty spacing line
  #   6. Panel height must not change based on content - always 4 lines
  #   7. Viewport height calculation must account for the 5-line Work Unit Details section (1 header + 4 content)
  #
  # EXAMPLES:
  #   1. Work unit TECH-001 with long description shows: Line 1: ID+title, Line 2: first desc line, Line 3: Status: backlog, Line 4: empty
  #   2. Work unit with no description shows: Line 1: ID+title, Line 2: empty, Line 3: Epic: x | Estimate: 5pts | Status: implementing, Line 4: empty
  #   3. No work unit selected shows: Line 1: 'No work unit selected' (centered), Lines 2-4: empty
  #
  # ========================================
  Background: User Story
    As a developer viewing the TUI Kanban board
    I want to see work unit details in a consistent layout
    So that the metadata line doesn't disappear when viewport calculations change

  Scenario: Work unit with description displays all 4 content lines
    Given I am viewing the TUI Kanban board
    And a work unit with a description is selected
    When I look at the Work Unit Details panel
    Then I should see exactly 4 content lines
    And line 1 should show the work unit ID and title
    And line 2 should show the first line of the description
    And line 3 should show metadata (Epic, Estimate, Status)
    And line 4 should be empty

  Scenario: Work unit without description maintains 4-line height
    Given I am viewing the TUI Kanban board
    And a work unit with no description is selected
    When I look at the Work Unit Details panel
    Then I should see exactly 4 content lines
    And line 2 should be empty
    And line 3 should still show metadata

  Scenario: No work unit selected shows empty 4-line panel
    Given I am viewing the TUI Kanban board
    And no work unit is selected
    When I look at the Work Unit Details panel
    Then I should see exactly 4 content lines
    And line 1 should show 'No work unit selected' centered
    And lines 2-4 should be empty
