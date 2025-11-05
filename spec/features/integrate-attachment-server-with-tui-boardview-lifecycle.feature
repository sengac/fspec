@tui
@integration
@attachment-viewer
@REFAC-004
Feature: Integrate attachment server with TUI (BoardView lifecycle)
  """
  URL format: http://localhost:{PORT}/view/{relativePath} where relativePath is from spec/attachments/
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The attachment server must start when BoardView mounts
  #   2. The attachment server must stop when BoardView unmounts
  #   3. The server port must be stored in BoardView state and accessible to child components
  #   4. AttachmentDialog must construct http://localhost:PORT/view/PATH URLs instead of file:// URLs
  #   5. The server must use the project root (process.cwd()) as the base directory
  #   6. Server startup failures must be logged but not crash the TUI
  #   7. The server must only run when the TUI is active (not during CLI commands)
  #
  # EXAMPLES:
  #   1. When I open the TUI and press 'a' on a work unit with an attachment, the browser opens with http://localhost:XXXX/view/spec/attachments/...
  #   2. When I exit the TUI, the attachment server stops and releases the port
  #   3. When the server fails to start (port conflict), I see a warning in the logs but the TUI continues working
  #   4. When I open a markdown attachment, the browser shows rendered HTML with working mermaid diagrams
  #   5. When I open an image attachment, the browser displays the image directly
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to view attachments in a browser with rendered markdown and mermaid
    So that I can see formatted documentation instead of raw markdown files

  Scenario: Open attachment from TUI and browser shows HTTP URL
    Given the TUI is running and the attachment server has started
    When I press 'a' on the work unit with the attachment
    Then the browser opens with a URL matching "http://localhost:[0-9]+/view/spec/attachments/TUI-012/architecture.md"
    And a work unit exists with an attachment at "spec/attachments/TUI-012/architecture.md"
    And the logs show "Opening attachment URL: http://localhost:"

  Scenario: Server stops when TUI exits
    Given the TUI is running and the attachment server is active on port 3456
    When I exit the TUI by pressing 'q'
    Then the attachment server should stop
    And port 3456 should be released and available for reuse
    And the logs should show "Attachment server stopped"

  Scenario: TUI continues working when server fails to start
    Given port 3000 is already in use by another process
    When I start the TUI
    Then the TUI should start successfully despite server failure
    And the logs should show a warning about server startup failure
    And I should be able to navigate the board normally

  Scenario: Markdown attachment renders with mermaid diagrams
    Given the TUI is running with attachment server on port 3456
    When I press 'a' to open the attachment
    Then the browser opens with URL "http://localhost:3456/view/spec/attachments/TUI-012/architecture.md"
    And a work unit has a markdown attachment "spec/attachments/TUI-012/architecture.md" containing mermaid code blocks
    And the browser displays rendered HTML with formatted text
    And mermaid diagrams are rendered as interactive SVG graphics

  Scenario: Image attachment displays directly in browser
    Given the TUI is running with attachment server on port 3456
    When I press 'a' to open the image attachment
    Then the browser opens with URL "http://localhost:3456/view/spec/attachments/TUI-012/diagram.png"
    And a work unit has an image attachment "spec/attachments/TUI-012/diagram.png"
    And the browser displays the image directly without rendering errors
