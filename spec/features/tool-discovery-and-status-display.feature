@done
@high
@cli
@research-tools
@tool-discovery
@RES-010
Feature: Tool Discovery and Status Display

  """
  Uses config resolution system from RES-012 (resolveConfig, validateConfig). Integrates with existing research tool registry. Shows configuration status (CONFIGURED, NOT CONFIGURED, PARTIALLY CONFIGURED) using visual indicators. Provides usage guidance with configuration command examples for unconfigured tools. Output is structured for both human and AI agent consumption.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool listing command must show all registered research tools with their names and descriptions
  #   2. Each tool must display configuration status: CONFIGURED (ready), NOT CONFIGURED (missing required config), or PARTIALLY CONFIGURED (some optional config missing)
  #   3. Configuration status must use the config resolution system from RES-012 to check all config sources (ENV, User, Project, Defaults)
  #   4. Tool listing must show which configuration source is being used for configured tools (ENV, USER, PROJECT, DEFAULT)
  #   5. For unconfigured tools, output must show usage guidance with configuration command examples
  #   6. Tool listing output must be AI-agent friendly (structured, clear status indicators, machine-parseable)
  #
  # EXAMPLES:
  #   1. User runs 'fspec research' and sees Perplexity tool with status CONFIGURED (source: ENV) because PERPLEXITY_API_KEY is set
  #   2. User runs 'fspec research' and sees Jira tool with status NOT CONFIGURED along with command example: export JIRA_URL=... JIRA_TOKEN=...
  #   3. User runs 'fspec research' and sees Confluence tool with status CONFIGURED (source: USER) showing config from ~/.fspec/fspec-config.json
  #   4. User runs 'fspec research' and sees all tools listed with descriptions, status indicators (✓ or ✗), and config sources
  #   5. User runs 'fspec research --tool=perplexity --help' and sees tool-specific help with configuration requirements and usage examples
  #
  # ========================================

  Background: User Story
    As a developer using research tools
    I want to see configuration status and usage guidance for all available tools
    So that I can quickly understand which tools are ready to use and how to configure them

  Scenario: Show tool with CONFIGURED status from environment variable
    Given I have PERPLEXITY_API_KEY environment variable set
    When I run "fspec research"
    Then the output should list Perplexity tool
    And the status should be "CONFIGURED"
    And the config source should be "ENV"


  Scenario: Show tool with NOT CONFIGURED status and usage guidance
    Given I have no JIRA_URL or JIRA_TOKEN configured
    When I run "fspec research"
    Then the output should list Jira tool
    And the status should be "NOT CONFIGURED"
    And the output should show configuration command examples
    And the examples should include "export JIRA_URL=..."
    And the examples should include "export JIRA_TOKEN=..."


  Scenario: Show tool with CONFIGURED status from user config file
    Given I have Confluence configuration in ~/.fspec/fspec-config.json
    When I run "fspec research"
    Then the output should list Confluence tool
    And the status should be "CONFIGURED"
    And the config source should be "USER"


  Scenario: List all tools with status indicators and config sources
    Given I have mixed tool configurations (some configured, some not)
    When I run "fspec research"
    Then the output should list all registered research tools
    And each tool should show a description
    And each tool should show a status indicator (✓ or ✗)
    And configured tools should show their config source


  Scenario: Show tool-specific help with configuration requirements
    Given I want to learn how to configure a specific tool
    When I run "fspec research --tool=perplexity --help"
    Then the output should show Perplexity tool description
    And the output should show configuration requirements
    And the output should show usage examples

