@high
@cli-interface
@provider-management
@providers
@CLI-001
Feature: Provider Manager for Dynamic Provider Selection

  """
  ProviderManager class handles credential detection and provider instantiation. Credentials read from env vars (ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_GENERATIVE_AI_API_KEY) and auth files (~/.codex/auth.json, ~/.claude/auth.json). Returns Box<dyn LlmProvider> trait object for polymorphic provider usage. Default selection follows priority: Claude API > Claude OAuth > Gemini > Codex > OpenAI. CLI --provider flag overrides auto-selection.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Default provider selected based on credential availability priority: Claude API key > Claude Code OAuth > Gemini > Codex OAuth > OpenAI
  #   2. CLI --provider flag overrides automatic provider selection
  #   3. Error must be thrown if no provider credentials are available
  #   4. Error must be thrown if requested provider has no credentials with helpful message showing available providers
  #   5. Credentials detected from environment variables (ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_GENERATIVE_AI_API_KEY) and auth files (~/.codex/auth.json, ~/.claude/auth.json)
  #
  # EXAMPLES:
  #   1. User has ANTHROPIC_API_KEY set, runs 'codelet "Hello"', uses Claude provider automatically
  #   2. User has ~/.codex/auth.json, runs 'codelet --provider codex "Hello"', uses Codex provider and responds with Codex model identity
  #   3. User has no credentials, runs 'codelet "Hello"', receives error 'No provider credentials found. Set ANTHROPIC_API_KEY or run codex auth login'
  #   4. User has ANTHROPIC_API_KEY only, runs 'codelet --provider openai "Hello"', receives error 'Provider openai not available. Available providers: claude'
  #   5. User has both ANTHROPIC_API_KEY and ~/.codex/auth.json, runs 'codelet "Hello"' without --provider flag, uses Claude (higher priority)
  #
  # ========================================

  Background: User Story
    As a developer using codelet CLI
    I want to select which LLM provider to use (Claude, OpenAI, Codex, Gemini)
    So that I can choose the best provider for my task and avoid vendor lock-in

  Scenario: Automatic Claude provider selection with API key
    Given the ANTHROPIC_API_KEY environment variable is set
    And no other provider credentials are available
    When I run codelet with prompt "Who made you?"
    Then the response should come from Claude provider
    And the response should mention "Anthropic"

  Scenario: Explicit Codex provider selection via CLI flag
    Given the ~/.codex/auth.json file exists with valid credentials
    When I run codelet with --provider codex and prompt "Who made you?"
    Then the response should come from Codex provider
    And the response should mention "OpenAI"

  Scenario: Error when no credentials available
    Given no provider credentials are set
    And no auth files exist
    When I run codelet with prompt "Hello"
    Then I should receive an error
    And the error message should contain "No provider credentials found"
    And the error message should suggest "Set ANTHROPIC_API_KEY or run codex auth login"

  Scenario: Error when requested provider unavailable
    Given only ANTHROPIC_API_KEY is set
    When I run codelet with --provider openai and prompt "Hello"
    Then I should receive an error
    And the error message should contain "Provider openai not available"
    And the error message should list available providers as "claude"

  Scenario: Priority-based provider selection
    Given both ANTHROPIC_API_KEY and ~/.codex/auth.json exist
    When I run codelet without --provider flag and prompt "Who made you?"
    Then the response should come from Claude provider
    And the Codex provider should not be used

  Scenario: Claude Code OAuth fallback
    Given no ANTHROPIC_API_KEY is set
    And CLAUDE_CODE_OAUTH_TOKEN environment variable is set
    And no other provider credentials are available
    When I run codelet with prompt "Who made you?"
    Then the response should come from Claude provider via OAuth
    And the response should mention "Anthropic"

  Scenario: Gemini provider selection
    Given GOOGLE_GENERATIVE_AI_API_KEY environment variable is set
    When I run codelet with --provider gemini and prompt "Hello"
    Then the response should come from Gemini provider

  Scenario: OpenAI provider selection
    Given OPENAI_API_KEY environment variable is set
    When I run codelet with --provider openai and prompt "Hello"
    Then the response should come from OpenAI provider
