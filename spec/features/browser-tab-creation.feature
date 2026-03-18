@EXT-011
Feature: Add browser_create_tab tool to fspec Browser Agent Chrome Extension
  """
  Uses chrome.tabs.create() API. Chromium C++ implementation: TabsCreateFunction in chrome/browser/extensions/api/tabs/tabs_api.h. API schema: chrome/common/extensions/api/tabs.json. Helper: ExtensionTabUtil::OpenTab in chrome/browser/extensions/extension_tab_util.h.
  Changes in 2 files only: extension/src/background/browser-tools.ts (handler + interface) and extension/host/lib/mcp-server.mjs (NATIVE_TOOLS entry). Plus docs: extension/webmcp-skill.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. chrome.tabs.create() requires NO additional permissions - creating a tab is permission-free per Chrome docs
  #   2. The handler must add 'create' to the ChromeTabsForTools interface for dependency injection
  #   3. When a URL is provided, the handler must wait for the tab to complete loading before returning (using waitForTabLoad)
  #   4. The MCP server NATIVE_TOOLS array must include the browser_create_tab tool definition with JSON Schema
  #   5. The tool must accept optional parameters: url, active, windowId, pinned
  #   6. Return value must include tabId, url, title, active, and windowId
  #   7. The webmcp-skill.md documentation must be updated with the new tool
  #
  # EXAMPLES:
  #   1. Agent calls browser_create_tab with {url: 'https://example.com'} → new tab opens, navigates to URL, returns {tabId, url, title, active: true, windowId}
  #   2. Agent calls browser_create_tab with no arguments → new blank tab opens (chrome://newtab), returns immediately with {tabId, url: '', title: '', active: true}
  #   3. Agent calls browser_create_tab with {url: 'https://example.com', active: false} → tab opens in background, returns {active: false}
  #   4. Agent calls browser_create_tab with {url: 'https://example.com', pinned: true} → pinned tab created
  #   5. Tool appears in tools/list response as 12th native tool alongside existing 11
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to create a new browser tab, optionally navigating to a URL
    So that I can open web pages without replacing the current tab's content

  Scenario: Create a new tab with a URL
    Given the browser tools are initialized with a mock tabs API
    When I call browser_create_tab with url "https://example.com"
    Then the handler should call tabs.create with url "https://example.com"
    And the handler should wait for the tab to finish loading
    And the result should contain tabId, url, title, active, and windowId

  Scenario: Create a new tab without a URL
    Given the browser tools are initialized with a mock tabs API
    When I call browser_create_tab with no arguments
    Then the handler should call tabs.create with an empty properties object
    And the result should return immediately without waiting for load
    And the result should contain tabId, url, title, active, and windowId

  Scenario: Create a background tab
    Given the browser tools are initialized with a mock tabs API
    When I call browser_create_tab with url "https://example.com" and active false
    Then the handler should call tabs.create with active set to false
    And the result should show active as false

  Scenario: Create a pinned tab
    Given the browser tools are initialized with a mock tabs API
    When I call browser_create_tab with url "https://example.com" and pinned true
    Then the handler should call tabs.create with pinned set to true

  Scenario: Tool is registered in the handler map
    Given the browser tools are initialized with a mock tabs API
    Then the tool names should include "browser_create_tab"

  Scenario: ChromeTabsForTools interface includes create method
    Given the browser tools source code
    Then the ChromeTabsForTools interface should declare a create method

  Scenario: MCP server NATIVE_TOOLS includes browser_create_tab
    Given the MCP server source code
    Then the NATIVE_TOOLS array should contain a tool named "browser_create_tab"
    And the tool schema should have optional properties url, active, windowId, and pinned

  Scenario: Skill documentation includes browser_create_tab
    Given the webmcp-skill.md documentation file
    Then the documentation should reference browser_create_tab with its parameters and return value
    And the tool count should be updated from 11 to 12
