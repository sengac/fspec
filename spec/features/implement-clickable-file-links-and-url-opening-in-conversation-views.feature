@navigation
@virtuallist
@tui
@LINK-001
Feature: Implement Clickable File Links and URL Opening in Conversation Views
  """
  Create linkUtils.ts with regex patterns for detecting file paths and URLs
  Create LinkifiedText.tsx component for rendering clickable text segments in Ink.js
  Update conversationUtils.ts to parse links during line wrapping
  Create linkHandlers.ts for VS Code integration and URL opening
  Detect VS Code environment via VSCODE_PID, TERM_PROGRAM environment variables
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
  #
  # EXAMPLES:
  #   1. Absolute path /home/user/file.ts is detected and becomes clickable
  #   2. Relative path ./src/file.ts is detected and becomes clickable
  #   3. URL https://github.com/user/repo is detected and becomes clickable
  #   4. File path /home/user/file.ts:42:10 opens file at line 42, column 10
  #   5. Clicking file link in VS Code terminal opens file in VS Code editor
  #   6. Clicking URL opens in user's default browser
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to click on file paths and URLs in conversation messages
    So that I can quickly navigate to referenced files and web resources without manual copying

  # Link Detection Scenarios
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

  # Link Opening Scenarios
  @integration
  Scenario: Open file at specific line and column
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

  # Component Coverage Scenarios
  @component
  Scenario: Links work in AgentView component
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
