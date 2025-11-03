@ui-enhancement
@tui-component
@TUI-015
Feature: Refactor header and details to component-based flexbox

  """
  Architecture notes:
  - Uses Ink (React for CLI) component-based architecture for all UI rendering
  - Hybrid border system: Box borderStyle='single' for HeaderContainer, junction separators (├┤┬┼┴) for continuous borders
  - NO string injection via rows.push() - ALL content must be Ink components (Box, Text)
  - Flexbox-only layout: flexGrow/flexShrink for dynamic sizing, static heights only where required
  - Component files: CheckpointStatus, KeybindingShortcuts, WorkUnitTitle, WorkUnitDescription, WorkUnitMetadata
  - Refactors UnifiedBoardLayout.tsx lines 496-617 (delete string injection) and 707-740 (replace hybrid with pure components)
  - Dynamic viewport calculation based on actual component heights (NOT magic number fixedRows=17)

  Critical implementation requirements:
  - HeaderContainer: Box borderStyle='single' with height implicit from 4-line content (Logo + InfoContainer)
  - InfoContainer: height={4} static, flexDirection='column', contains CheckpointStatus + KeybindingShortcuts
  - WorkUnitDetailsContainer: height={5} static, borderLeft + borderRight only (junction separators render top/bottom)
  - WorkUnitDescription: flexGrow={1} to fill 3 lines, bold cyan text, word-wrap with ellipsis on line 3
  - Junction separators connect all sections into ONE continuous border structure (├┤┬┼┴)
  - calculateViewportHeight must sum component heights, NOT use hardcoded fixedRows
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. NO string injection - all content must be rendered as Ink components
  #   2. Use ONLY flexbox (flexGrow/flexShrink) and static heights - NEVER percentages
  #   3. Header container must have internal height of 4 lines (Logo + CheckpointStatus + KeybindingShortcuts)
  #   4. Work unit details container must have internal height of 5 lines (Title[1] + Description[3] + Metadata[1])
  #   5. Borders must use hybrid border system where corners connect properly (Box borderStyle for header, separators for details)
  #   6. Logo component width is 12 characters with flexShrink, InfoContainer uses flexGrow
  #   7. KeybindingShortcuts component has borderTop only (all other borders false)
  #   8. WorkUnitDescription component height is 3 lines using flexGrow with bold cyan text
  #
  # EXAMPLES:
  #   1. Header Box renders with Logo (12ch width) on left and InfoContainer (flexGrow) on right, total internal height 4 lines
  #   2. CheckpointStatus shows 'Checkpoints: 3 Manual, 5 Auto' when checkpoints exist
  #   3. CheckpointStatus shows 'Checkpoints: None' when no checkpoints exist
  #   4. KeybindingShortcuts displays 'C View Checkpoints ◆ F View Changed Files' with borderTop only
  #   5. WorkUnitTitle displays 'BOARD-001: Feature Title' on single line, truncates if too long
  #   6. WorkUnitDescription wraps text across 3 lines, truncates line 3 with '...' if description exceeds 3 lines
  #   7. WorkUnitMetadata displays 'Epic: auth | Estimate: 5pts | Status: implementing' on single line
  #   8. Fixed rows calculation is 19 total (6 header Box + 1 separator + 5 details + 7 footer/columns)
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to view header and work unit details as pure Ink components
    So that the layout is maintainable, testable, and uses proper flexbox instead of string injection

  Scenario: Header renders with component-based flexbox layout
    Given the UnifiedBoardLayout component is mounted
    When the header section renders
    Then the HeaderContainer should use Box with borderStyle='single'
    And the HeaderContainer should contain a Logo component with width 12 and flexShrink
    And the HeaderContainer should contain an InfoContainer with flexGrow and height 4
    And the InfoContainer should contain CheckpointStatus with flexGrow
    And the InfoContainer should contain KeybindingShortcuts with flexGrow and borderTop
    And NO string injection via rows.push should be used for header rendering

  Scenario: CheckpointStatus displays count when checkpoints exist
    Given there are 3 manual checkpoints and 5 auto checkpoints
    When the CheckpointStatus component renders
    Then it should display "Checkpoints: 3 Manual, 5 Auto"
    And it should use flexGrow to fill available vertical space
    And it should have NO borders (all border props false or undefined)

  Scenario: CheckpointStatus displays none when no checkpoints exist
    Given there are 0 manual checkpoints and 0 auto checkpoints
    When the CheckpointStatus component renders
    Then it should display "Checkpoints: None"

  Scenario: KeybindingShortcuts renders with borderTop only
    Given the KeybindingShortcuts component is rendered inside InfoContainer
    When the component displays
    Then it should show "C View Checkpoints ◆ F View Changed Files"
    And it should have borderTop set to true
    And it should have borderBottom, borderLeft, borderRight set to false
    And the borderTop should be internal to the HeaderContainer

  Scenario: WorkUnitDetailsContainer renders with component-based layout
    Given a work unit is selected with id "TUI-015" and title "Feature Title"
    When the WorkUnitDetailsContainer renders
    Then it should have height 5 and flexDirection column
    And it should have borderLeft and borderRight only (NOT borderStyle)
    And it should contain WorkUnitTitle component with height 1
    And it should contain WorkUnitDescription component with flexGrow 1
    And it should contain WorkUnitMetadata component with height 1
    And NO string injection via rows.push should be used for details rendering

  Scenario: WorkUnitTitle displays and truncates without ellipsis
    Given a work unit with id "BOARD-001" and title "Very Long Feature Title That Exceeds Width"
    And the container width is 30 characters
    When the WorkUnitTitle component renders
    Then it should display "BOARD-001: Very Long Featur" without ellipsis
    And it should use Text wrap='truncate' prop

  Scenario: WorkUnitDescription wraps text across 3 lines with ellipsis on line 3
    Given a work unit with description "This is a very long description that spans many lines and needs to be truncated properly when it exceeds the three line limit"
    When the WorkUnitDescription component renders
    Then it should wrap the text using word-wrap (NOT char-wrap)
    And line 1 should display "This is a very long description that"
    And line 2 should display "spans many lines and needs to be"
    And line 3 should display "truncated properly when it exce..." with ellipsis
    And lines 4+ should be discarded
    And all text should be bold cyan color

  Scenario: WorkUnitDescription displays 3 blank lines when empty
    Given a work unit with empty or whitespace-only description
    When the WorkUnitDescription component renders
    Then lines 2-4 should display as blank lines
    And it should NOT display "No description" text

  Scenario: WorkUnitMetadata displays available fields with separator
    Given a work unit with epic "auth", estimate 5, and status "implementing"
    When the WorkUnitMetadata component renders
    Then it should display "Epic: auth | Estimate: 5pts | Status: implementing"
    And fields should be joined with " | " separator
    And it should have height 1

  Scenario: WorkUnitMetadata displays only present fields
    Given a work unit with only estimate 3 (no epic, no status)
    When the WorkUnitMetadata component renders
    Then it should display "Estimate: 3pts"
    And it should NOT display leading or trailing " | " separators

  Scenario: No work unit selected displays centered message
    Given no work unit is selected
    When the WorkUnitDetailsContainer renders
    Then line 1 should display "No work unit selected" centered
    And lines 2-5 should display as empty lines
    And it should NOT display "No description" text

  Scenario: calculateViewportHeight uses component heights not magic numbers
    Given a terminal height of 24 rows
    When calculateViewportHeight is called
    Then it should calculate HeaderContainer height as 6 (1+4+1)
    And it should calculate separators as 4 rows
    And it should calculate Details height as 5 rows
    And it should calculate column headers as 2 rows
    And it should calculate footer as 2 rows
    And it should calculate bottom border as 1 row
    And it should sum all component heights (NOT use fixedRows=17 magic number)
    And it should return 6 rows for dynamic columns (24 - 18 static)

  Scenario: Terminal resize recalculates column viewport dynamically
    Given a terminal height of 40 rows
    When calculateViewportHeight is called
    Then static component heights should remain at 18 rows
    And it should return 22 rows for dynamic columns (40 - 18 static)

  Scenario: Junction separators create continuous border structure
    Given the UnifiedBoardLayout is fully rendered
    When the border structure is displayed
    Then the HeaderContainer bottom should connect via junction separator (├┤)
    And the junction should extend vertical borders to Details top
    And the Details bottom should connect via junction with column dividers (├┬┤)
    And column headers should connect via cross junction (├┼┤)
    And the footer should connect via bottom junction (├┴┤)
    And ALL borders should form ONE continuous structure using junctions
    And there should be NO gaps or double lines between sections
