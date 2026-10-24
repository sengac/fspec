@context-window
@cli
@done
@LIMITS-005
Feature: Fix Compaction Threshold With Corrected Inputs
  """
  PROV-161 supersedes historical blanket Claude ceilings. All consumers use effective model limits: stream-loop threshold resolution, thinking exhaustion, context fill, subordinate propagation, session caches and compaction hooks. Explicit smaller context overrides remain valid; automatic model limits come from matching catalog metadata.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. resolve_compaction_threshold() receives effective context_window and output budget.
  #   2. Thinking exhaustion uses the effective context window, including modern catalog limits.
  #   3. All 8 compaction trigger points must use values derived from the clamped context_window
  #   4. Sub-agent propagation (DeepSearch, AgentManager) via raw_model_context_window() must use clamped values
  #   5. Session cached compaction_threshold (AtomicU32) must be written with correct value at session creation and model change
  #   6. NAPI SessionModel.context_window and .compaction_threshold must reflect clamped values
  #
  # EXAMPLES:
  #   1. With catalog context 1000000 and output 16384, Claude threshold is 800000.
  #   2. At 87k tokens with 160k threshold, fill shows ~54% (not 10% from wrong 800k threshold)
  #   3. Compaction fires BEFORE 200k API limit — no more prompt-too-long errors from missed compaction
  #
  # ========================================
  Background: User Story
    As a developer
    I want to verify the compaction threshold chain uses provider-clamped context_window and max_output values
    So that compaction fires at the correct threshold and context fill percentages are accurate

  Scenario: Claude Opus 4.6 compaction threshold uses effective catalog limits
    Given a Claude model where models.dev reports context_window of 1000000 and max_output of 16384
    When resolve_compaction_threshold is called with the effective values and model_id "claude-opus-4-6"
    Then the threshold should be 800000 tokens (80 percent of 1000000)
    And the Claude provider retains context_window 1000000 and max_output 16384

  Scenario: Context fill percentage is accurate with clamped threshold
    Given a clamped compaction threshold of 160000 tokens
    When the current token count is 87000
    Then the fill percentage should be approximately 54 percent (not 10 percent from wrong 800k threshold)

  Scenario: Compaction fires before API limit with clamped threshold
    Given a CompactionHook with threshold 160000 derived from clamped context_window
    When the token count reaches 160001 tokens
    Then compaction should be triggered well before the 200000 API limit

  Scenario: Gemini 2.5 Pro compaction threshold uses 80 percent of context window
    Given a Gemini model with context_window of 1000000 and max_output of 65536
    When resolve_compaction_threshold is called with model_id "gemini-2.5-pro"
    Then the threshold should be 800000 tokens (80 percent of 1M)

  Scenario: GPT-4o compaction threshold uses 80 percent of context window
    Given an OpenAI model with context_window of 128000 and max_output of 16384
    When resolve_compaction_threshold is called with model_id "gpt-4o"
    Then the threshold should be 102400 tokens (80 percent of 128k)

  Scenario: Sub-agent propagation uses effective context window
    Given a ProviderManager with registry context_window of 1000000 for Claude
    When raw_model_context_window is called for DeepSearch or AgentManager sub-agent propagation
    Then it should return 1000000 from the effective model limits

  Scenario: Summarization budget uses clamped context window
    Given a clamped context_window of 200000 for Claude
    When calculate_summarization_budget is called with 200000
    Then the budget should be 150000 (200k minus 50k AUTOCOMPACT_BUFFER)
