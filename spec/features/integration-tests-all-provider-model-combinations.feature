@context-window
@providers
@provider-abstraction
@rust
@done
@LIMITS-007
Feature: Integration Tests — All Provider/Model Combinations

  """
  Tests use Rust resolve_model_limits and resolve_context_window / resolve_max_output_tokens functions against real provider structs. ProviderManager.context_window() and max_output_tokens() resolve through ModelLimitsResolver. Claude clamps to 200k/8192, others trust registry.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Unit tests for resolve_model_limits() with each provider's resolver
  #   2. Unit tests for resolve_compaction_threshold() with corrected inputs from each provider
  #   3. Integration tests for session creation → NAPI → badge display chain for each provider
  #   4. Edge case tests: user override > provider max → clamped, zero values → defaults, missing registry data → defaults
  #   5. Sub-agent propagation tests: DeepSearch inherits clamped values, AgentManager subordinates inherit clamped values
  #   6. Existing CTX-006/007/008/009 tests must be updated with correct expected values and re-verified
  #   7. Every provider/model combination must have assertions for context_window, max_output_tokens, and compaction threshold
  #   8. resolve_model_limits with each provider's resolver must verify clamping vs pass-through behavior
  #   9. ProviderManager full chain: select_model → context_window() → compaction threshold for every provider
  #   10. Edge cases: user override > provider max → clamped, zero → default, no registry → default
  #   11. TypeScript tests must have correct expected values reflecting provider-clamped outputs (200k not 1M for Claude)
  #
  # EXAMPLES:
  #   1. Claude claude-sonnet-4: context=200k, max_output=8192, threshold=191808
  #   2. Claude claude-opus-4-6: context=200k (clamped from 1M), max_output=8192 (clamped from 128k), threshold=191808
  #   3. OpenAI gpt-4o: context=128k, max_output=16384, threshold=102400 (80%)
  #   4. Gemini gemini-2.5-pro: context=1M, max_output=8192, threshold=800k (80%)
  #   5. Codex default: context=272k, max_output=4096, threshold=217600 (80%)
  #   6. Z.AI glm-4-plus: context=128k, max_output=8192, threshold=102400 (80%)
  #   7. Copilot fallback: context=200k, max_output=4096, threshold=160000 (80%)
  #   8. User override 500k on Claude → clamped to 200k
  #
  # ========================================

  Background: User Story
    As a developer
    I want to run comprehensive integration tests covering every provider/model combination
    So that verify the full model limits resolution chain produces correct context_window, max_output_tokens, and compaction_threshold values for all providers

  Scenario: Claude Sonnet 4 resolves context window to 200k and max output to 8192
    Given the Claude provider resolver with max_context_window 200000 and max_output_tokens_limit 8192
    When resolve_context_window is called with registry value 200000 and no user override
    Then context_window should be 200000 and max_output_tokens should be 8192


  Scenario: Claude Opus 4.6 clamps 1M registry to 200k and 128k output to 8192
    Given the Claude provider resolver with max_context_window 200000 and max_output_tokens_limit 8192
    When resolve_context_window is called with registry value 1000000 and no user override
    Then context_window should be 200000 and max_output_tokens should be 8192


  Scenario: OpenAI gpt-4o trusts registry values without clamping
    Given the OpenAI provider resolver with no max_context_window and default 128000
    When resolve_context_window is called with registry value 128000
    Then context_window should be 128000 and max_output_tokens should be 16384


  Scenario: Gemini 2.5 Pro trusts 1M registry value and uses 80% threshold
    Given the Gemini provider resolver with no max_context_window and default 1000000
    When resolve_context_window is called with registry value 1000000
    Then context_window should be 1000000 and max_output_tokens should be 8192


  Scenario: Codex falls back to 272k default context window with no registry
    Given the Codex provider resolver with no max_context_window and default 272000
    When resolve_context_window is called with no registry value and no user override
    Then context_window should be 272000 and max_output_tokens should be 4096


  Scenario: Z.AI resolves 128k context window and 8192 max output
    Given the Z.AI provider resolver with no max_context_window and default 128000
    When resolve_context_window is called with no registry value
    Then context_window should be 128000 and max_output_tokens should be 8192


  Scenario: Copilot falls back to 200k default context and 4096 max output
    Given the Copilot provider resolver with no max_context_window and default 200000
    When resolve_context_window is called with no registry value
    Then context_window should be 200000 and max_output_tokens should be 4096


  Scenario: User override exceeding provider max is clamped
    Given the Claude provider resolver with max_context_window 200000
    When a user override of 500000 is applied
    Then context_window should be clamped to 200000


  Scenario: ProviderManager full chain resolves clamped values for all providers
    Given a ProviderManager configured for each provider type
    When context_window() and max_output_tokens() are called with registry values
    Then each provider returns correctly clamped or pass-through values


  Scenario: Sub-agent propagation returns clamped values for Claude
    Given a ProviderManager with Claude and registry context_window of 1000000
    When raw_model_context_window() is called for sub-agent propagation
    Then the returned value should be 200000 not 1000000

