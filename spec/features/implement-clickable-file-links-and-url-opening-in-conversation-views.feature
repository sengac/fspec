@navigation
@virtuallist
@tui
@LINK-001
Feature: Implement Clickable File Links and URL Opening in Conversation Views
  """
  Create src/tui/types/links.ts with LinkSegment, LinkType, ParsedLine, and LinkEnvironment interfaces
  Extend ConversationLine type in src/tui/types/conversation.ts with linkSegments?: LinkSegment[] and hasLinks?: boolean fields
  Create linkUtils.ts for regex-based link detection supporting absolute paths, relative paths, URLs, and line:column notation
  Create LinkifiedText.tsx Ink component to render clickable text segments using onPress handlers
  Update conversationUtils.ts wrapMessageToLines to preserve link boundaries when wrapping long lines
  Create linkHandlers.ts with VS Code environment detection (VSCODE_PID, TERM_PROGRAM) and cross-platform file/URL opening
  Create VSCodeIntegration class in linkHandlers.ts with environment detection, URI creation, and file opening methods
  Update VirtualList.tsx to pass linkEnvironment and onLinkClick/onLinkHover/onLinkFocus props to renderItem
  Wire AgentView.tsx with link handlers by passing onLinkClick callback through VirtualList to LinkifiedText
  Wire SplitSessionView.tsx with same link handlers pattern as AgentView for consistent behavior
  Add links.* configuration schema to support enabled, maxLinksPerMessage, validateFiles, editor, useRelativePaths settings
  Use open package for cross-platform URL and file opening as fallback when not in VS Code
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. File paths (absolute and relative) in conversation messages must be detected and rendered as clickable links
  #   2. HTTP/HTTPS URLs in conversation messages must be detected and rendered as clickable links
  #   3. File links with line:column notation (e.g., /path/file.ts:42:10) must open at the specified location
  #   4. When running in VS Code terminal, file links must open in VS Code editor
  #   5. URLs must open in system default browser
  #   6. Links must work in both AgentView and SplitSessionView components
  #   7. Windows paths (e.g., C:\path\to\file.ts:10:5) must be detected and rendered as clickable links
  #   8. VS Code URI scheme (vscode://file/path:line:col) must be supported and open correctly
  #   9. Links must preserve boundaries when text wraps across lines - links should not be split mid-word
  #   10. Invalid or non-existent file paths must show an error message when clicked
  #   11. Invalid or unreachable URLs must show an error message when clicked
  #   12. Keyboard navigation (Tab to move between links, Enter to activate) must be supported
  #   13. Performance limit: maximum number of links per message must be configurable to prevent slowdown
  #   14. Relative paths should be displayed instead of absolute paths when possible for better readability
  #   15. Link hover should show a preview tooltip with the full path or URL
  #   16. Links must be clickable in ALL message types including AI responses and tool output (Bash, Read, Write, etc.)
  #   17. Directory links should open in the system file manager or VS Code's file explorer panel
  #   18. File links must be displayed in blue color, URL links must be displayed in cyan color for visual distinction
  #   19. mailto: and other non-http URI schemes must be passed to the system default handler
  #
  # EXAMPLES:
  #   1. Absolute path /home/user/file.ts is detected and becomes clickable
  #   2. Relative path ./src/file.ts is detected and becomes clickable
  #   3. URL https://github.com/user/repo is detected and becomes clickable
  #   4. File path /home/user/file.ts:42:10 opens file at line 42, column 10
  #   5. Clicking file link in VS Code terminal opens file in VS Code editor
  #   6. Clicking URL opens in user's default browser
  #   7. Windows path C:\Users\dev\project\file.ts:10:5 is detected and becomes clickable
  #   8. vscode://file/home/user/file.ts:42:10 URI opens file in VS Code at specified location
  #   9. Long URL https://github.com/user/repo/blob/main/src/very/long/path/file.ts wraps to next line but remains a single clickable link
  #   10. Clicking /nonexistent/path/file.ts shows error: File not found
  #   11. Clicking https://invalid.domain.notreal shows error: Unable to open URL
  #   12. Pressing Tab moves focus to next link in message, Enter activates the focused link
  #   13. Message with 100 URLs only renders first 50 as clickable when links.maxLinksPerMessage is set to 50
  #   14. Absolute path /home/user/projects/fspec/src/file.ts displays as src/file.ts when working directory is /home/user/projects/fspec
  #   15. Hovering over ./src/file.ts link shows tooltip with full resolved path
  #   16. File path in Bash tool output /home/user/file.ts is detected and becomes clickable
  #   17. Clicking directory path /home/user/projects opens system file manager at that location
  #   18. File link /path/file.ts is displayed in blue, URL https://example.com is displayed in cyan
  #   19. Clicking mailto:user@example.com opens system default email client
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to click on file paths and URLs in conversation messages
    So that I can quickly navigate to referenced files and web resources without manual copying

  # ===========================================
  # LINK DETECTION SCENARIOS
  # ===========================================
  @happy-path
  Scenario: Detect absolute file path in conversation message
    Given a conversation message contains "/home/user/project/src/file.ts"
    When the message is rendered in the conversation view
    Then the path "/home/user/project/src/file.ts" should be displayed as a clickable link

  @happy-path
  Scenario: Detect relative file path in conversation message
    Given a conversation message contains "./src/utils/helper.ts"
    When the message is rendered in the conversation view
    Then the path "./src/utils/helper.ts" should be displayed as a clickable link

  @happy-path
  Scenario: Detect HTTP URL in conversation message
    Given a conversation message contains "https://github.com/user/repo"
    When the message is rendered in the conversation view
    Then the URL "https://github.com/user/repo" should be displayed as a clickable link

  @happy-path
  Scenario: Detect file path with line and column notation
    Given a conversation message contains "/home/user/file.ts:42:10"
    When the message is rendered in the conversation view
    Then the path "/home/user/file.ts:42:10" should be displayed as a clickable link

  @happy-path
  @windows
  Scenario: Detect Windows file path in conversation message
    Given a conversation message contains "C:\Users\dev\project\file.ts:10:5"
    When the message is rendered in the conversation view
    Then the path "C:\Users\dev\project\file.ts:10:5" should be displayed as a clickable link

  @happy-path
  Scenario: Detect VS Code URI scheme in conversation message
    Given a conversation message contains "vscode://file/home/user/file.ts:42:10"
    When the message is rendered in the conversation view
    Then the URI "vscode://file/home/user/file.ts:42:10" should be displayed as a clickable link

  @integration
  Scenario: Open file at specific line and column
  # ===========================================
  # LINK OPENING SCENARIOS
  # ===========================================
    Given the user is viewing a conversation with file link "/home/user/file.ts:42:10"
    When the user clicks on the file link
    Then the file should open at line 42, column 10

  @integration
  @vscode
  Scenario: Open file in VS Code when running in VS Code terminal
    Given fspec is running in a VS Code integrated terminal
    And a conversation message contains a file link "/home/user/file.ts"
    When the user clicks on the file link
    Then the file should open in VS Code editor

  @integration
  Scenario: Open URL in system default browser
    Given a conversation message contains URL "https://docs.example.com/guide"
    When the user clicks on the URL link
    Then the URL should open in the user's default browser

  @integration
  Scenario: Open VS Code URI directly
    Given a conversation message contains "vscode://file/home/user/file.ts:42:10"
    When the user clicks on the VS Code URI
    Then the file should open in VS Code at line 42, column 10

  @integration
  Scenario: Open directory in file manager
    Given a conversation message contains directory path "/home/user/projects"
    When the user clicks on the directory link
    Then the system file manager should open at that location

  @integration
  Scenario: Open mailto link in email client
    Given a conversation message contains "mailto:user@example.com"
    When the user clicks on the mailto link
    Then the system default email client should open

  @error-handling
  Scenario: Show error for non-existent file path
  # ===========================================
  # ERROR HANDLING SCENARIOS
  # ===========================================
    Given a conversation message contains file link "/nonexistent/path/file.ts"
    And the file does not exist
    When the user clicks on the file link
    Then an error message "File not found" should be displayed

  @error-handling
  Scenario: Show error for invalid URL
    Given a conversation message contains URL "https://invalid.domain.notreal"
    When the user clicks on the URL link
    And the URL cannot be reached
    Then an error message "Unable to open URL" should be displayed

  @styling
  Scenario: File links displayed in blue color
  # ===========================================
  # VISUAL STYLING SCENARIOS
  # ===========================================
    Given a conversation message contains file path "/path/to/file.ts"
    When the message is rendered
    Then the file link should be displayed in blue color

  @styling
  Scenario: URL links displayed in cyan color
    Given a conversation message contains URL "https://example.com"
    When the message is rendered
    Then the URL link should be displayed in cyan color

  @styling
  Scenario: Display relative path instead of absolute path
    Given the working directory is "/home/user/projects/fspec"
    And a conversation message contains "/home/user/projects/fspec/src/file.ts"
    When the message is rendered
    Then the link should display as "src/file.ts"

  @styling
  Scenario: Show tooltip on link hover
    Given a conversation message contains relative path "./src/file.ts"
    When the user hovers over the link
    Then a tooltip should show the full resolved path

  @wrapping
  Scenario: Long URL preserves link boundary across line wrap
  # ===========================================
  # LINE WRAPPING SCENARIOS
  # ===========================================
    Given a conversation message contains "https://github.com/user/repo/blob/main/src/very/long/path/file.ts"
    And the terminal width requires the URL to wrap to the next line
    When the message is rendered
    Then the entire URL should remain a single clickable link

  @keyboard
  Scenario: Tab navigates between links
  # ===========================================
  # KEYBOARD NAVIGATION SCENARIOS
  # ===========================================
    Given a conversation message contains multiple links
    When the user presses Tab
    Then focus should move to the next link in the message

  @keyboard
  Scenario: Enter activates focused link
    Given a link in the conversation is focused
    When the user presses Enter
    Then the focused link should be activated

  @performance
  Scenario: Limit maximum clickable links per message
  # ===========================================
  # PERFORMANCE SCENARIOS
  # ===========================================
    Given the configuration has "links.maxLinksPerMessage" set to 50
    And a conversation message contains 100 URLs
    When the message is rendered
    Then only the first 50 URLs should be rendered as clickable links

  @component
  Scenario: Links work in AgentView component
  # ===========================================
  # COMPONENT COVERAGE SCENARIOS
  # ===========================================
    Given the user is viewing conversations in AgentView
    And a message contains file path "/path/to/file.ts"
    When the message is rendered
    Then the file path should be clickable in AgentView

  @component
  Scenario: Links work in SplitSessionView component
    Given the user is viewing conversations in SplitSessionView
    And a message contains file path "/path/to/file.ts"
    When the message is rendered
    Then the file path should be clickable in SplitSessionView

  @component
  Scenario: Links work in tool output messages
    Given a Bash tool output contains file path "/home/user/file.ts"
    When the tool output is rendered in the conversation view
    Then the file path should be displayed as a clickable link
