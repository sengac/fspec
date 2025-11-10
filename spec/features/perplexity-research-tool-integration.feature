@research
@cli
@high
@RES-003
Feature: Perplexity research tool integration
  """
  Uses Node.js script (#!/usr/bin/env node) placed in spec/research-scripts/perplexity with executable bit set. Integrates with RES-002 research framework via auto-discovery (framework scans directory for executables). Calls Perplexity API POST /chat/completions endpoint with Bearer token authentication. Config stored in user-level ~/.fspec/fspec-config.json under research.perplexity.apiKey. Supports multiple output formats: markdown (default with metadata), json (structured), text (plain answer). Error handling: exit code 1 (missing args), 2 (config errors), 3 (API/network errors). Default model: llama-3.1-sonar-small-128k-online (overridable via --model flag).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Script must be auto-discoverable in spec/research-scripts/ with executable bit set (framework scans for any executable file, not just .sh)
  #   2. Script must be standalone with its own CLI interface supporting flags: --query (required), --help, --format (markdown/json/text), --model
  #   3. API key must be stored in user-level config ~/.fspec/fspec-config.json under research.perplexity.apiKey (NOT project-level)
  #   4. Script must support multiple output formats: markdown (default), json, text
  #   5. Script must provide comprehensive help text with --help flag showing usage, options, examples, configuration, and exit codes
  #   6. Script must handle errors with proper exit codes: 1 (missing args), 2 (config errors), 3 (API/network errors)
  #   7. Script uses Perplexity API POST /chat/completions endpoint with Bearer token authentication
  #   8. Default model is llama-3.1-sonar-small-128k-online (overridable via --model flag or config)
  #
  # EXAMPLES:
  #   1. Developer runs 'perplexity --query "How does OAuth2 work?"', receives markdown-formatted research results with source, date, answer, and token usage
  #   2. Developer runs 'perplexity --query "What is BDD?" --format json', receives JSON output with query, source, model, timestamp, answer, and usage fields
  #   3. Developer runs 'perplexity --query "Explain ACDD" --format text', receives plain text answer without metadata
  #   4. Developer runs 'perplexity --help', sees comprehensive help text with usage, options, examples, configuration instructions, and exit codes
  #   5. Developer runs 'perplexity' without --query flag, script exits with code 1 and error message: 'Error: Missing required flag --query'
  #   6. Developer runs perplexity but ~/.fspec/fspec-config.json doesn't exist, script exits with code 2 and shows config setup instructions
  #   7. Developer runs perplexity with invalid API key, Perplexity API returns 401, script exits with code 3 and shows API error details
  #   8. Developer hits rate limit, Perplexity API returns 429, script exits with code 3 and shows 'Rate limit exceeded. Retry after 60s' message
  #   9. fspec research framework scans spec/research-scripts/, finds executable perplexity file, auto-discovers it as available tool
  #   10. Developer runs 'fspec research --tool=perplexity --query "OAuth2 token refresh"', framework executes script and prompts 'Attach results to work unit? (y/n)', results saved to spec/attachments/AUTH-001/perplexity-oauth2-refresh-2025-11-07.md
  #
  # ========================================
  Background: User Story
    As a AI agent or developer using fspec
    I want to research questions using Perplexity during Example Mapping
    So that I get real-time AI-powered search results to inform acceptance criteria

  Scenario: Research with default markdown output
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains a valid Perplexity API key
    When I run "perplexity --query 'How does OAuth2 work?'"
    Then the script should exit with code 0
    And the output should be in markdown format
    And the output should contain a title with the query text
    And the output should contain source metadata "Perplexity AI"
    And the output should contain a timestamp
    And the output should contain the answer content
    And the output should contain token usage statistics

  Scenario: Research with JSON output format
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains a valid Perplexity API key
    When I run "perplexity --query 'What is BDD?' --format json"
    Then the script should exit with code 0
    And the output should be valid JSON
    And the JSON should contain field "query" with value "What is BDD?"
    And the JSON should contain field "source" with value "Perplexity AI"
    And the JSON should contain field "model"
    And the JSON should contain field "timestamp"
    And the JSON should contain field "answer"
    And the JSON should contain field "usage" with promptTokens, completionTokens, totalTokens

  Scenario: Research with text output format
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains a valid Perplexity API key
    When I run "perplexity --query 'Explain ACDD' --format text"
    Then the script should exit with code 0
    And the output should be plain text
    And the output should contain only the answer content
    And the output should not contain metadata or formatting

  Scenario: Display help documentation
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    When I run "perplexity --help"
    Then the script should exit with code 0
    And the output should contain section "USAGE"
    And the output should contain section "OPTIONS"
    And the output should contain section "EXAMPLES"
    And the output should contain section "CONFIGURATION"
    And the output should contain section "EXIT CODES"
    And the output should describe the --query flag as required
    And the output should describe the --format flag with options: markdown, json, text
    And the output should describe the --model flag with default model

  Scenario: Error on missing required query flag
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    When I run "perplexity" without the --query flag
    Then the script should exit with code 1
    And stderr should contain "Error: Missing required flag --query"

  Scenario: Error on missing configuration file
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json does not exist
    When I run "perplexity --query 'test query'"
    Then the script should exit with code 2
    And stderr should contain "Error: Config file not found"
    And stderr should contain setup instructions with mkdir command
    And stderr should contain example config structure with apiKey field

  Scenario: Error on invalid API key
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains an invalid Perplexity API key
    When I run "perplexity --query 'test query'"
    Then the script should exit with code 3
    And stderr should contain "Error: Perplexity API request failed (HTTP 401)"
    And stderr should contain "Invalid API key" or "Unauthorized"

  Scenario: Error on rate limit exceeded
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains a valid Perplexity API key
    And the Perplexity API rate limit has been exceeded
    When I run "perplexity --query 'test query'"
    Then the script should exit with code 3
    And stderr should contain "Error: Perplexity API request failed (HTTP 429)"
    And stderr should contain "Rate limit exceeded"
    And stderr should suggest "Retry after 60s" or similar retry guidance

  Scenario: Auto-discovery by fspec research framework
    Given the perplexity script exists in spec/research-scripts/perplexity
    And the file has executable permissions set
    When the fspec research framework scans spec/research-scripts/
    Then the framework should discover the perplexity tool
    And the tool name should be derived as "perplexity" from the filename
    And the tool should be listed in available research tools

  Scenario: Integration with fspec research command and attachment
    Given the perplexity script exists in spec/research-scripts/ with executable permissions
    And ~/.fspec/fspec-config.json contains a valid Perplexity API key
    And work unit AUTH-001 exists
    When I run "fspec research --tool=perplexity --query 'OAuth2 token refresh'"
    Then the framework should execute the perplexity script with the query
    And the script should return markdown-formatted results
    And the framework should prompt "Attach research results to work unit? (y/n)"
    When I respond with "y"
    Then the results should be saved to "spec/attachments/AUTH-001/perplexity-oauth2-refresh-{date}.md"
    And the attachment should be linked to work unit AUTH-001
    And running "fspec show-work-unit AUTH-001" should list the attachment
