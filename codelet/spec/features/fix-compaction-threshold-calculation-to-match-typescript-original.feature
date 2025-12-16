@done
@context-management
@cli
@compaction
@critical
@CORE-008
Feature: Fix Compaction Threshold Calculation to Match TypeScript Original

  """
  Updates cli/src/compaction_threshold.rs to match TypeScript runner.ts threshold calculation. Separates threshold calculation (contextWindow * 0.9) from summarization budget calculation (contextWindow - AUTOCOMPACT_BUFFER). Ensures all existing compaction tests continue to pass.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Compaction threshold must match TypeScript: contextWindow * 0.9
  #   2. Summarization budget calculation must remain separate from threshold calculation
  #   3. All existing tests for anchor detection, weighting, and turn selection must continue to pass
  #
  # EXAMPLES:
  #   1. For 200k context window: threshold should be 180,000 (200k * 0.9), not 135,000
  #   2. For 128k context window: threshold should be 115,200 (128k * 0.9), not 70,200
  #   3. Summarization budget for 200k window should be 150,000 (200k - 50k buffer)
  #
  # ========================================

  Background: User Story
    As a developer porting codelet to Rust
    I want to ensure compaction triggers at the same token threshold as TypeScript original
    So that both implementations have identical behavior and avoid confusing differences

  Scenario: Calculate threshold for 200k context window matching TypeScript
    Given I have a provider with a 200,000 token context window
    When I calculate the compaction threshold
    Then the threshold should be 180,000 tokens
    And the threshold should equal contextWindow * 0.9
    And the threshold should NOT be 135,000 tokens

  Scenario: Calculate threshold for 128k context window matching TypeScript
    Given I have a provider with a 128,000 token context window
    When I calculate the compaction threshold
    Then the threshold should be 115,200 tokens
    And the threshold should equal contextWindow * 0.9

  Scenario: Summarization budget remains separate from threshold
    Given I have a provider with a 200,000 token context window
    When I calculate the summarization budget
    Then the budget should be 150,000 tokens
    And the budget should equal contextWindow - AUTOCOMPACT_BUFFER
    And the budget calculation should be independent of threshold calculation

  Scenario: Existing anchor detection tests continue to pass
    Given I have updated the threshold calculation
    When I run the anchor detection test suite
    Then all tests for ErrorResolution anchor detection should pass
    And all tests for TaskCompletion anchor detection should pass
    And anchor confidence values should remain unchanged

  Scenario: Existing turn selection tests continue to pass
    Given I have updated the threshold calculation
    When I run the turn selection test suite
    Then tests for keeping last 2-3 turns should pass
    And tests for anchor-based selection should pass
    And turn selection logic should remain unchanged
