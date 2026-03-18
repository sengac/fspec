@LOCATE-008
Feature: MCP Tool Definitions & Skill Documentation
  """
  Files: extension/webmcp-skill.md (main skill doc), extension/host/lib/mcp-server.mjs (NATIVE_TOOLS), extension/inject-webmcp-tools-skill.md (injection skill). MCP server already has tool definitions. Primary change is documentation updates to webmcp-skill.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. webmcp-skill.md header must show 14 native tools (not 12)
  #   2. browser_scan_page and browser_diff_page must have full tool documentation sections in webmcp-skill.md
  #   3. browser_click_element and browser_fill_form docs must mention @ref syntax (e.g. @e1) as an alternative to CSS selectors
  #   4. Common Workflows must include the scan→interact→verify workflow
  #   5. A Ref Lifecycle section must explain that refs are ephemeral and invalidated on navigation
  #   6. NATIVE_TOOLS in mcp-server.mjs must include inputSchema for browser_scan_page and browser_diff_page
  #   7. inject-webmcp-tools-skill.md must not reference old extension name or stale tool names
  #   8. Troubleshooting section must include ref-related errors (e.g. stale ref, ref not found)
  #
  # EXAMPLES:
  #   1. webmcp-skill.md shows 14 native tools after update, with browser_scan_page and browser_diff_page listed in the Native Browser Control Tools section
  #   2. browser_click_element docs show selector accepts CSS OR @ref (e.g. @e3 from browser_scan_page)
  #   3. Common Workflows section includes scan→interact→verify: navigate→scan→fill→click→diff→scan workflow
  #   4. Ref Lifecycle section explains refs are assigned by browser_scan_page, invalidated on navigation, and suggests re-scanning
  #   5. NATIVE_TOOLS in mcp-server.mjs already has both browser_scan_page and browser_diff_page with correct inputSchemas
  #   6. Troubleshooting includes: Ref not found error with suggestion to re-scan after page navigation
  #
  # ========================================
  Background: User Story
    As a AI agent developer
    I want to have accurate skill documentation for the browser scan, diff, and ref tools
    So that my AI agents can effectively use the scan→interact→verify workflow without guesswork

  Scenario: Skill documentation lists all 14 native tools
    Given the webmcp-skill.md file exists
    When I read the header section
    Then it should state 14 native browser control tools
    And browser_scan_page and browser_diff_page should be listed in the Native Browser Control Tools section

  Scenario: Click and fill tools document ref syntax
    Given the webmcp-skill.md file exists
    When I read the browser_click_element and browser_fill_form documentation
    Then the selector parameter should mention accepting @ref syntax from browser_scan_page

  Scenario: Common workflows include scan-interact-verify pattern
    Given the webmcp-skill.md file exists
    When I read the Common Workflows section
    Then it should include a workflow showing navigate, scan, fill, click, diff, and re-scan steps

  Scenario: Ref lifecycle documentation
    Given the webmcp-skill.md file exists
    When I read the Ref Lifecycle section
    Then it should explain that refs are assigned by browser_scan_page and are ephemeral
    And it should state that refs are invalidated on page navigation

  Scenario: MCP server NATIVE_TOOLS includes scan and diff tools
    Given the mcp-server.mjs file exists
    When I inspect the NATIVE_TOOLS array
    Then it should contain browser_scan_page with tabId, interactive, and selector properties
    And it should contain browser_diff_page with tabId property

  Scenario: Troubleshooting covers ref-related errors
    Given the webmcp-skill.md file exists
    When I read the Troubleshooting section
    Then it should include guidance for ref not found errors suggesting to re-scan

  Scenario: Inject skill file has no stale references
    Given the inject-webmcp-tools-skill.md file exists
    When I search for old extension name references
    Then no references to old tool names or old extension names should be found
