@done
@high
@cli
@research-tools
@config-management
@RES-012
Feature: Configuration Management with TUI Integration
  """
  Phase 1 implementation: Multi-layer config resolution and validation only. Uses dotenv for .env file loading. Config resolution follows priority: ENV vars (highest) → User config → Project config → Defaults (lowest). Validation checks required fields before tool execution to prevent runtime errors.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Configuration sources are checked in priority order: ENV vars → User config (~/.fspec/fspec-config.json) → Project config (spec/fspec-config.json) → Defaults
  #   2. Environment variables take precedence over all config files (e.g., PERPLEXITY_API_KEY overrides config file settings)
  #   3. User-level config (~/.fspec/fspec-config.json) stores personal API keys and is never version controlled
  #   4. Project-level config (spec/fspec-config.json) stores team defaults and URLs but no secrets (can be version controlled)
  #   5. Config validation must check for required fields before tool execution
  #   6. .env file support must load environment variables from project root .env file if it exists
  #
  # EXAMPLES:
  #   1. Perplexity API key set via PERPLEXITY_API_KEY env var overrides user config file setting
  #   2. Jira URL in project config (spec/fspec-config.json) is used when JIRA_URL env var is not set and user config doesn't exist
  #   3. .env file in project root loads PERPLEXITY_API_KEY=pplx-abc123 which then takes precedence over config files
  #   4. Validation detects missing CONFLUENCE_TOKEN before tool execution and shows clear error message
  #   5. Default Perplexity model 'sonar' is used when no PERPLEXITY_MODEL env var or config file setting exists
  #
  # ========================================
  Background: User Story
    As a developer using research tools
    I want to configure and validate research tool settings without manual file editing
    So that I can quickly set up tools with confidence and see configuration status at a glance

  Scenario: Environment variable overrides user config file
    Given I have Perplexity API key "user-key" in ~/.fspec/fspec-config.json
    And I have PERPLEXITY_API_KEY environment variable set to "env-key"
    When the config resolution system loads Perplexity configuration
    Then the API key should be "env-key"
    And the config source should be "ENV"

  Scenario: Project config used when env var and user config don't exist
    Given I have no JIRA_URL environment variable set
    And I have no ~/.fspec/fspec-config.json file
    And I have Jira URL "https://company.atlassian.net" in spec/fspec-config.json
    When the config resolution system loads Jira configuration
    Then the Jira URL should be "https://company.atlassian.net"
    And the config source should be "PROJECT"

  Scenario: .env file loads environment variables with precedence over config files
    Given I have a .env file in project root with PERPLEXITY_API_KEY=pplx-abc123
    And I have Perplexity API key "user-key" in ~/.fspec/fspec-config.json
    When the config resolution system loads with dotenv support
    Then the API key should be "pplx-abc123"
    And the config source should be "ENV"

  Scenario: Validation detects missing required configuration
    Given I have no CONFLUENCE_TOKEN environment variable set
    And I have no Confluence configuration in any config file
    When I attempt to use the Confluence research tool
    Then validation should fail with error "Missing required configuration: CONFLUENCE_TOKEN"
    And the error message should suggest how to configure the tool

  Scenario: Default values used when no configuration exists
    Given I have no PERPLEXITY_MODEL environment variable set
    And I have no Perplexity model setting in any config file
    When the config resolution system loads Perplexity configuration
    Then the model should be "sonar"
    And the config source should be "DEFAULT"
