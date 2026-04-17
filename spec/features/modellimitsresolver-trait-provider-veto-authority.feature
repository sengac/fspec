@done
@rust
@context-window
@provider-abstraction
@providers
@LIMITS-002
Feature: ModelLimitsResolver Trait — Provider Veto Authority
  """
  The resolve_model_limits function must be a standalone pure function in model_limits.rs — not a method on ProviderManager — for testability and separation of concerns
  The trait extends LlmProvider (or is composed with it), not a separate trait hierarchy — providers implement one trait that covers both runtime operations and limits
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The trait must provide max_context_window() returning Option<usize> — providers return Some(limit) to clamp, None to trust registry
  #   2. The trait must provide max_output_tokens_limit() returning Option<usize> — same semantics as max_context_window()
  #   3. The trait must provide default_context_window() and default_max_output_tokens() — used when no registry data exists
  #   4. A standalone resolve_model_limits(registry_value, user_override, resolver) function must implement the priority chain: user_override → clamp by provider max → registry → clamp by provider max → default
  #   5. The trait must live in codelet/providers/src/ as a new file (model_limits.rs) — it is part of the provider abstraction, not the CLI
  #   6. The trait should optionally provide should_send_max_output_tokens() -> bool (default true) — Codex needs to return false since its API rejects max_output_tokens
  #   7. Env var overrides (OPENAI_CONTEXT_WINDOW, etc.) must be handled inside the resolver, not scattered across provider constructors and manager fallbacks
  #
  # EXAMPLES:
  #   1. Claude resolver with max_context_window=Some(200_000): registry says 1M → resolve returns 200k (clamped)
  #   2. OpenAI resolver with max_context_window=None: registry says 128k → resolve returns 128k (trusted)
  #   3. User override of 500k on Claude (max 200k): resolve returns 200k (user override clamped by provider max)
  #   4. No registry data, no user override for Codex: resolve returns 272k (provider default)
  #   5. OPENAI_CONTEXT_WINDOW env var set to 256000: OpenAI resolver uses this as the env-derived default, overriding compile-time 128k
  #   6. Codex resolver returns should_send_max_output_tokens()=false — API rejects this parameter
  #
  # ========================================
  Background: User Story
    As a provider implementor
    I want to declare hard limits and defaults for context window and max output tokens
    So that the resolution system clamps external registry values to what the provider's API actually supports

  @clamping
  Scenario: Provider clamps registry value to its hard maximum
    Given a provider declares max_context_window as 200000
    And the registry reports a context window of 1000000
    And no user override is set
    When the model limits are resolved
    Then the resolved context window should be 200000

  @trust
  Scenario: Provider trusts registry value when no max is declared
    Given a provider declares max_context_window as None
    And the registry reports a context window of 128000
    And no user override is set
    When the model limits are resolved
    Then the resolved context window should be 128000

  @user-override
  @clamping
  Scenario: User override is clamped by provider max
    Given a provider declares max_context_window as 200000
    And no registry value is available
    And the user override is set to 500000
    When the model limits are resolved
    Then the resolved context window should be 200000

  @user-override
  @trust
  Scenario: User override is trusted when provider declares no max
    Given a provider declares max_context_window as None
    And no registry value is available
    And the user override is set to 100000
    When the model limits are resolved
    Then the resolved context window should be 100000

  @defaults
  Scenario: Provider default is used when no registry or user override exists
    Given a provider declares default_context_window as 272000
    And no registry value is available
    And no user override is set
    When the model limits are resolved
    Then the resolved context window should be 272000

  @output-tokens
  Scenario: Provider can suppress sending max_output_tokens
    Given a provider declares should_send_max_output_tokens as false
    Then the resolver should indicate max_output_tokens must not be sent

  @output-tokens
  @clamping
  Scenario: Max output tokens are clamped by provider limit
    Given a provider declares max_output_tokens_limit as 8192
    And the registry reports max output tokens of 128000
    And no user override is set
    When the model output limits are resolved
    Then the resolved max output tokens should be 8192

  @output-tokens
  @defaults
  Scenario: Provider default max output tokens used when no registry data
    Given a provider declares default_max_output_tokens as 4096
    And no registry value is available
    And no user override is set
    When the model output limits are resolved
    Then the resolved max output tokens should be 4096
