@done
@infrastructure
@configuration
@file-ops
@phase-1
@CONFIG-001
Feature: Shared configuration management utilities for user and project config files
  """
  Error handling: Missing/empty files return {} (silent fallback). Invalid JSON throws error with helpful message (fail fast). Utilities: loadConfig() for reading+merging, writeConfig() for writing to specific scope.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. JSON format for both user-level (~/.fspec/fspec-config.json) and project-level (spec/fspec-config.json)
  #   2. Project-level config overrides user-level config (for project-specific overrides)
  #   3. Deep merge - merge nested objects recursively. This preserves user defaults while allowing project-specific overrides without duplication.
  #   4. Keep it generic - accept any JSON structure and let different features define their own config namespaces. No schema enforcement at the config utility level.
  #   5. File doesn't exist or is empty: return empty object {} (silent fallback). Invalid JSON syntax: throw error with helpful message (fail fast). Be lenient with absence, strict with mistakes.
  #
  # EXAMPLES:
  #   1. User has no config files. Call loadConfig() returns empty object {}. No errors thrown.
  #   2. User has ~/.fspec/fspec-config.json with {"timeout": 60}. Project has no config. loadConfig() returns {"timeout": 60}.
  #   3. User has {"research": {"timeout": 60, "tools": ["perplexity"]}}. Project has {"research": {"tools": ["jira"]}}. loadConfig() returns {"research": {"timeout": 60, "tools": ["jira"]}} via deep merge.
  #   4. Project config has invalid JSON syntax (missing bracket). loadConfig() throws error with message: 'Invalid JSON in spec/fspec-config.json: Unexpected token...'.
  #   5. Developer calls writeConfig('user', {"newSetting": true}) and it writes to ~/.fspec/fspec-config.json with proper formatting.
  #
  # QUESTIONS (ANSWERED):
  #   Q: What format should the config files use? Should both user-level and project-level use the same format?
  #   A: true
  #
  #   Q: When both user-level and project-level config files have the same setting, which takes precedence?
  #   A: true
  #
  #   Q: How should nested config objects be merged? Deep merge (recursive) or shallow merge (replace entire objects)?
  #   A: true
  #
  #   Q: What types of settings should the config support? Specific schemas or generic JSON structure?
  #   A: true
  #
  #   Q: What should happen when config file doesn't exist, has invalid JSON, or is empty?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to manage configuration across user-level and project-level scopes
    So that I can set personal defaults and project-specific overrides without duplication

  Scenario: Load config when no config files exist
    Given neither user-level nor project-level config files exist
    When I call loadConfig()
    Then it should return an empty object {}
    And no errors should be thrown

  Scenario: Load config from user-level only
    Given user-level config at ~/.fspec/fspec-config.json contains {"timeout": 60}
    And project-level config does not exist
    When I call loadConfig()
    Then it should return {"timeout": 60}

  Scenario: Deep merge user-level and project-level config
    Given user-level config contains {"research": {"timeout": 60, "tools": ["perplexity"]}}
    And project-level config contains {"research": {"tools": ["jira"]}}
    When I call loadConfig()
    Then it should return {"research": {"timeout": 60, "tools": ["jira"]}}
    And the timeout setting from user-level should be preserved
    And the tools setting from project-level should override user-level

  Scenario: Throw error on invalid JSON syntax
    Given project-level config has invalid JSON syntax (missing bracket)
    When I call loadConfig()
    Then it should throw an error
    And the error message should contain "Invalid JSON in spec/fspec-config.json"
    And the error message should contain "Unexpected token"

  Scenario: Write config to user-level scope
    Given I want to save personal defaults
    When I call writeConfig('user', {"newSetting": true})
    Then it should write to ~/.fspec/fspec-config.json
    And the file should contain properly formatted JSON
    And the file should be readable as valid JSON
