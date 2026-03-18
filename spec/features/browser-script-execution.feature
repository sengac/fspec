@EXT-010
Feature: browser_execute_script returns null — eval() blocked by CSP in extension isolated world
  """
  Replace eval()-based execution in browser-tools.ts with chrome.userScripts.execute() using USER_SCRIPT world. Add ChromeUserScriptsForTools interface to deps. Call configureWorld() in service-worker.ts on startup. Add userScripts permission to manifest.json.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. chrome.scripting.executeScript with eval() is blocked by MV3 CSP in the extension's ISOLATED world — eval() silently returns null
  #   2. chrome.userScripts API (Chrome 120+ configureWorld, Chrome 135+ execute) provides a USER_SCRIPT world exempt from page CSP and configurable with custom CSP
  #   3. userScripts permission and configureWorld() must be called on service worker startup before any script execution
  #   4. When userScripts API is unavailable (user hasn't enabled toggle), the handler must return a clear error message explaining the requirement
  #   5. The USER_SCRIPT world CSP must include 'unsafe-eval' and 'unsafe-inline' to allow eval() and dynamic code execution
  #   6. Script execution errors must be caught and returned as MCP error results, not silently swallowed
  #
  # EXAMPLES:
  #   1. Agent executes 'document.title' and receives the actual page title instead of null
  #   2. Agent executes '1 + 1' and receives '2'
  #   3. Agent executes invalid JS like 'throw new Error("test")' and receives an MCP error result with the error message
  #   4. When userScripts API is unavailable, agent receives an error explaining how to enable it (toggle 'Allow User Scripts')
  #   5. Service worker startup calls configureWorld to set permissive CSP for USER_SCRIPT world
  #   6. manifest.json includes userScripts permission
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to execute arbitrary JavaScript in a browser tab via browser_execute_script
    So that I can extract data, check state, and manipulate the DOM beyond what specialized tools offer

  Scenario: Execute script that returns a string value
    Given the USER_SCRIPT world is configured with permissive CSP
    And a tab is open with a web page
    When I call browser_execute_script with code "document.title"
    Then I should receive a text result containing the page title
    And the result should not be "null"

  Scenario: Execute script that returns an expression result
    Given the USER_SCRIPT world is configured with permissive CSP
    And a tab is open with a web page
    When I call browser_execute_script with code "1 + 1"
    Then I should receive a text result containing "2"

  Scenario: Execute script that throws an error
    Given the USER_SCRIPT world is configured with permissive CSP
    And a tab is open with a web page
    When I call browser_execute_script with code that throws an error
    Then I should receive an MCP error result
    And the error message should contain the thrown error details

  Scenario: Execute script when userScripts API is unavailable
    Given the userScripts API is not available
    And a tab is open with a web page
    When I call browser_execute_script with any code
    Then I should receive an MCP error result
    And the error message should explain how to enable user scripts

  Scenario: Configure USER_SCRIPT world on service worker startup
    Given the extension service worker is starting
    When the userScripts API is available
    Then configureWorld should be called with a CSP that allows unsafe-eval
    And configureWorld should be called with a CSP that allows unsafe-inline

  Scenario: Manifest includes userScripts permission
    Given the extension manifest.json file
    Then it should include the "userScripts" permission
