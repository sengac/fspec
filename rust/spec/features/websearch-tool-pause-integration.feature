@PAUSE-001
Feature: WebSearch Tool Pause Integration
  """
  WebSearchTool integration with tool pause mechanism.
  Adds pause parameter to OpenPage, CaptureScreenshot, FindInPage.
  pause: true auto-implies headless: false.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pause is triggered explicitly via pause: true parameter
  #   2. pause: true auto-implies headless: false (pausing headless is pointless)
  #   3. Do not auto-open Chrome DevTools - user opens manually if needed
  #   4. Initial scope: WebSearchTool only (rust-headless-chrome)
  #
  # EXAMPLES:
  #   1. OpenPage with pause: true → browser visible, waits for Enter
  #   2. CaptureScreenshot with pause: true → user can interact before capture
  #   3. pause: true + headless: true → headless auto-overridden to false
  #
  # ========================================

  Background: WebSearchTool with pause support
    Given the agent is processing a request
    And WebSearchTool supports pause parameter on OpenPage, CaptureScreenshot, and FindInPage

  Scenario: OpenPage with pause shows visible browser and waits for user
    When the agent calls WebSearchTool OpenPage with url "https://example.com" and pause set to true
    Then the browser window should be visible to the user
    And the page should be fully loaded
    And the session status should be "paused"
    And the TUI should show "WebSearch paused: Page loaded" with "(Press Enter to continue)"
    When the user presses Enter
    Then the tool should extract the page content
    And the session status should return to "running"
    And the tool should return the extracted content to the agent

  Scenario: Pause with headless true auto-overrides to visible browser
    When the agent calls WebSearchTool OpenPage with url "https://example.com" and pause set to true and headless set to true
    Then headless should be automatically overridden to false
    And the browser window should be visible to the user

  Scenario: CaptureScreenshot with pause allows user interaction before capture
    When the agent calls WebSearchTool CaptureScreenshot with url "https://example.com" and pause set to true
    Then the browser window should be visible to the user
    And the session status should be "paused"
    And the user can interact with the page and scroll
    When the user presses Enter
    Then the screenshot should capture the current state of the page
    And the tool should return the screenshot path to the agent
