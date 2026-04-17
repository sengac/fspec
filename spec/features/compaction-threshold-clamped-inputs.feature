@done
@LIMITS-005
Feature: Fix Compaction Threshold With Corrected Inputs
  """
  After LIMITS-004, ProviderManager.context_window() returns provider-clamped values (200k for Claude, not 1M from models.dev). This fix propagates to all 7 consumers: stream_loop threshold resolution, thinking exhaustion check, context fill emission, sub-agent propagation (DeepSearch/AgentManager), session cached AtomicU32 values, NAPI SessionModel reads, and CompactionHook threshold. The compaction_threshold.rs pure functions (resolve_compaction_threshold, calculate_usable_context) are input-correct because they receive clamped values from callers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. resolve_compaction_threshold() receives correct context_window (200k for Claude, not 1M) after LIMITS-004
  #   2. The thinking exhaustion check (stream_loop.rs ~line 1033) uses context_window directly — must verify it gets clamped 200k not raw 1M
  #   3. All 8 compaction trigger points must use values derived from the clamped context_window
  #   4. Sub-agent propagation (DeepSearch, AgentManager) via raw_model_context_window() must use clamped values
  #   5. Session cached compaction_threshold (AtomicU32) must be written with correct value at session creation and model change
  #   6. NAPI SessionModel.context_window and .compaction_threshold must reflect clamped values
  #
  # EXAMPLES:
  #   1. Claude Opus 4.6: threshold = 200,000 - 8,192 = 191,808 (legacy formula with clamped inputs)
  #   2. At 87k tokens with 191k threshold, fill shows ~45% (not 9% from wrong 968k threshold)
  #   3. Compaction fires BEFORE 200k API limit — no more prompt-too-long errors from missed compaction
  #
  # ========================================
  Background: User Story
    As a developer
    I want to verify the compaction threshold chain uses provider-clamped context_window and max_output values
    So that compaction fires at the correct threshold and context fill percentages are accurate

  Scenario: Claude Opus 4.6 compaction threshold uses clamped context window
    Given a Claude model where models.dev reports context_window of 1000000 and max_output of 16384
    When resolve_compaction_threshold is called with the clamped values and model_id "claude-opus-4-6"
    Then the threshold should be 191808 tokens (200000 minus 8192)
    And the Claude provider clamps context_window to 200000 and max_output to 8192

  Scenario: Context fill percentage is accurate with clamped threshold
    Given a clamped compaction threshold of 191808 tokens
    When the current token count is 87000
    Then the fill percentage should be approximately 45 percent (not 9 percent from wrong 968k threshold)

  Scenario: Compaction fires before API limit with clamped threshold
    Given a CompactionHook with threshold 191808 derived from clamped context_window
    When the token count reaches 192000 tokens
    Then compaction should be triggered well before the 200000 API limit

  Scenario: Gemini 2.5 Pro compaction threshold uses 80 percent of context window
    Given a Gemini model with context_window of 1000000 and max_output of 65536
    When resolve_compaction_threshold is called with model_id "gemini-2.5-pro"
    Then the threshold should be 800000 tokens (80 percent of 1M)

  Scenario: GPT-4o compaction threshold uses 80 percent of context window
    Given an OpenAI model with context_window of 128000 and max_output of 16384
    When resolve_compaction_threshold is called with model_id "gpt-4o"
    Then the threshold should be 102400 tokens (80 percent of 128k)

  Scenario: Sub-agent propagation uses clamped context window
    Given a ProviderManager with registry context_window of 1000000 for Claude
    When raw_model_context_window is called for DeepSearch or AgentManager sub-agent propagation
    Then it should return 200000 (clamped by provider hard max) not 1000000

  Scenario: Summarization budget uses clamped context window
    Given a clamped context_window of 200000 for Claude
    When calculate_summarization_budget is called with 200000
    Then the budget should be 150000 (200k minus 50k AUTOCOMPACT_BUFFER)
