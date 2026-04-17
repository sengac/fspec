@done
@context-window
@CTX-007
Feature: Per-Model Configurable Compaction Threshold
  """
  Add CompactionThresholdConfig enum (Tokens/Percentage) to compaction_threshold.rs with resolve(context_window) method
  Add compaction_threshold_config: Option<CompactionThresholdConfig> field to ProviderManager and a compaction_threshold() method that implements the priority chain
  stream_loop.rs changes from calculate_usable_context(context_window, max_output) to session.provider_manager().compaction_threshold() — one-line change, all downstream consumers automatically use the new value
  Model family detection uses ProviderManager::selected_model_info().family for registry models; for profile models (no registry) use model_id string prefix matching as fallback
  CTX-006 added context_window/max_output_tokens to SessionModel NAPI struct. CTX-007 will also expose compaction_threshold through the same mechanism for display in CTX-008
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Compaction threshold can be configured as either absolute tokens (e.g., 200000) or percentage of context window (e.g., 80%)
  #   2. Resolution priority: user-configured override > built-in model family default > legacy calculate_usable_context formula
  #   3. Built-in defaults: Claude family = context_window minus output reservation (legacy behavior retained); Gemini/OpenAI/others = 80% of context_window
  #   4. All compaction trigger paths (pre-prompt, CompactionHook, emergency, retry) must use the same resolved threshold
  #   5. Context fill percentage must be computed relative to the compaction threshold, not the raw context window
  #   6. If a user-configured threshold exceeds context_window, it should be clamped to context_window minus output reservation
  #   7. The ProviderManager gains a compaction_threshold() method that replaces direct calculate_usable_context() calls in the stream loop
  #   8. Post-compaction budget (summarization_budget) should be relative to the threshold, not the context_window
  #
  # EXAMPLES:
  #   1. Claude Sonnet 4 with 200k context → threshold = 200k - 8k = 191,808 (legacy behavior retained, no change)
  #   2. Gemini 2.5 Pro with 1M context → threshold = 800,000 (80% of 1M)
  #   3. GPT-4o with 128k context → threshold = 102,400 (80% of 128k)
  #   4. User sets absolute threshold of 150,000 tokens on a custom model with 200k context → compaction fires at 150k regardless of model family
  #   5. User sets percentage threshold of 60% on a 200k context model → compaction fires at 120,000
  #   6. Unknown model with 100k context and no family info → threshold falls through to legacy formula: 100k - 32k = 68,000
  #   7. User sets threshold of 300,000 on a model with 200k context → clamped to 200k - output_reservation (168,000)
  #   8. Context fill shows 50% when at 100k tokens with 200k threshold — not when at 500k tokens with 1M context
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have compaction trigger at a sensible threshold per model, independent of context window size
    So that compaction fires proactively before hitting the API limit, even when context_window and the practical compaction point differ

  @core
  @regression
  Scenario: Claude model retains legacy threshold behavior
    Given a Claude Sonnet 4 model with 200000 context window and 8192 max output
    And no user-configured compaction threshold override
    When the compaction threshold is resolved
    Then the threshold should equal 191808 tokens
    And the calculation should use context_window minus min(max_output, 32000)

  @core
  Scenario: Gemini model uses 80% built-in default
    Given a Gemini 2.5 Pro model with 1000000 context window
    And no user-configured compaction threshold override
    When the compaction threshold is resolved
    Then the threshold should equal 800000 tokens

  @core
  Scenario: OpenAI model uses 80% built-in default
    Given a GPT-4o model with 128000 context window
    And no user-configured compaction threshold override
    When the compaction threshold is resolved
    Then the threshold should equal 102400 tokens

  @core
  Scenario: User-configured absolute token threshold
    Given a custom model with 200000 context window
    And the user has configured a compaction threshold of 150000 tokens
    When the compaction threshold is resolved
    Then the threshold should equal 150000 tokens
    And the built-in model family default should be ignored

  @core
  Scenario: User-configured percentage threshold
    Given a model with 200000 context window
    And the user has configured a compaction threshold of 60 percent
    When the compaction threshold is resolved
    Then the threshold should equal 120000 tokens

  @core
  Scenario: Unknown model falls through to legacy formula
    Given an unknown model with 100000 context window and 0 max output
    And no model family information is available
    And no user-configured compaction threshold override
    When the compaction threshold is resolved
    Then the threshold should equal 68000 tokens
    And the legacy calculate_usable_context formula should be used

  @edge-case
  Scenario: User threshold exceeding context window is clamped
    Given a model with 200000 context window and 100000 max output
    And the user has configured a compaction threshold of 300000 tokens
    When the compaction threshold is resolved
    Then the threshold should be clamped to 168000 tokens
    And the clamped value should equal context_window minus output reservation

  @core
  Scenario: Context fill percentage uses compaction threshold
    Given a session with 200000 compaction threshold
    And the session has consumed 100000 tokens
    When the context fill percentage is calculated
    Then the fill percentage should be 50 percent
    And the percentage should be relative to the compaction threshold not the context window

  @integration
  Scenario: Stream loop uses ProviderManager compaction threshold
    Given a session with a configured ProviderManager
    When the agent stream loop starts
    Then the threshold should come from ProviderManager compaction_threshold method
    And all compaction trigger paths should use the same resolved threshold value

  @core
  Scenario: Threshold resolution priority chain
    Given a Claude model with 200000 context window
    And the user has configured a compaction threshold of 150000 tokens
    When the compaction threshold is resolved
    Then the user-configured threshold of 150000 should take priority
    And the built-in Claude family default should be ignored
    And the legacy formula should not be used
