@done
@board-visualization
@high
@tui
@ui-formatting
@ITF-008
Feature: Improve work unit details panel formatting
  """
  Modifies UnifiedBoardLayout.tsx to change Work Unit Details panel layout. Current implementation: 1-line header + 4-line content (line 1: ID+Title with 2-space padding, line 2: description with 2-space padding, line 3: metadata with 2-space padding, line 4: empty). New implementation: NO header + 5-line content (line 1: ID+Title NO padding, lines 2-4: description up to 3 lines in bold cyan NO padding with ... truncation on line 4 if needed, line 5: metadata NO padding). Panel height is now static at 5 lines. Added wrapText() helper function for word-wrapping descriptions across multiple lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The 'Work Unit Details' header line MUST be removed from the panel
  #   2. Description MUST display up to 3 lines maximum (not 1 line)
  #   3. If description exceeds 3 lines, the third line MUST end with '...' to indicate truncation
  #   4. Left padding (currently 2 spaces) MUST be removed from all content lines
  #   5. Panel MUST remain static at 4 lines total height (existing BOARD-015 constraint)
  #   6. Option A: Line 1 = ID+Title, Lines 2-3 = Description (max 2 lines), Line 4 = Metadata
  #   7. With Option A layout, description uses lines 2-3 (max 2 lines), metadata always on line 4. If description has 3+ lines, truncate on line 3.
  #   8. Truncation '...' should replace the last 3 characters of line 3 to fit within available width (standard truncation pattern)
  #   9. Truncation '...' should replace the last 3 characters of line 3 to fit within available width (standard truncation pattern)
  #
  # EXAMPLES:
  #   1. Work unit with short description (1 line): Shows ID, title on line 1, description on line 2, metadata on line 3, empty line 4. NO header line. NO left padding.
  #   2. No work unit selected: Shows 'No work unit selected' centered on line 1, empty lines 2-4. NO header line.
  #   3. Work unit with no description: Shows ID, title on line 1, empty line 2, metadata on line 3, empty line 4
  #   4. Work unit with long description (5 lines): Line 1 = ID+Title, Line 2 = first desc line, Line 3 = second desc line ending with '...', Line 4 = Metadata
  #   5. Work unit with exactly 2 lines of description: Line 1 = ID+Title, Line 2 = first desc line, Line 3 = second desc line (NO '...'), Line 4 = Metadata
  #   6. Work unit with no description: Line 1 = ID+Title, Line 2 = empty, Line 3 = empty, Line 4 = Metadata
  #
  # QUESTIONS (ANSWERED):
  #   Q: For long descriptions, should the truncation '...' replace the last 3 characters of line 3, or be appended after available width?
  #   A: true
  #
  #   Q: What should the line layout be? Option A: [1: ID+Title, 2-3: Description, 4: Metadata] or Option B: [1: ID+Title, 2: Desc line 1, 3: Desc line 2, 4: Desc line 3 OR Metadata]?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to view work unit details with better formatting
    So that I can read descriptions more easily without visual clutter

  Scenario: Display work unit with short description (1 line)
    Given I am viewing the TUI board
    And a work unit with a 1-line description is selected
    When the Work Unit Details panel is rendered
    Then line 1 should display the work unit ID and title without left padding
    And line 2 should display the description in bold cyan without left padding
    And line 3 should be empty
    And line 4 should be empty
    And line 5 should display metadata without left padding
    And the "Work Unit Details" header line should NOT be displayed

  Scenario: Display work unit with no description
    Given I am viewing the TUI board
    And a work unit with no description is selected
    When the Work Unit Details panel is rendered
    Then line 1 should display the work unit ID and title without left padding
    And line 2 should be empty
    And line 3 should be empty
    And line 4 should be empty
    And line 5 should display metadata without left padding
    And the "Work Unit Details" header line should NOT be displayed

  Scenario: Display work unit with long description (exceeds 3 lines)
    Given I am viewing the TUI board
    And a work unit with a 5-line description is selected
    When the Work Unit Details panel is rendered
    Then line 1 should display the work unit ID and title without left padding
    And line 2 should display the first line of description in bold cyan without left padding
    And line 3 should display the second line of description in bold cyan without left padding
    And line 4 should display the third line of description in bold cyan ending with "..." truncation indicator
    And line 5 should display metadata without left padding
    And the "Work Unit Details" header line should NOT be displayed

  Scenario: Display work unit with exactly 3 lines of description
    Given I am viewing the TUI board
    And a work unit with a 3-line description is selected
    When the Work Unit Details panel is rendered
    Then line 1 should display the work unit ID and title without left padding
    And line 2 should display the first line of description in bold cyan without left padding
    And line 3 should display the second line of description in bold cyan without left padding
    And line 4 should display the third line of description in bold cyan WITHOUT "..." truncation indicator
    And line 5 should display metadata without left padding
    And the "Work Unit Details" header line should NOT be displayed

  Scenario: Display empty state when no work unit selected
    Given I am viewing the TUI board
    And no work unit is selected
    When the Work Unit Details panel is rendered
    Then line 1 should display "No work unit selected" centered
    And lines 2-5 should be empty
    And the "Work Unit Details" header line should NOT be displayed
