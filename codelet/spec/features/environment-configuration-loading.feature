@CORE-006
Feature: Environment Configuration Loading

  """
  Uses dotenvy crate (0.15.7) for .env file parsing. Called in main.rs before any provider initialization. Silently ignores missing .env files. Does NOT override existing environment variables (dotenvy default behavior).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. MUST load .env file from current working directory on startup
  #   2. MUST silently ignore if .env file does not exist
  #   3. MUST NOT override existing environment variables
  #   4. SHOULD use dotenvy crate for .env parsing
  #
  # EXAMPLES:
  #   1. User has .env with CLAUDE_CODE_OAUTH_TOKEN, runs codelet, agent authenticates successfully
  #   2. User has no .env file, runs codelet with ANTHROPIC_API_KEY already exported, agent works
  #   3. User has .env with API key but also exports different key, exported key takes precedence
  #
  # ========================================

  Background: User Story
    As a developer
    I want to have API keys loaded automatically from .env files
    So that I don't have to manually export environment variables every time

  Scenario: Load API key from .env file on startup
    Given a .env file exists with CLAUDE_CODE_OAUTH_TOKEN set
    And no environment variables are exported
    When the user runs codelet
    Then the agent should authenticate using the token from .env

  Scenario: Run without .env file using exported environment variable
    Given no .env file exists in the current directory
    And ANTHROPIC_API_KEY is exported in the shell
    When the user runs codelet
    Then the agent should authenticate using the exported API key

  Scenario: Exported environment variable takes precedence over .env
    Given a .env file exists with ANTHROPIC_API_KEY=key-from-env-file
    And ANTHROPIC_API_KEY=key-from-shell is exported
    When the user runs codelet
    Then the agent should use key-from-shell, not key-from-env-file

