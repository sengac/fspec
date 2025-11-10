@research
@cli
@high
@RES-004
Feature: JIRA research tool integration
  """
  Uses Node.js script (#!/usr/bin/env node) placed in spec/research-scripts/jira with executable bit set. Integrates with RES-002 research framework via auto-discovery. Calls JIRA REST API v2/v3 with Basic authentication (username + API token). Config stored in user-level ~/.fspec/fspec-config.json under research.jira with fields: jiraUrl, username, apiToken. Supports JQL (JIRA Query Language) for advanced filtering. Multiple output formats: markdown (default with issue details), json (structured array), text (plain summaries). Error handling: exit code 1 (missing/invalid args), 2 (config/auth errors), 3 (API/network errors). Query modes: --issue (single issue), --project (all project issues), --query (JQL query).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Script must be auto-discoverable in spec/research-scripts/ with executable bit set
  #   2. Script must be standalone with CLI interface supporting flags: --query (JQL query), --issue (issue key), --project (project key), --help, --format (markdown/json/text)
  #   3. JIRA credentials (URL, username, API token) must be stored in user-level config ~/.fspec/fspec-config.json under research.jira (NOT project-level)
  #   4. Script must support multiple output formats: markdown (default), json, text
  #   5. Script must provide comprehensive help text with --help flag showing usage, options, JQL examples, configuration, and exit codes
  #   6. Script must handle errors with proper exit codes: 1 (missing args/invalid input), 2 (config/auth errors), 3 (API/network errors)
  #   7. Script uses JIRA REST API v2 or v3 with Basic authentication (username + API token)
  #   8. Script must support JQL (JIRA Query Language) for advanced issue filtering
  #
  # EXAMPLES:
  #   1. Developer runs 'jira --issue AUTH-123', receives markdown-formatted issue details with key, summary, description, status, assignee, labels
  #   2. Developer runs 'jira --project MYPROJ --format json', receives JSON array of issues in the project with key, summary, status fields
  #   3. Developer runs 'jira --query "project = AUTH AND status = Open"', receives issues matching JQL query in markdown format
  #   4. Developer runs 'jira --help', sees comprehensive help with usage, JQL examples, config instructions, exit codes
  #   5. Developer runs 'jira' without any flags, script exits with code 1 and error: 'Error: At least one of --query, --issue, or --project is required'
  #   6. Developer runs jira but ~/.fspec/fspec-config.json is missing JIRA config, script exits with code 2 and shows config setup instructions with jiraUrl, username, apiToken fields
  #   7. Developer runs jira with invalid API token, JIRA API returns 401 Unauthorized, script exits with code 2 and shows authentication error
  #   8. Developer runs 'jira --issue INVALID-999', JIRA API returns 404 Not Found, script exits with code 3 and shows 'Issue not found' error
  #   9. fspec research framework scans spec/research-scripts/, finds executable jira file, auto-discovers it as available tool
  #   10. Developer runs 'fspec research --tool=jira --query="project = AUTH" during Example Mapping, framework executes script, prompts to attach results, saves to spec/attachments/WORK-001/jira-auth-issues-{date}.md
  #
  # ========================================
  Background: User Story
    As a AI agent or developer using fspec
    I want to search JIRA issues and tickets during Example Mapping
    So that I can reference existing work items and requirements from JIRA

  Scenario: Fetch single issue by key with markdown output
    Given the jira script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid JIRA credentials
    When I run "jira --issue AUTH-123"
    Then the script should exit with code 0
    And the output should be in markdown format
    And the output should contain the issue key "AUTH-123"
    And the output should contain the issue summary
    And the output should contain the issue description
    And the output should contain the issue status
    And the output should contain the issue assignee
    And the output should contain the issue labels

  Scenario: Search project issues with JSON output
    Given the jira script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid JIRA credentials
    When I run "jira --project MYPROJ --format json"
    Then the script should exit with code 0
    And the output should be valid JSON array
    And each JSON item should contain field "key"
    And each JSON item should contain field "summary"
    And each JSON item should contain field "status"

  Scenario: Search issues using JQL query
    Given the jira script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid JIRA credentials
    When I run "jira --query 'project = AUTH AND status = Open'"
    Then the script should exit with code 0
    And the output should be in markdown format
    And the output should list issues matching the JQL query
    And each issue should show key, summary, and status

  Scenario: Display help documentation with JQL examples
    Given the jira script exists in spec/research-scripts/ with executable permissions
    When I run "jira --help"
    Then the script should exit with code 0
    And the output should contain section "USAGE"
    And the output should contain section "OPTIONS"
    And the output should contain section "JQL EXAMPLES"
    And the output should contain section "CONFIGURATION"
    And the output should contain section "EXIT CODES"
    And the output should describe --issue flag for single issue lookup
    And the output should describe --project flag for project search
    And the output should describe --query flag for JQL queries
    And the output should describe --format flag with options: markdown, json, text

  Scenario: Error on missing required flags
    Given the jira script exists in spec/research-scripts/ with executable permissions
    When I run "jira" without any search flags
    Then the script should exit with code 1
    And stderr should contain "Error: At least one of --query, --issue, or --project is required"

  Scenario: Error on missing JIRA configuration
    Given the jira script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json does not contain research.jira section
    When I run "jira --issue TEST-1"
    Then the script should exit with code 2
    And stderr should contain "Error: JIRA configuration not found"
    And stderr should contain setup instructions
    And stderr should show required config fields: jiraUrl, username, apiToken

  Scenario: Error on invalid API token
    Given the jira script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains invalid JIRA API token
    When I run "jira --issue TEST-1"
    Then the script should exit with code 2
    And stderr should contain "Error: JIRA API authentication failed (HTTP 401)"
    And stderr should contain "Unauthorized" or "Invalid credentials"

  Scenario: Error on issue not found
    Given the jira script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid JIRA credentials
    When I run "jira --issue INVALID-999"
    And the issue does not exist in JIRA
    Then the script should exit with code 3
    And stderr should contain "Error: JIRA API request failed (HTTP 404)"
    And stderr should contain "Issue not found"

  Scenario: Auto-discovery by fspec research framework
    Given the jira script exists in spec/research-scripts/jira
    And the file has executable permissions set
    When the fspec research framework scans spec/research-scripts/
    Then the framework should discover the jira tool
    And the tool name should be derived as "jira" from the filename
    And the tool should be listed in available research tools

  Scenario: Integration with fspec research command and attachment
    Given the jira script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains valid JIRA credentials
    And work unit WORK-001 exists
    When I run "fspec research --tool=jira --query='project = AUTH'"
    Then the framework should execute the jira script with the JQL query
    And the script should return markdown-formatted results
    And the framework should prompt "Attach research results to work unit? (y/n)"
    When I respond with "y"
    Then the results should be saved to "spec/attachments/WORK-001/jira-auth-issues-{date}.md"
    And the attachment should be linked to work unit WORK-001
    And running "fspec show-work-unit WORK-001" should list the attachment
