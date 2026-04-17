@done
@LIMITS-003
@provider
@model-limits
Feature: Provider Model Limits Resolution
  """
  Each provider implements the ModelLimitsResolver trait in its own existing file
  (claude.rs, openai.rs, gemini.rs, codex/mod.rs, zai.rs, copilot/provider.rs).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Claude: max_context_window=Some(200_000), max_output_tokens_limit=Some(8_192), defaults=200k/8k — until CONFIG-007 opt-in
  #   2. OpenAI: max_context_window=None (trusts registry), defaults=128k/4k, reads OPENAI_CONTEXT_WINDOW and OPENAI_MAX_OUTPUT_TOKENS env vars as default overrides
  #   3. Gemini: max_context_window=None (trusts registry), defaults=1M/8k — API genuinely supports advertised limits
  #   4. Codex: max_context_window=None, defaults=272k/4k, should_send_max_output_tokens()=false since API rejects it
  #   5. Z.AI: max_context_window=None, defaults=128k/8k — straightforward implementation
  #   6. Copilot: max_context_window=None, defaults=200k/4k — real limits come from live /models endpoint via registry
  #   7. Each provider implementation must have unit tests verifying clamp behavior, default values, and should_send_max_output_tokens
  #
  # EXAMPLES:
  #   1. Claude selects claude-opus-4-6 (registry: 1M/128k) → resolver clamps to 200k/8k
  #   2. Claude selects claude-sonnet-4 (registry: 200k/8k) → resolver returns 200k/8k (no clamp needed, values agree)
  #   3. OpenAI selects gpt-4o (registry: 128k/16k) → resolver returns 128k/16k (trusted)
  #   4. Codex with no registry data → resolver returns 272k/4k defaults, should_send_max_output=false
  #   5. OpenAI with OPENAI_CONTEXT_WINDOW=256000 set → resolver default becomes 256k instead of 128k
  #
  # ========================================
  Background: User Story
    Given the system maintains six LLM providers
    And each provider implements the ModelLimitsResolver trait

  # -- Claude Provider -------------------------------------------------------
  Scenario: Claude resolver clamps registry context window to 200k
    Given the Claude provider's resolver declares max_context_window as 200000
    When the registry reports a context window of 1000000
    Then the resolved context window should be clamped to 200000

  Scenario: Claude resolver clamps registry max output tokens to 8192
    Given the Claude provider's resolver declares max_output_tokens_limit as 8192
    When the registry reports max output tokens of 128000
    Then the resolved max output tokens should be clamped to 8192

  Scenario: Claude resolver returns correct defaults
    Given the Claude provider's resolver is queried with no registry or user data
    Then the default context window should be 200000
    And the default max output tokens should be 8192
    And should_send_max_output_tokens should be true

  # -- OpenAI Provider -------------------------------------------------------
  Scenario: OpenAI resolver trusts registry values
    Given the OpenAI provider's resolver declares max_context_window as None
    When the registry reports a context window of 128000 and max output tokens of 16384
    Then the resolved context window should be 128000
    And the resolved max output tokens should be 16384

  Scenario: OpenAI resolver reads default from OPENAI_CONTEXT_WINDOW env var
    Given the OPENAI_CONTEXT_WINDOW environment variable is set to 256000
    When the OpenAI resolver is queried with no registry data
    Then the default context window should be 256000

  Scenario: OpenAI resolver reads default from OPENAI_MAX_OUTPUT_TOKENS env var
    Given the OPENAI_MAX_OUTPUT_TOKENS environment variable is set to 16384
    When the OpenAI resolver is queried with no registry data
    Then the default max output tokens should be 16384

  Scenario: OpenAI resolver falls back to compile-time defaults
    Given no OPENAI_CONTEXT_WINDOW or OPENAI_MAX_OUTPUT_TOKENS env vars are set
    When the OpenAI resolver is queried with no registry data
    Then the default context window should be 128000
    And the default max output tokens should be 4096
    And should_send_max_output_tokens should be true

  # -- Gemini Provider -------------------------------------------------------
  Scenario: Gemini resolver trusts registry and has correct defaults
    Given the Gemini provider's resolver declares max_context_window as None
    Then the default context window should be 1000000
    And the default max output tokens should be 8192
    And should_send_max_output_tokens should be true

  # -- Codex Provider --------------------------------------------------------
  Scenario: Codex resolver returns correct defaults and suppresses max_output_tokens
    Given the Codex provider's resolver is queried
    Then the default context window should be 272000
    And the default max output tokens should be 4096
    And should_send_max_output_tokens should be false

  Scenario: Codex resolver does not clamp registry values
    Given the Codex provider's resolver declares max_context_window as None
    When the registry reports a context window of 300000
    Then the resolved context window should be 300000

  # -- Z.AI Provider ---------------------------------------------------------
  Scenario: Z.AI resolver returns correct defaults
    Given the Z.AI provider's resolver is queried
    Then the default context window should be 128000
    And the default max output tokens should be 8192
    And should_send_max_output_tokens should be true

  # -- Copilot Provider ------------------------------------------------------
  Scenario: Copilot resolver returns correct defaults
    Given the Copilot provider's resolver is queried
    Then the default context window should be 200000
    And the default max output tokens should be 4096
    And should_send_max_output_tokens should be true
