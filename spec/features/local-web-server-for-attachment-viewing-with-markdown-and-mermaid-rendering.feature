@done
@high
@tui
@attachment-viewer
@http-server
@TUI-020
Feature: Local web server for attachment viewing with markdown and mermaid rendering
  """
  Express-based HTTP server with lifecycle tied to BoardView component (React useEffect). Server uses port=0 for random port allocation. Client-side rendering with Prism (syntax highlighting), Mermaid (diagrams), and theme detection via window.matchMedia. Reuses MindStrike pattern for code blocks. Path validation prevents directory traversal. Extensible architecture for future content types (example maps, event storming).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Server must only run when TUI is active (BoardView mounted)
  #   2. Server must serve markdown files as HTML with rendered content
  #   3. Mermaid code blocks must be rendered as diagrams in the browser
  #   4. Server must prevent directory traversal attacks with path validation
  #   5. Server must integrate with TUI-019 attachment dialog to open files via http:// URLs
  #   6. Server must serve non-markdown files (images, PDFs, etc) with appropriate content types
  #   7. Server uses random port allocation (port=0) to avoid conflicts
  #   8. Server failure must be non-fatal - TUI continues working without attachment viewing
  #   9. Viewer must detect OS theme preference using window.matchMedia('prefers-color-scheme: dark')
  #   10. Viewer must provide theme toggle button to override OS preference
  #   11. Viewer must store theme preference in localStorage and restore on page load
  #   12. Markdown renderer must support syntax-highlighted code blocks (reuse MindStrike pattern)
  #   13. Server architecture must be extensible to support future content types (example maps, event storming boards)
  #
  # EXAMPLES:
  #   1. User opens TUI, server starts automatically on random port
  #   2. User presses 'o' on work unit with markdown attachment, browser opens showing formatted HTML with mermaid diagram
  #   3. User exits TUI, server stops automatically
  #   4. Malicious user tries ../../../etc/passwd path, server blocks with 403 Forbidden
  #   5. User opens PNG attachment, browser displays image directly
  #   6. User with dark OS theme opens attachment, sees dark mermaid diagrams. Toggles to light theme, preference saved. Reopens attachment later, light theme restored from localStorage
  #   7. User opens markdown with Python code block, browser shows syntax-highlighted code with copy button and language badge
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the server also handle non-markdown files (like images, PDFs) or only markdown?
  #   A: true
  #
  #   Q: What should happen if the server fails to start (port conflict, permission error)? Should TUI still work without it?
  #   A: true
  #
  #   Q: Should mermaid diagrams use light theme or dark theme to match TUI aesthetics?
  #   A: true
  #
  #   Q: You mentioned future support for example maps and event storming boards - should the server architecture support rendering different content types beyond markdown?
  #   A: true
  #
  #   Q: Should we reuse the existing mermaid validation utilities from the codebase, or is the client-side rendering enough?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer viewing work unit attachments in TUI
    I want to open markdown files with mermaid diagrams in my browser
    So that I can view formatted documentation and diagrams without leaving my workflow

  Scenario: Server starts automatically when TUI opens
    Given the TUI is not running
    When I run the TUI with 'fspec'
    Then the attachment server should start on a random available port
    And the server should be accessible at http://localhost:<port>

  Scenario: Open markdown attachment with mermaid diagram in browser
    Given the TUI is running with attachment server active
    When I press 'a' on the work unit
    Then the browser should open showing the formatted HTML
    And I have a work unit with a markdown attachment containing a mermaid diagram
    And the mermaid diagram should be rendered as an SVG

  Scenario: Server stops automatically when TUI exits
    Given the TUI is running with attachment server active
    When I exit the TUI
    Then the attachment server should stop
    And the server port should be released

  Scenario: Block directory traversal attacks
    Given the TUI is running with attachment server active
    When a request is made for '../../../etc/passwd'
    Then the server should return 403 Forbidden
    And the file should not be served

  Scenario: Serve non-markdown files directly
    Given the TUI is running with attachment server active
    When I press 'a' on the work unit
    Then the browser should open displaying the image directly
    And I have a work unit with a PNG image attachment
    And the content-type should be 'image/png'

  Scenario: Theme detection and persistence
    Given the TUI is running with attachment server active
    When I open a markdown attachment with mermaid diagrams
    Then the viewer should display with dark theme
    And my OS is set to dark mode
    When I click the theme toggle button
    Then the theme should switch to light mode
    And the preference should be saved in localStorage
    When I reopen the attachment later
    Then the light theme should be restored from localStorage

  Scenario: Syntax-highlighted code blocks with UI features
    Given the TUI is running with attachment server active
    When I press 'a' on the work unit
    Then the browser should show syntax-highlighted Python code
    And I have a markdown attachment with a Python code block
    And a copy button should be visible on hover
    And a language badge showing 'python' should be displayed
