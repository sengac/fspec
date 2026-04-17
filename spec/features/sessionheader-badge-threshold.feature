@done
@CTX-009
Feature: SessionHeader badge shows compaction threshold
  """
  TypeScript-only change, no Rust modifications needed
  SessionHeader gains compactionThreshold prop, uses it for badge (falls back to contextWindow)
  AgentView reads rustSnapshot.model.compactionThreshold and passes to SessionHeader
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The SessionHeader badge must display the compaction threshold, not the raw context window
  #   2. The fill percentage and badge must use the same denominator (the compaction threshold)
  #   3. When compaction threshold is not yet available (pre-model-selection), fall back to context window
  #   4. The compaction threshold comes from rustSnapshot.model.compactionThreshold (already exposed by CTX-007)
  #   5. formatContextWindow utility works unchanged — it formats any token count
  #
  # EXAMPLES:
  #   1. Claude Sonnet 4 with 200k context / ~192k threshold → badge shows [192k], fill% relative to 192k
  #   2. Gemini 2.5 Pro with 1M context / 800k threshold → badge shows [800k], fill% relative to 800k
  #   3. Custom model with user-configured 150k threshold on 200k context → badge shows [150k]
  #   4. Before model selection (no threshold yet) → badge falls back to context window from models.dev
  #   5. Session resume restores compaction threshold from Rust state → badge shows threshold
  #
  # ========================================
  Background: User Story
    As a developer using codelet
    I want to see the compaction threshold in the SessionHeader badge
    So that the badge and fill percentage agree — I know exactly how close I am to compaction

  Scenario: Badge shows compaction threshold for Claude model
    Given a Claude Sonnet 4 model with 200000 token context window
    And the Rust-resolved compaction threshold is 191808
    When the SessionHeader renders with both values
    Then the badge should display "[192k]" using the compaction threshold
    And the badge should not display "[200k]" from the raw context window

  Scenario: Badge shows compaction threshold for large-context model
    Given a Gemini 2.5 Pro model with 1000000 token context window
    And the Rust-resolved compaction threshold is 800000
    When the SessionHeader renders with both values
    Then the badge should display "[800k]" using the compaction threshold
    And the badge should not display "[1M]" from the raw context window

  Scenario: Badge shows user-configured custom threshold
    Given a custom model with 200000 token context window
    And the user has configured a compaction threshold of 150000
    When the SessionHeader renders with both values
    Then the badge should display "[150k]" using the custom threshold
    And the badge should not display "[200k]" from the raw context window

  Scenario: Badge falls back to context window when threshold is unavailable
    Given a model with 200000 token context window
    And no compaction threshold is available yet
    When the SessionHeader renders without a compaction threshold
    Then the badge should display "[200k]" from the context window as fallback

  Scenario: Badge shows threshold after session resume
    Given a resumed session with Rust state containing compaction threshold 191808
    And the context window from model data is 200000
    When the SessionHeader renders with the restored threshold
    Then the badge should display "[192k]" using the restored compaction threshold
