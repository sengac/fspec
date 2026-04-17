@done
@LIMITS-004
Feature: Fix ProviderManager Resolution Chain — Use ModelLimitsResolver
  """
  ProviderManager must resolve context_window() and max_output_tokens() through
  the ModelLimitsResolver trait so that provider hard limits clamp any registry
  or user-override values. This prevents models.dev reporting 1M context for
  Claude Opus 4.6 when the API only accepts 200k.
  """

  Background: User Story
    As a developer using any provider
    I want to have context_window() and max_output_tokens() return provider-clamped values
    So that models.dev registry data never exceeds what the provider's API actually accepts

  @critical
  Scenario: Claude context window is clamped from 1M registry to 200k
    Given a ProviderManager configured for the Claude provider
    And select_model stores a registry context window of 1000000
    And the Claude resolver declares max_context_window as 200000
    When context_window() is called
    Then the result should be 200000

  @critical
  Scenario: Claude max output tokens clamped from 128k registry to 8192
    Given a ProviderManager configured for the Claude provider
    And select_model stores a registry max output of 128000
    And the Claude resolver declares max_output_tokens_limit as 8192
    When max_output_tokens() is called
    Then the result should be 8192

  Scenario: OpenAI context window passes through unclamped
    Given a ProviderManager configured for the OpenAI provider
    And select_model stores a registry context window of 128000
    And the OpenAI resolver declares max_context_window as None
    When context_window() is called
    Then the result should be 128000

  Scenario: Codex with no registry data returns provider default
    Given a ProviderManager configured for the Codex provider
    And no registry context window is set
    And no user context window override is set
    When context_window() is called
    Then the result should be 272000

  Scenario: User override is clamped by provider max
    Given a ProviderManager configured for the Claude provider
    And the user overrides context window to 500000 via NAPI
    And the Claude resolver declares max_context_window as 200000
    When context_window() is called
    Then the result should be 200000

  Scenario: User override takes priority over registry value
    Given a ProviderManager configured for the OpenAI provider
    And select_model stores a registry context window of 128000
    And the user overrides context window to 64000 via NAPI
    When context_window() is called
    Then the result should be 64000

  Scenario: Sub-agent propagation returns clamped values
    Given a ProviderManager configured for the Claude provider
    And select_model stores a registry context window of 1000000
    When raw_model_context_window() is called for sub-agent propagation
    Then the result should be 200000
    And it should equal the value from context_window()

  Scenario: OpenAI env var fallback when no registry data
    Given a ProviderManager configured for the OpenAI provider
    And no registry context window is set
    And no user context window override is set
    And the OPENAI_CONTEXT_WINDOW environment variable is set to 256000
    When context_window() is called
    Then the result should be 256000
