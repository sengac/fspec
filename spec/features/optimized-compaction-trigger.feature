@done
@CTX-002 @cli @context-window
Feature: Optimized Compaction Window Limit Trigger

  """
  Key architectural decisions:
  - Implements optimized isOverflow() algorithm for context compaction
  - Called after each assistant response to check if compaction is needed
  - Short-circuit evaluation: disable flag → zero context → calculation (in that order)

  Dependencies and integrations:
  - codelet/core/src/compaction_hook.rs - Replace calculate_effective_tokens() with simple sum
  - codelet/cli/src/compaction_threshold.rs - Add SESSION_OUTPUT_TOKEN_MAX constant and should_trigger_compaction()
  - codelet/cli/src/compaction_threshold.rs - Deprecate calculate_compaction_threshold() (keep for backwards compat)
  - codelet/providers/src/lib.rs - Add max_output_tokens() to Provider trait
  - codelet/providers/src/claude.rs - Return model-specific max_output (varies by model variant)
  - codelet/providers/src/openai.rs - Return model-specific max_output (varies by model variant)
  - codelet/providers/src/gemini.rs - Return model-specific max_output (varies by model variant)

  Algorithm (must follow this exact order):
  1. If disable_autocompact flag is set → return false (no compaction)
  2. If context_window == 0 → return false (no context limit means no compaction)
  3. Calculate: token_count = input_tokens + cache_read_tokens + output_tokens
  4. Calculate: output_reservation = min(model_max_output, SESSION_OUTPUT_TOKEN_MAX)
  5. If output_reservation == 0 → output_reservation = SESSION_OUTPUT_TOKEN_MAX (fallback)
  6. Calculate: usable_context = context_window - output_reservation
  7. Return: token_count > usable_context (strictly greater than, not >=)

  Constants:
  - SESSION_OUTPUT_TOKEN_MAX = 32,000 tokens (reasonable upper bound for output reservation)

  Type considerations (Rust):
  - All token counts are u64
  - Use saturating_sub for usable_context calculation to prevent underflow
  - max_output_tokens() returns u64 (0 means unknown/use fallback)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Token count = input_tokens + cache_read_tokens + output_tokens (simple sum, no discounting)
  #   2. Output reservation = min(model_max_output, SESSION_OUTPUT_TOKEN_MAX), fallback to 32k if result is 0
  #   3. Usable context = context_window - output_reservation
  #   4. Trigger compaction when: token_count > usable_context (strictly greater than)
  #   5. SESSION_OUTPUT_TOKEN_MAX = 32,000 tokens (constant)
  #   6. Short-circuit: disable flag checked FIRST, then context==0, then calculation
  #   7. Provider trait MUST expose max_output_tokens() returning model-specific limit (0 if unknown)
  #   8. Each model variant may have different max_output (e.g., Claude Opus vs Sonnet)
  #
  # EXAMPLES:
  #   1. Claude Sonnet 3.5 (200k context, 8192 max output): usable = 200,000 - 8,192 = 191,808
  #   2. GPT-4 (128k context, 4096 max output): usable = 128,000 - 4,096 = 123,904
  #   3. High-output model (200k context, 64k max output): capped at 32k → usable = 200,000 - 32,000 = 168,000
  #   4. Unknown model (100k context, max_output=0): fallback to 32k → usable = 100,000 - 32,000 = 68,000
  #   5. Token sum: 150k input + 20k cache_read + 5k output = 175,000 total
  #   6. Under threshold: 175,000 total < 191,808 usable → NO compaction
  #   7. Over threshold: 195,000 total > 191,808 usable → TRIGGER compaction
  #   8. Exact boundary: 191,808 total == 191,808 usable → NO trigger (uses > not >=)
  #   9. Zero context: context=0 → returns false immediately (no limit = no compaction)
  #   10. Disable flag: DISABLE_AUTOCOMPACT=true → returns false regardless of tokens
  #   11. Benefit: Claude agent gets 11,808 more tokens before compaction (191,808 vs legacy 180,000)
  #
  # ========================================

  Background: User Story
    As a AI coding agent using codelet
    I want to have compaction triggered at the correct token threshold
    So that I maximize usable context while reserving space for model output and avoid premature or late compaction triggers

  # ========================================
  # USABLE CONTEXT CALCULATION SCENARIOS
  # ========================================

  Scenario: Calculate usable context for Claude Sonnet
    Given a model with context_window of 200000 tokens
    And the model has max_output_tokens of 8192
    And SESSION_OUTPUT_TOKEN_MAX is 32000
    When I calculate usable context
    Then usable context should be 191808 tokens
    # 200,000 - min(8,192, 32,000) = 200,000 - 8,192 = 191,808

  Scenario: Calculate usable context for GPT-4
    Given a model with context_window of 128000 tokens
    And the model has max_output_tokens of 4096
    And SESSION_OUTPUT_TOKEN_MAX is 32000
    When I calculate usable context
    Then usable context should be 123904 tokens
    # 128,000 - min(4,096, 32,000) = 128,000 - 4,096 = 123,904

  Scenario: SESSION_OUTPUT_MAX caps high-output models
    Given a model with context_window of 200000 tokens
    And the model has max_output_tokens of 64000
    And SESSION_OUTPUT_TOKEN_MAX is 32000
    When I calculate usable context
    Then usable context should be 168000 tokens
    # 200,000 - min(64,000, 32,000) = 200,000 - 32,000 = 168,000

  Scenario: Unknown model with zero max_output uses SESSION_OUTPUT_MAX fallback
    Given a model with context_window of 100000 tokens
    And the model has max_output_tokens of 0
    And SESSION_OUTPUT_TOKEN_MAX is 32000
    When I calculate usable context
    Then usable context should be 68000 tokens
    # min(0, 32000) = 0, but 0 triggers fallback to SESSION_OUTPUT_MAX
    # usable = 100,000 - 32,000 = 68,000 (NOT 100,000)

  # ========================================
  # TOKEN COUNTING SCENARIOS
  # ========================================

  Scenario: Token count includes all three token types
    Given input_tokens of 150000
    And cache_read_tokens of 20000
    And output_tokens of 5000
    When I calculate total token count
    Then total token count should be 175000
    # 150,000 + 20,000 + 5,000 = 175,000 (NOT discounted)

  Scenario: Token count does not discount cache tokens
    Given input_tokens of 180000
    And cache_read_tokens of 50000
    And output_tokens of 10000
    When I calculate total token count
    Then total token count should be 240000
    # 180,000 + 50,000 + 10,000 = 240,000
    # NOT: 180,000 - (50,000 * 0.9) = 135,000 (old broken calculation)

  # ========================================
  # COMPACTION TRIGGER SCENARIOS
  # ========================================

  Scenario: No compaction when under usable context threshold
    Given a Claude model with context_window of 200000 and max_output of 8192
    And input_tokens of 150000
    And cache_read_tokens of 20000
    And output_tokens of 5000
    When I check if compaction should trigger
    Then compaction should NOT trigger
    # total=175,000, usable=191,808 → 175,000 < 191,808 → NO trigger

  Scenario: Trigger compaction when over usable context threshold
    Given a Claude model with context_window of 200000 and max_output of 8192
    And input_tokens of 170000
    And cache_read_tokens of 20000
    And output_tokens of 5000
    When I check if compaction should trigger
    Then compaction should trigger
    # total=195,000, usable=191,808 → 195,000 > 191,808 → TRIGGER

  Scenario: Compaction triggers later than legacy 90% threshold
    Given a Claude model with context_window of 200000 and max_output of 8192
    And input_tokens of 165000
    And cache_read_tokens of 10000
    And output_tokens of 6000
    When I check if compaction should trigger
    Then compaction should NOT trigger
    # total=181,000, usable=191,808 → no trigger
    # Legacy 90% threshold=180,000 would have triggered here

  Scenario: No compaction when token count exactly equals usable context
    Given a Claude model with context_window of 200000 and max_output of 8192
    And input_tokens of 171808
    And cache_read_tokens of 15000
    And output_tokens of 5000
    When I check if compaction should trigger
    Then compaction should NOT trigger
    # total=191,808, usable=191,808 → 191,808 > 191,808 is FALSE
    # Uses strictly greater than (>), not greater-or-equal (>=)

  # ========================================
  # EDGE CASE AND BYPASS SCENARIOS
  # ========================================

  Scenario: No compaction when context_window is zero
    Given a model with context_window of 0 tokens
    And the model has max_output_tokens of 8192
    And input_tokens of 50000
    And cache_read_tokens of 10000
    And output_tokens of 5000
    When I check if compaction should trigger
    Then compaction should NOT trigger
    # context_window = 0 means no context limit, so no compaction needed

  Scenario: No compaction when disable flag is set
    Given a model with context_window of 200000 tokens
    And the model has max_output_tokens of 8192
    And input_tokens of 190000
    And cache_read_tokens of 50000
    And output_tokens of 10000
    And the disable autocompact flag is set to true
    When I check if compaction should trigger
    Then compaction should NOT trigger
    # Even though total (250,000) far exceeds usable (191,808)
    # The disable flag bypasses all compaction logic

  # ========================================
  # PROVIDER TRAIT SCENARIOS
  # ========================================
  # NOTE: Provider-specific max_output_tokens() implementation will be
  # handled in a separate work unit (CTX-003) for the providers crate.
  # See: codelet/providers/src/{claude,openai,gemini}.rs
