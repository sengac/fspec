@pause-integration
@codelet
@PAUSE-001
Feature: WebSearch Tool Pause Integration

  """
  WebSearchTool integration: Add pause field to OpenPage, FindInPage, CaptureScreenshot
  variants in codelet/common/src/web_search.rs. Call pause_for_user() in
  codelet/tools/src/web_search.rs when pause=true.
  Rule: pause: true auto-implies headless: false (pausing a headless browser is pointless)
  """

  Background: User Story
    As a developer debugging web interactions
    I want to pause the browser after page load
    So that I can inspect the page and DevTools before the tool returns results

  Scenario: OpenPage with pause shows visible browser
    Given the agent is processing a request
    When the agent calls WebSearchTool OpenPage with pause set to true
    Then the browser window should be visible
    And the session status should be "paused"
    And the pause state should contain tool name "WebSearch"
    When the user presses Enter
    Then the tool should return the page content

  Scenario: Pause with headless true auto-overrides to visible
    Given the agent calls OpenPage with pause true and headless true
    When the tool processes the request
    Then headless should be automatically overridden to false
    And the browser window should be visible

  Scenario: CaptureScreenshot with pause allows interaction
    Given the agent calls CaptureScreenshot with pause set to true
    When the tool navigates to the page
    Then the user can interact with the page
    When the user presses Enter
    Then the screenshot captures the current state
