@interactive-cli
@high
@tui
@bug-fix
@BOARD-006
Feature: Incorrect column dividers in non-columnar TUI sections
  """
  Fix UnifiedBoardLayout.tsx to use correct box-drawing characters. Modify buildBorderRow() helper to accept separator type parameter: 'plain' (──), 'top' (┬), 'cross' (┼), 'bottom' (┴).

  Lines requiring fixes:
  - Line 168 (top border): Use 'plain' (no columns above top border)
  - Line 184 (after Git Stashes): Use 'plain' (no columns above or below)
  - Line 202 (after Changed Files): Use 'top' (columns start below)
  - Line 227 (Kanban header separator): Keep 'cross' (columns above and below)
  - Line 265 (before footer): Use 'bottom' (columns end above)
  - Line 271 (bottom border): Use 'plain' (no columns below bottom border)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Line 184 (after Git Stashes): Use ├──┤ with NO column dividers (no columns above or below)
  #   2. Line 202 (after Changed Files): Use ├──┬──┤ with TOP junctions (no columns above, columns start below)
  #   3. Line 252 (before key mapping footer): Use ├──┴──┤ with BOTTOM junctions (columns end above, no columns below)
  #   4. Lines 155 and 214: Keep ├──┼──┤ for actual Kanban board section (columns above and below)
  #   5. buildBorderRow helper must accept separator type parameter to distinguish between ─ (plain), ┬ (top), ┼ (cross), ┴ (bottom)
  #
  # EXAMPLES:
  #   1. Line 184 currently shows ├──┼──┤ but should show ├────────┤ (no column dividers between Git Stashes and Changed Files)
  #   2. Line 202 currently shows ├──┼──┤ but should show ├──┬──┬──┤ (top junctions where Kanban columns start)
  #   3. Line 252 currently shows ├──┼──┤ but should show ├──┴──┴──┤ (bottom junctions where Kanban columns end)
  #   4. Lines 155 and 214 correctly use ├──┼──┤ for the Kanban board header separator
  #
  # ========================================
  Background: User Story
    As a developer viewing TUI board
    I want to see correct box-drawing separators between sections
    So that the interface looks visually correct and separators match their context

  Scenario: Separator between Git Stashes and Changed Files
    Given UnifiedBoardLayout component renders Git Stashes and Changed Files sections
    And both sections are full-width with no columns
    When the separator between them is rendered at line 184
    Then it should use ├────┤ with plain horizontal separators
    And it should NOT use ├──┼──┤ with column dividers

  Scenario: Transition from Changed Files to Kanban board
    Given Changed Files section is full-width with no columns
    And Kanban board section below has 7 columns
    When the separator between them is rendered at line 202
    Then it should use ├──┬──┬──┤ with top junction characters
    And the ┬ characters mark where columns start below

  Scenario: Transition from Kanban board to key mapping footer
    Given Kanban board section has 7 columns
    And key mapping footer below is full-width with no columns
    When the separator between them is rendered at line 252
    Then it should use ├──┴──┴──┤ with bottom junction characters
    And the ┴ characters mark where columns end above

  Scenario: Top border uses plain horizontal separator
    Given UnifiedBoardLayout component renders the table
    And the top border is the first row with no columns above it
    When the top border is rendered at line 168
    Then it should use ┌────┐ with plain horizontal separators
    And it should NOT use ┌──┬──┐ with column dividers

  Scenario: Bottom border uses plain horizontal separator
    Given UnifiedBoardLayout component renders the table
    And the bottom border is the last row with no columns below it
    When the bottom border is rendered at line 271
    Then it should use └────┘ with plain horizontal separators
    And it should NOT use └──┴──┘ with column dividers
