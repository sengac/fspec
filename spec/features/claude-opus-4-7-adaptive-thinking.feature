@BUG-582
@PROV-079
@providers
@thinking-detection
Feature: Add Claude Opus 4.7 adaptive thinking support
  """
  MODEL-010 (thinking-levels-reach-each-provider-through-a-wire-table.feature) replaced the
  thinking-config scenarios: the anthropic wire sends adaptive thinking for an effort and enabled
  thinking for a token budget, checked against the model's advertised capability, so no model-id
  routing or budget table picks the form. is_adaptive_thinking_model now chooses only the beta headers.
  Default-adaptive design: Claude 4.6+ models automatically use adaptive thinking.
  No per-model constant or allowlist entry needed — new models are adaptive by default.
  Only old models (4.5 and earlier) are listed as budgeted exceptions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Claude Opus 4.7 is adaptive-only: it MUST receive {"type": "adaptive"}, never {"type": "enabled", "budget_tokens": N}
  #   2. Opus 4.7 does NOT support manual budget_tokens — requests with budget_tokens must be rejected or ignored
  #   3. Opus 4.7 must NOT receive the interleaved-thinking-2025-05-14 beta header
  #   4. Default-adaptive design: all Claude 4.6+ models are adaptive — no per-model constant needed
  #   5. is_adaptive_thinking_model() uses prefix-based denylist, not per-model allowlist
  #   6. The NAPI getThinkingConfig for claude-opus-4-7 must return {"thinking":{"type":"adaptive"}} regardless of requested thinking level (except Off)
  #
  # EXAMPLES:
  #   1. getThinkingConfig('claude-opus-4-7', High) returns {"thinking":{"type":"adaptive"}} — NOT {"thinking":{"type":"enabled","budget_tokens":32000}}
  #   2. getThinkingConfig('claude-opus-4-7', Off) returns {} — thinking disabled respects user intent
  #   3. build_beta_headers('claude-opus-4-7', false) does NOT include interleaved-thinking-2025-05-14
  #   4. is_adaptive_thinking_model('claude-opus-4-7') returns true
  #   5. Existing claude-opus-4-6 and claude-sonnet-4-6 adaptive behaviour is unchanged (no regression)
  #
  # ========================================
  Background: User Story
    As a developer using fspec with Claude Opus 4.7
    I want to have thinking mode work correctly
    So that the model can reason about complex tasks instead of failing with API errors

  @unit
  Scenario: Opus 4.7 is detected as adaptive thinking model
    Given the model identifier "claude-opus-4-7"
    When I check is_adaptive_thinking_model
    Then the result should be true

  @unit
  Scenario: Opus 4.7 beta headers exclude interleaved-thinking
    Given the model identifier "claude-opus-4-7"
    When I build beta headers for the model
    Then the headers should NOT include "interleaved-thinking-2025-05-14"
    And the headers should include "prompt-caching-2024-07-31"
