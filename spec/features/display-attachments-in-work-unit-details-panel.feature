@done
@tui
@ui-enhancement
@medium
@TUI-012
Feature: Display attachments in work unit details panel
  """
  Truncate list with ellipsis if too long for available terminal width, show keybinding hint for full dialog (TUI-013)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a work unit has no attachments, show 'No attachments'
  #   2. When a work unit has attachments, show each filename extracted from the path
  #   3. Attachments section should fit within the existing 5-line details panel layout
  #   4. Attachments appear on line 4 of the details panel, reducing description from 3 lines to 2 lines
  #   5. When attachments don't fit on line 4, truncate with ellipsis showing count (e.g., 'mockup.png, diagram.png, ...3 more')
  #   6. Display keybinding hint showing which key opens the full attachments dialog (TUI-013)
  #   7. Attachments appear on line 3 of description area (replacing the 3rd description line), reducing description from 3 lines to 2 lines
  #
  # EXAMPLES:
  #   1. Work unit TUI-012 has no attachments → Details panel shows 'No attachments'
  #   2. Work unit TUI-012 has one attachment at spec/attachments/TUI-012/mockup.png → Details panel shows 'mockup.png'
  #   3. Work unit AUTH-001 has two attachments (spec/attachments/AUTH-001/login-flow.png and spec/attachments/AUTH-001/requirements.pdf) → Details panel shows both 'login-flow.png' and 'requirements.pdf'
  #   4. Work unit has 5 attachments but line 4 only fits 2 filenames → Shows 'file1.png, file2.pdf, ...3 more' with keybinding hint
  #
  # QUESTIONS (ANSWERED):
  #   Q: If there are many attachments that don't fit in the available space, should we truncate the list (e.g., show first 3 and '...2 more') or make the section scrollable?
  #   A: true
  #
  #   Q: TUI-013 mentions 'selecting an attachment and pressing o to open' - should TUI-012 include selection UI (highlighting attachments with arrow keys), or is selection UI part of TUI-013?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to view attachments associated with a work unit
    So that I can see what supporting materials are available without leaving the board view

  Scenario: Display 'No attachments' when work unit has no attachments
    Given I am viewing the TUI Kanban board
    When I view the work unit details panel
    Then line 3 should show 'No attachments'
    And a work unit with no attachments is selected

  Scenario: Display single attachment filename
    Given I am viewing the TUI Kanban board
    When I view the work unit details panel
    Then line 3 should show 'mockup.png'
    And a work unit with one attachment 'mockup.png' is selected

  Scenario: Display multiple attachment filenames
    Given I am viewing the TUI Kanban board
    When I view the work unit details panel
    Then line 3 should show both filenames
    And a work unit with two attachments 'login-flow.png' and 'requirements.pdf' is selected
    And filenames should be comma-separated

  Scenario: Truncate attachment list when too many to fit
    Given I am viewing the TUI Kanban board
    When I view the work unit details panel
    Then line 3 should show first 2 filenames
    And a work unit has 5 attachments
    And line 3 can only fit 2 filenames
    And show '...3 more' to indicate truncation
