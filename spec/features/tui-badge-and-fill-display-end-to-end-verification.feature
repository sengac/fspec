@done
@LIMITS-006
Feature: TUI Badge and Fill% Display — End-to-End Verification
  """
  Tests verify that SessionHeader badge displays compactionThreshold (not raw contextWindow). AgentView.tsx reads rustModel.contextWindow and rustModel.compactionThreshold from Rust state. SessionHeader.tsx uses compactionThreshold ?? contextWindow for badge. formatContextWindow from sessionHeaderUtils.ts formats token count to human-readable string (e.g. 192k, 800k, 102k).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Badge shows [192k] for Claude Opus 4.6 (not [968k])
  #   2. Badge shows [800k] for Gemini 2.5 Pro (80% of 1M)
  #   3. Badge shows [102k] for GPT-4o (80% of 128k)
  #   4. Fill% is always calculated against the compaction threshold, never the raw context window
  #   5. Before model selection, badge falls back to models.dev value temporarily then updates to Rust-clamped value
  #   6. Session resume shows cached correct values immediately without re-querying
  #   7. CTX-006/CTX-009 feature files and tests must be updated to use correct expected values (192k not 968k for Claude)
  #
  # EXAMPLES:
  #   1. Claude Opus 4.6 at 87k tokens: badge [192k], fill [45%] — correct values visible in TUI header
  #   2. Gemini 2.5 Pro at 400k tokens: badge [800k], fill [50%]
  #   3. GPT-4o at 51k tokens: badge [102k], fill [50%]
  #
  # ========================================
  Background: User Story
    As a developer using the TUI
    I want to see the correct badge value and fill percentage reflecting the provider-clamped compaction threshold
    So that the displayed numbers match reality and I know exactly how close I am to compaction

  Scenario: Badge shows [192k] for Claude Opus 4.6
    Given a Claude Opus 4.6 model with 200000 context window
    And the Rust-resolved compaction threshold is 191808 tokens
    And the session has consumed 87000 tokens
    When the SessionHeader renders
    Then the badge should display "[192k]"
    And the badge should not display "[968k]"
    And the fill percentage should be approximately 45 percent

  Scenario: Badge shows [800k] for Gemini 2.5 Pro
    Given a Gemini 2.5 Pro model with 1000000 context window
    And the Rust-resolved compaction threshold is 800000 tokens
    And the session has consumed 400000 tokens
    When the SessionHeader renders
    Then the badge should display "[800k]"
    And the badge should not display "[1M]"
    And the fill percentage should be 50 percent

  Scenario: Badge shows [102k] for GPT-4o
    Given a GPT-4o model with 128000 context window
    And the Rust-resolved compaction threshold is 102400 tokens
    And the session has consumed 51200 tokens
    When the SessionHeader renders
    Then the badge should display "[102k]"
    And the fill percentage should be 50 percent

  Scenario: Fill percentage uses compaction threshold as denominator
    Given a Claude Sonnet 4 model with 200000 context window
    And the Rust-resolved compaction threshold is 191808 tokens
    And the session has consumed 95904 tokens
    When the context fill percentage is calculated
    Then the fill percentage should use the compaction threshold as the denominator
    And the fill percentage should be 50 percent

  Scenario: Badge falls back to context window before model selection
    Given a session where no model has been selected yet
    And no compaction threshold is available
    When the SessionHeader renders with a context window of 200000
    Then the badge should display "[200k]" as a fallback
