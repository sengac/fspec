@CLI-018
Feature: CLI-018: Retry Logic for LLM Summary Generation
  As a CLI user
  I want LLM summary generation to retry on failure
  So that transient errors don't block context compaction

  Background:
    Given the context compaction system is active
    And an LLM provider is configured

  Scenario: Successful summary generation on first attempt
    Given the LLM provider is functioning normally
    When compaction triggers summary generation
    Then the summary should be generated successfully
    And no retries should be attempted

  Scenario: Retry on transient failure with eventual success
    Given the LLM provider fails on first attempt
    And the LLM provider succeeds on second attempt
    When compaction triggers summary generation
    Then the first attempt should fail
    And a retry should be attempted after 1000ms delay
    And the second attempt should succeed
    And the summary should be returned

  Scenario: Retry with exponential backoff delays
    Given the LLM provider fails on all attempts
    When compaction triggers summary generation with max 3 retries
    Then retry 1 should occur after 0ms delay (immediate)
    And retry 2 should occur after 1000ms delay
    And retry 3 should occur after 2000ms delay
    And then all retries should be exhausted

  Scenario: Fallback behavior when all retries fail
    Given the LLM provider fails on all 3 retry attempts
    When compaction triggers summary generation
    Then compaction should not fail entirely
    And a fallback summary should be generated
    And the fallback summary should indicate summarization failed
    And kept messages should still be returned

  # Note: Current implementation retries on ALL errors uniformly.
  # Error-type-specific retry logic could be added in a future iteration.
