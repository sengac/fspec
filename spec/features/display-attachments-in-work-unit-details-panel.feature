@attachments
@ui-enhancement
@tui
@medium
@TUI-012
Feature: Display attachments in work unit details panel
  """

  Key architectural decisions:
  - Display attachments in work unit details panel (accessed via Enter key on work unit)
  - Use Ink's Box and Text components for rendering attachment list
  - Read attachment data from work unit's attachments array in work-units.json
  - Show filename extracted from attachment path
  - Show optional description if present

  Dependencies and integrations:
  - src/tui/components/WorkUnitDetailsPanel.tsx (or similar) - Details panel component
  - src/storage/workUnits.ts - Work unit storage utilities
  - @types/work-units.ts - WorkUnit interface with attachments field

  Critical implementation requirements:
  - MUST display attachments section below work unit description
  - MUST show 'No attachments' when attachments array is empty or undefined
  - MUST extract filename from relative path for display
  - MUST show description when provided (format: 'filename - description')
  - UI should use consistent styling with rest of details panel

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Attachments section should be displayed in the work unit details panel below the description
  #   2. Each attachment should show the filename and optional description
  #   3. If no attachments exist, show 'No attachments' message
  #   4. Attachment paths are stored as relative paths from project root
  #
  # EXAMPLES:
  #   1. Work unit has 2 attachments (diagram.png, requirements.pdf), details panel shows both with filenames
  #   2. Work unit has attachment with description 'Architecture diagram v2', details panel shows filename and description
  #   3. Work unit has no attachments, details panel shows 'No attachments' message
  #   4. Work unit has 1 attachment in spec/attachments/TUI-012/mockup.png, displayed as 'mockup.png'
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to view attachments associated with a work unit in the details panel
    So that I can quickly see what supporting materials are available for the work

  Scenario: Display multiple attachments in details panel
    Given I have a work unit with 2 attachments (diagram.png and requirements.pdf)
    When I press Enter to view the work unit details
    Then the details panel should show an 'Attachments' section
    And the section should list 'diagram.png'
    And the section should list 'requirements.pdf'

  Scenario: Display attachment with description
    Given I have a work unit with an attachment 'diagram.png' with description 'Architecture diagram v2'
    When I press Enter to view the work unit details
    Then the attachments section should show 'diagram.png - Architecture diagram v2'

  Scenario: Display 'No attachments' message when none exist
    Given I have a work unit with no attachments
    When I press Enter to view the work unit details
    Then the attachments section should show 'No attachments'
