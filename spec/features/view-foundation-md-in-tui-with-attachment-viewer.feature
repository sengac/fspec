@tui
@cli
@board-visualization
@ui-enhancement
@TUI-029
Feature: View FOUNDATION.md in TUI with attachment viewer
  """
  Add 'D' key binding in BoardView to open spec/FOUNDATION.md in browser via HTTP
  Reuse existing attachment server to serve FOUNDATION.md (same /view/{path} endpoint)
  Use openInBrowser() utility to spawn browser with HTTP URL (same as attachment viewing)
  Update KeybindingShortcuts component to display '◆ D View FOUNDATION.md' after 'F View Changed Files'
  NOT a view mode - directly opens browser, does not change TUI view
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must reuse existing attachment server to serve FOUNDATION.md via HTTP (same infrastructure)
  #   2. FOUNDATION.md is NOT an attachment and does NOT belong to any work unit
  #   3. New keybinding 'D' (uppercase D) must open spec/FOUNDATION.md in browser
  #   4. UI must display '◆ D View FOUNDATION.md' in the keybinding help line
  #   5. UI element must appear to the RIGHT of 'F View Changed Files' with diamond separator
  #   6. Must use diamond separator '◆' before 'D View FOUNDATION.md' text
  #   7. File path is spec/FOUNDATION.md relative to project root (cwd)
  #   8. Browser opens via openInBrowser() utility with HTTP URL (same as attachments)
  #   9. NOT a view mode - spawns browser, does NOT change TUI view
  #   10. If FOUNDATION.md does not exist, log error but do not crash
  #
  # EXAMPLES:
  #   1. User runs 'fspec' (TUI), presses 'D' key, browser opens with spec/FOUNDATION.md rendered as HTML
  #   2. User sees keybinding help line showing '◆ F View Changed Files ◆ D View FOUNDATION.md' in main TUI
  #   3. Browser displays FOUNDATION.md with rendered Markdown (headings, lists, code blocks, etc.)
  #   4. User presses 'D' in project without spec/FOUNDATION.md, error logged but TUI does not crash
  #   5. Attachment server serves both work unit attachments AND FOUNDATION.md via same /view/{path} endpoint
  #   6. Diamond separator '◆' appears between 'F View Changed Files' and 'D View FOUNDATION.md' in help line
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to view the spec/FOUNDATION.md file directly from the main board
    So that I can quickly review project foundation documentation without leaving the TUI

  Scenario: Press D key to open FOUNDATION.md in browser
    Given I am viewing the main TUI board
    And spec/FOUNDATION.md exists in the project
    And the attachment server is running
    When I press the "D" key
    Then a browser should open with URL "http://localhost:{port}/view/spec/FOUNDATION.md"
    And the browser should display rendered Markdown content

  Scenario: Keybinding help line displays D View FOUNDATION.md
    Given I am viewing the main TUI board
    When I look at the keybinding help line
    Then I should see "F View Changed Files" on the help line
    And I should see a diamond separator "◆" after it
    And I should see "D View FOUNDATION.md" to the right of the separator
    And the help line should read "◆ F View Changed Files ◆ D View FOUNDATION.md"

  Scenario: Error message when FOUNDATION.md does not exist
    Given I am viewing the main TUI board
    And spec/FOUNDATION.md does not exist in the project
    When I press the "D" key
    Then I should see an error message
    And the error message should say "FOUNDATION.md not found at spec/FOUNDATION.md"
    And the TUI should not crash

  Scenario: Reuse attachment server infrastructure
    Given the attachment server is running for work unit attachments
    When I press the "D" key to view FOUNDATION.md
    Then the same HTTP server should serve spec/FOUNDATION.md
    And the server should use the same /view/{path} endpoint pattern
    And openInBrowser() should be called with the HTTP URL

  Scenario: Diamond separator appears in correct position
    Given I am viewing the main TUI board
    When I examine the keybinding help line
    Then "F View Changed Files" should appear first
    And a diamond separator "◆" should appear immediately after it
    And "D View FOUNDATION.md" should appear immediately after the separator
    And no other text should appear between these elements
