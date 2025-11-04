@done
@attachments
@tui
@medium
@TUI-013
Feature: Open attachments in browser from details panel
  """

  Key architectural decisions:
  - Use 'open' npm package for cross-platform browser launching (same as cage project)
  - Create openAttachment utility function similar to cage's openInBrowser
  - Convert relative attachment paths to absolute file:// URLs before opening
  - Use wait: false option to keep TUI running after launching browser
  - Add keyboard shortcut 'o' to open selected attachment in details panel

  Dependencies and integrations:
  - npm package: 'open' (add as dependency)
  - src/tui/components/WorkUnitDetailsPanel.tsx - Add keyboard handler for 'o' key
  - src/utils/browserLauncher.ts (new) - Browser launching utility
  - path.resolve() for converting relative to absolute paths

  Critical implementation requirements:
  - MUST use file:// protocol for local file paths
  - MUST convert relative paths (spec/attachments/...) to absolute paths
  - MUST use {wait: false} to prevent blocking TUI
  - MUST work cross-platform (macOS/Windows/Linux)
  - Error handling: show error message in TUI if file doesn't exist
  - Keyboard: 'o' key when attachment is focused/selected

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. User should be able to select an attachment and press 'o' to open it
  #   2. Opening attachment should use 'open' npm package for cross-platform browser launching
  #   3. Attachment file path should be converted from relative to absolute path before opening
  #   4. Opening should use file:// protocol for local files
  #   5. TUI should remain open after launching browser (non-blocking)
  #
  # EXAMPLES:
  #   1. User selects 'diagram.png' attachment and presses 'o', browser opens file:///path/to/spec/attachments/TUI-012/diagram.png
  #   2. User opens PDF attachment, browser opens it in default PDF viewer
  #   3. After opening attachment, TUI remains active and user can continue working
  #   4. Opening works on macOS (open command), Windows (start), and Linux (xdg-open)
  #
  # ========================================
  Background: User Story
    As a developer viewing work unit attachments in fspec TUI
    I want to open an attachment file in my default browser
    So that I can quickly view diagrams, mockups, and documents without leaving the TUI workflow

  Scenario: Open image attachment in browser
    Given I am viewing work unit details with an attachment 'diagram.png'
    When I select the attachment and press 'o'
    Then the default browser should open the file with file:// protocol
    And the TUI should remain active

  Scenario: Open PDF attachment in default viewer
    Given I am viewing work unit details with a PDF attachment 'requirements.pdf'
    When I select the attachment and press 'o'
    Then the PDF should open in the default PDF viewer/browser

  Scenario: Handle missing attachment file gracefully
    Given I am viewing work unit details with an attachment that doesn't exist on disk
    When I select the attachment and press 'o'
    Then the TUI should show an error message
    And the browser should not open
