@done
@BUG-493
@providers
@claude
@streaming
@provider-abstraction
@headers
@rust
Feature: Refactor providers to use rig with streaming
  """
  ClaudeProvider (rust/providers/src/claude.rs) is built on rig's Anthropic
  client, keeps the LlmProvider interface and reports streaming support. Its
  anthropic-beta header comes from build_beta_headers: OAuth mode adds
  claude-code-20250219 and oauth-2025-04-20, prompt caching is always on, and
  interleaved-thinking-2025-05-14 is added for every model that
  is_adaptive_thinking_model (rust/tools/src/facade/thinking_config.rs) does
  not treat as adaptive. Since BUG-438 (claude-budget-only-thinking-models)
  the dated Claude 4.0 ids such as claude-sonnet-4-20250514 and Claude Sonnet
  4.5 are budget-only, so they get the interleaved-thinking header.
  The tests came with the codelet REFAC-003 provider refactor, before the
  repository rename (ff53a2e38), and named this feature before it existed;
  BUG-493 wrote it. Each scenario states what its test checks: the tests
  build providers and read their properties and never stream a response.
  Tests: rust/core/tests/rig_provider_streaming_test.rs.
  """

  Background: User Story
    As a developer maintaining the Claude provider
    I want to have the rig-based Claude provider keep its interface, streaming support and decided beta headers
    So that a provider or beta-header regression fails the core suite

  Scenario: Claude provider is created from an API key and a model
    Given an API key and the model "claude-sonnet-4-20250514"
    When a Claude provider is created from them
    Then the provider is created
    And it reports the name "claude" and the model "claude-sonnet-4-20250514"

  Scenario: An API-key Claude provider supports streaming
    Given a Claude provider created from an API key and the model "claude-sonnet-4-20250514"
    When its streaming support is checked
    Then it supports streaming

  Scenario: A Claude provider supports streaming alongside a Read tool definition
    Given a Claude provider created from an API key and the model "claude-sonnet-4-20250514"
    And a Read tool definition with a required file_path
    When its streaming support is checked
    Then it supports streaming

  Scenario: An OAuth Claude provider for a budget-only model sends the OAuth and interleaved-thinking beta headers
    Given an OAuth token and the budget-only model "claude-sonnet-4-20250514"
    When a Claude provider is created from them in OAuth mode
    Then the provider is created in OAuth mode
    And it supports streaming
    And its anthropic-beta header contains "oauth-2025-04-20"
    And its anthropic-beta header contains "interleaved-thinking-2025-05-14"

  Scenario: A Claude Sonnet 4.5 provider sends the interleaved-thinking beta header
    Given a Claude provider created from an API key and the model "claude-sonnet-4-5"
    When its streaming support and anthropic-beta header are checked
    Then it supports streaming
    And its anthropic-beta header contains "interleaved-thinking-2025-05-14"

  Scenario: The Claude provider keeps the LlmProvider interface and declared limits
    Given a Claude provider created from an API key and the model "claude-sonnet-4-20250514"
    When its LlmProvider properties are read
    Then it reports the name "claude", a 200000-token context window and 8192 max output tokens
    And it supports prompt caching
