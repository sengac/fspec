@done
@CTX-002
@cli
@context-window
Feature: Optimized Compaction Window Limit Trigger
  """
  Key architectural decisions:
  - Implements optimized isOverflow() algorithm for context compaction
  - Checked before each API call (CompactionHook::on_completion_call) against the resolved compaction threshold
  - No disable-autocompact setting exists (product owner decision, 2026-11-05)

  Dependencies and integrations:
  - rust/core/src/compaction_hook.rs - Replace calculate_effective_tokens() with simple sum
  - rust/cli/src/compaction_threshold.rs - Add SESSION_OUTPUT_TOKEN_MAX constant and should_trigger_compaction()
  - rust/cli/src/compaction_threshold.rs - Deprecate calculate_compaction_threshold() (keep for backwards compat)
  - rust/providers/src/lib.rs - Add max_output_tokens() to Provider trait
  - rust/providers/src/claude.rs - Return model-specific max_output (varies by model variant)
  - rust/providers/src/openai.rs - Return model-specific max_output (varies by model variant)
  - rust/providers/src/gemini.rs - Return model-specific max_output (varies by model variant)

  Algorithm:
  1. If context_window == 0 or model_max_output == 0 → usable_context = 0 (no 32k fallback)
  2. Calculate: output_reservation = min(model_max_output, SESSION_OUTPUT_TOKEN_MAX)
  3. Calculate: usable_context = context_window - output_reservation
  4. Calculate: threshold = min(user override or default 80% of context_window, usable_context)
  5. Calculate: token_count = input_tokens + cache_read_tokens + cache_creation_tokens + output_tokens (or the estimated payload, if larger)
  6. Return: token_count > threshold (strictly greater than, not >=)

  Constants:
  - SESSION_OUTPUT_TOKEN_MAX = 32,000 tokens (reasonable upper bound for output reservation)

  Type considerations (Rust):
  - All token counts are u64
  - Use saturating_sub for usable_context calculation to prevent underflow
  - a max_output of 0 leaves no usable context (no fallback); an unavailable limit yields no threshold (PROV-165)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Token count = input_tokens + cache_read_tokens + output_tokens (simple sum, no discounting)
  #   2. Output reservation = min(model_max_output, SESSION_OUTPUT_TOKEN_MAX); a zero max_output leaves no usable context (no 32k fallback)
  #   3. Usable context = context_window - output_reservation
  #   4. Trigger compaction when: token_count > threshold = min(override or 80% of context_window, usable_context) (strictly greater than)
  #   5. SESSION_OUTPUT_TOKEN_MAX = 32,000 tokens (constant)
  #   6. No disable flag: no disable-autocompact setting exists (product owner decision, 2026-11-05)
  #   7. Provider trait MUST expose max_output_tokens() returning model-specific limit (0 if unknown)
  #   8. Each model variant may have different max_output (e.g., Claude Opus vs Sonnet)
  #
  # EXAMPLES:
  #   1. Claude Sonnet 3.5 (200k context, 8192 max output): usable = 200,000 - 8,192 = 191,808
  #   2. GPT-4 (128k context, 4096 max output): usable = 128,000 - 4,096 = 123,904
  #   3. High-output model (200k context, 64k max output): capped at 32k → usable = 200,000 - 32,000 = 168,000
  #   4. Unknown model (100k context, max_output=0): no 32k fallback → usable = 0
  #   5. Token sum: 150k input + 20k cache_read + 5k output = 175,000 total
  #   6. Under threshold: 175,000 total < 191,808 usable → NO compaction
  #   7. Over threshold: 195,000 total > 191,808 usable → TRIGGER compaction
  #   8. Exact boundary: 191,808 total == 191,808 usable → NO trigger (uses > not >=)
  #   9. Zero context: context=0 → returns false immediately (no limit = no compaction)
  #   10. No disable flag: no disable-autocompact setting exists (product owner decision, 2026-11-05)
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
  Scenario: Unknown model with zero max_output leaves no usable context
    Given a model with context_window of 100000 tokens
    And the model has max_output_tokens of 0
    When I calculate usable context
    Then usable context should be 0 tokens

    # a zero output limit is unavailable, so no usable context (and no threshold) exists
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
  Scenario: No compaction when under the compaction threshold
    Given a Claude model with context_window of 200000 and max_output of 8192
    And input_tokens of 130000
    And cache_read_tokens of 20000
    And output_tokens of 5000
    When I check if compaction should trigger
    Then compaction should NOT trigger

    # total=155,000, threshold=min(80% of 200,000, 191,808)=160,000 → NO trigger
  Scenario: Trigger compaction when over the compaction threshold
    Given a Claude model with context_window of 200000 and max_output of 8192
    And input_tokens of 170000
    And cache_read_tokens of 20000
    And output_tokens of 5000
    When I check if compaction should trigger
    Then compaction should trigger

    # total=195,000, threshold=160,000 → 195,000 > 160,000 → TRIGGER
  Scenario: No compaction when token count exactly equals the compaction threshold
    Given a Claude model with context_window of 200000 and max_output of 8192
    And input_tokens of 140000
    And cache_read_tokens of 15000
    And output_tokens of 5000
    When I check if compaction should trigger
    Then compaction should NOT trigger
