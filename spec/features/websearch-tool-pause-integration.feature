@BUG-582
@pause-integration
@codelet
@PAUSE-001
@BUG-556
Feature: WebSearch Tool Pause Integration
  """
  WebSearchTool integration: Add pause field to OpenPage, FindInPage, CaptureScreenshot
  variants in rust/common/src/web_search.rs. Call pause_for_user() in
  rust/tools/src/web_search.rs when pause=true.

  Pause implies a visible browser: pause: true auto-implies headless: false (pausing a headless browser is pointless).
  """

  Background: User Story
    As a developer debugging web interactions
    I want to pause the browser after page load
    So that I can inspect the page and DevTools before the tool returns results

  Scenario: OpenPage with pause shows visible browser
    Given the agent calls WebSearchTool OpenPage on "https://example.com" with pause set to true
    When the tool prepares the page action
    Then the browser runs visible
    And the pause handler is asked to continue a "WebSearch" pause saying "Page loaded: https://example.com"
    When the user presses Enter
    Then the tool goes on to return the page content

  Scenario: Pause with headless true auto-overrides to visible
    Given the agent calls OpenPage with pause true and headless true
    When the tool prepares the page action
    Then headless should be automatically overridden to false
    And the browser runs visible

  Scenario: CaptureScreenshot with pause allows interaction
    Given the agent calls CaptureScreenshot on "https://example.com" with pause set to true
    When the tool prepares the page action
    Then the browser runs visible
    And the pause invites the user to interact with the page and press Enter to capture the screenshot
    When the user presses Enter
    Then the tool goes on to capture the screenshot
