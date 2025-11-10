@discovery
@high
@cli
@research
@integration
@RES-001
Feature: Interactive research command with multiple backend support

  """
  Integrates with RES-002 research framework for auto-discovery of research tools. Scans spec/research-scripts/ directory for executable files at runtime. Uses child_process.execSync to execute research scripts with arguments. Supports attachment workflow via fspec add-attachment command. Tool discovery is dynamic (no manifest file required).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command lists available research tools when run without arguments
  #   2. Auto-discover tools from spec/research-scripts/ directory by scanning for executable files
  #   3. Tool name derived from filename (e.g., perplexity → 'perplexity')
  #   4. Execute research tool with --tool and --query flags
  #   5. Prompt user to attach results to work unit after research completes
  #   6. Display tool usage examples when listing available tools
  #   7. Return error with helpful message if tool not found
  #   8. Save research results as attachment in spec/attachments/WORK-ID/ directory
  #   9. Support all research scripts: perplexity, jira, confluence, and future tools
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec research' without args, sees list of available tools with descriptions and usage examples
  #   2. Developer runs 'fspec research --tool=perplexity --query="How does OAuth2 work?"', perplexity script executes and returns formatted results
  #   3. Developer runs 'fspec research --tool=jira --issue AUTH-123', jira script fetches issue details
  #   4. After research completes, system prompts 'Attach results to work unit? (y/n)', user says yes, results saved to spec/attachments/AUTH-001/perplexity-oauth2-2025-11-08.md
  #   5. Developer runs 'fspec research --tool=confluence --page "Authentication Guide"', confluence script fetches page content
  #   6. Developer runs 'fspec research --tool=invalid', command returns error 'Error: Research tool not found: invalid'
  #   7. Developer adds new research script spec/research-scripts/github with executable permissions, runs 'fspec research', sees github in available tools list
  #   8. During Example Mapping, developer answers no to attachment prompt, research results displayed but not saved
  #
  # ========================================

  Background: User Story
    As a AI agent or developer using fspec
    I want to research questions interactively during Example Mapping
    So that I can choose from multiple research backends and get formatted results

  Scenario: List available research tools without arguments
    Given research tools exist in spec/research-scripts/ directory
    And the tools include perplexity, jira, and confluence
    When I run "fspec research" without arguments
    Then the output should list all available research tools
    And each tool should show its name and description
    And each tool should show usage examples

  Scenario: Execute perplexity research tool with query
    Given the perplexity research tool exists in spec/research-scripts/
    And the perplexity tool has executable permissions
    When I run "fspec research --tool=perplexity --query='How does OAuth2 work?'"
    Then the perplexity script should execute
    And the output should contain research results
    And the command should prompt to attach results to work unit

  Scenario: Execute jira research tool with issue key
    Given the jira research tool exists in spec/research-scripts/
    And the jira tool has executable permissions
    When I run "fspec research --tool=jira --issue AUTH-123"
    Then the jira script should execute with --issue flag
    And the output should contain issue details
    And the command should prompt to attach results to work unit

  Scenario: Attach research results to work unit
    Given the perplexity research tool exists in spec/research-scripts/
    And work unit AUTH-001 exists
    When I run "fspec research --tool=perplexity --query='OAuth2'"
    And I respond "y" to the attachment prompt
    Then the research results should be saved to spec/attachments/AUTH-001/
    And the attachment filename should include the tool name and date
    And the work unit should reference the attachment

  Scenario: Execute confluence research tool with page title
    Given the confluence research tool exists in spec/research-scripts/
    And the confluence tool has executable permissions
    When I run "fspec research --tool=confluence --page 'Authentication Guide'"
    Then the confluence script should execute with --page flag
    And the output should contain page content
    And the command should prompt to attach results to work unit

  Scenario: Error on invalid research tool
    Given no research tool named "invalid" exists
    When I run "fspec research --tool=invalid"
    Then the command should exit with error code 1
    And the output should contain "Error: Research tool not found: invalid"
    And the output should suggest running "fspec research" to see available tools

  Scenario: Auto-discover new research tool
    Given research tools exist in spec/research-scripts/
    When I add a new executable script spec/research-scripts/github
    And I run "fspec research" without arguments
    Then the output should include "github" in the available tools list
    And the github tool should show its name and description

  Scenario: Decline attachment prompt
    Given the perplexity research tool exists in spec/research-scripts/
    And work unit AUTH-001 exists
    When I run "fspec research --tool=perplexity --query='test'"
    And I respond "n" to the attachment prompt
    Then the research results should be displayed
    And no attachment should be saved
    And the work unit should not reference any new attachment
