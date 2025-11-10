@research
@cli
@high
@RES-005
Feature: Confluence research tool integration
  """
  Uses Node.js script (#!/usr/bin/env node) placed in spec/research-scripts/confluence with executable bit set. Integrates with RES-002 research framework via auto-discovery. Calls Confluence REST API v2 with Basic authentication (username + API token). Config stored in user-level ~/.fspec/fspec-config.json under research.confluence with fields: confluenceUrl, username, apiToken. Supports CQL (Confluence Query Language) for advanced searching and full-text search. Multiple output formats: markdown (default with page summaries), json (structured array), text (plain content). Error handling: exit code 1 (missing/invalid args), 2 (config/auth errors), 3 (API/network errors). Query modes: --query (text search), --space (all pages in space), --page (single page by title).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Script must be auto-discoverable in spec/research-scripts/ with executable bit set
  #   2. Script must be standalone with CLI interface supporting flags: --query (full-text search), --space (space key), --page (page title), --help, --format (markdown/json/text)
  #   3. Confluence credentials (URL, username, API token) must be stored in user-level config ~/.fspec/fspec-config.json under research.confluence (NOT project-level)
  #   4. Script must support multiple output formats: markdown (default), json, text
  #   5. Script must provide comprehensive help text with --help flag showing usage, options, search examples, configuration, and exit codes
  #   6. Script must handle errors with proper exit codes: 1 (missing args/invalid input), 2 (config/auth errors), 3 (API/network errors)
  #   7. Script uses Confluence REST API v2 with Basic authentication (username + API token)
  #   8. Script must support CQL (Confluence Query Language) for advanced page searching and full-text search
  #
  # EXAMPLES:
  #   1. Developer runs 'confluence --query "API documentation"', receives markdown-formatted list of pages with titles, space keys, excerpts, and URLs
  #   2. Developer runs 'confluence --space DOCS --format json', receives JSON array of pages in the DOCS space with id, title, excerpt, url fields
  #   3. Developer runs 'confluence --page "Authentication Guide"', receives full page content in markdown format with title, body, last modified date
  #   4. Developer runs 'confluence --help', sees comprehensive help with usage, search examples, CQL syntax, config instructions, exit codes
  #   5. Developer runs 'confluence' without any flags, script exits with code 1 and error: 'Error: At least one of --query, --space, or --page is required'
  #   6. Developer runs confluence but ~/.fspec/fspec-config.json is missing Confluence config, script exits with code 2 and shows config setup instructions with confluenceUrl, username, apiToken fields
  #   7. Developer runs confluence with invalid API token, Confluence API returns 401 Unauthorized, script exits with code 2 and shows authentication error
  #   8. Developer runs 'confluence --page "NonExistent Page"', Confluence API returns 404 or empty results, script exits with code 3 and shows 'Page not found' error
  #   9. fspec research framework scans spec/research-scripts/, finds executable confluence file, auto-discovers it as available tool
  #   10. Developer runs 'fspec research --tool=confluence --query="OAuth implementation" during Example Mapping, framework executes script, prompts to attach results, saves to spec/attachments/WORK-001/confluence-oauth-docs-{date}.md
  #
  # ========================================
  Background: User Story
    As a AI agent or developer using fspec
    I want to search Confluence pages and documentation during Example Mapping
    So that I can reference existing documentation and knowledge base articles

  Scenario: Search pages with full-text query and markdown output
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid Confluence credentials
    When I run "confluence --query 'API documentation'"
    Then the script should exit with code 0
    And the output should be in markdown format
    And the output should list pages matching the search query
    And each page should show title, space key, excerpt, and URL

  Scenario: List all pages in a space with JSON output
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid Confluence credentials
    When I run "confluence --space DOCS --format json"
    Then the script should exit with code 0
    And the output should be valid JSON array
    And each JSON item should contain field "id"
    And each JSON item should contain field "title"
    And each JSON item should contain field "excerpt"
    And each JSON item should contain field "url"

  Scenario: Fetch single page by title with full content
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid Confluence credentials
    When I run "confluence --page 'Authentication Guide'"
    Then the script should exit with code 0
    And the output should be in markdown format
    And the output should contain the page title "Authentication Guide"
    And the output should contain the full page body content
    And the output should contain the last modified date

  Scenario: Display help documentation with CQL examples
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    When I run "confluence --help"
    Then the script should exit with code 0
    And the output should contain section "USAGE"
    And the output should contain section "OPTIONS"
    And the output should contain section "SEARCH EXAMPLES"
    And the output should contain section "CQL SYNTAX"
    And the output should contain section "CONFIGURATION"
    And the output should contain section "EXIT CODES"
    And the output should describe --query flag for full-text search
    And the output should describe --space flag for space search
    And the output should describe --page flag for page lookup by title
    And the output should describe --format flag with options: markdown, json, text

  Scenario: Error on missing required flags
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    When I run "confluence" without any search flags
    Then the script should exit with code 1
    And stderr should contain "Error: At least one of --query, --space, or --page is required"

  Scenario: Error on missing Confluence configuration
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json does not contain research.confluence section
    When I run "confluence --query 'test'"
    Then the script should exit with code 2
    And stderr should contain "Error: Confluence configuration not found"
    And stderr should contain setup instructions
    And stderr should show required config fields: confluenceUrl, username, apiToken

  Scenario: Error on invalid API token
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains invalid Confluence API token
    When I run "confluence --query 'test'"
    Then the script should exit with code 2
    And stderr should contain "Error: Confluence API authentication failed (HTTP 401)"
    And stderr should contain "Unauthorized" or "Invalid credentials"

  Scenario: Error on page not found
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid Confluence credentials
    When I run "confluence --page 'NonExistent Page'"
    And the page does not exist in Confluence
    Then the script should exit with code 3
    And stderr should contain "Error: Page not found"

  Scenario: Auto-discovery by fspec research framework
    Given the confluence script exists in spec/research-scripts/confluence
    And the file has executable permissions set
    When the fspec research framework scans spec/research-scripts/
    Then the framework should discover the confluence tool
    And the tool name should be derived as "confluence" from the filename
    And the tool should be listed in available research tools

  Scenario: Integration with fspec research command and attachment
    Given the confluence script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid Confluence credentials
    And work unit WORK-001 exists
    When I run "fspec research --tool=confluence --query='OAuth implementation'"
    Then the framework should execute the confluence script with the query
    And the script should return markdown-formatted results
    And the framework should prompt "Attach research results to work unit? (y/n)"
    When I respond with "y"
    Then the results should be saved to "spec/attachments/WORK-001/confluence-oauth-docs-{date}.md"
    And the attachment should be linked to work unit WORK-001
    And running "fspec show-work-unit WORK-001" should list the attachment
