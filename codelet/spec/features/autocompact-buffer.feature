@CLI-020
Feature: CLI-020: Autocompact Buffer for Compaction Threshold
  As a CLI user with long conversations
  I want compaction to leave headroom below the threshold
  So that I don't experience frequent re-compactions after each turn

  Background:
    Given the codelet CLI application is running
    And an LLM provider is configured with a context window

  # Problem statement:
  # Without a buffer, compaction reduces context to just below threshold.
  # The next turn's tokens immediately push back over threshold, causing
  # frequent re-compactions that degrade user experience.

  Scenario: Compaction threshold accounts for autocompact buffer
    Given the provider has a context window of 200,000 tokens
    When calculating the compaction threshold
    Then the summarization budget should be 150,000 tokens (context_window - buffer)
    And the threshold should be 135,000 tokens (budget * 0.9)

  Scenario: Buffer scales appropriately for OpenAI
    Given the provider has a context window of 128,000 tokens
    When calculating the compaction threshold
    Then threshold should be (128,000 - 50,000) * 0.9 = 70,200

  Scenario: Buffer scales appropriately for Codex
    Given the provider has a context window of 272,000 tokens
    When calculating the compaction threshold
    Then threshold should be (272,000 - 50,000) * 0.9 = 199,800

  Scenario: Buffer scales appropriately for Gemini
    Given the provider has a context window of 1,000,000 tokens
    When calculating the compaction threshold
    Then threshold should be (1,000,000 - 50,000) * 0.9 = 855,000

  Scenario: Buffer does not cause premature compaction for small contexts
    Given the provider has a context window of 128,000 tokens
    And the session has accumulated 60,000 effective tokens
    When checking if compaction should trigger
    Then compaction should NOT trigger
    # Because 60,000 < 70,200 (threshold)

  Scenario: Buffer handles edge case where context window is small
    Given the provider has a context window of 60,000 tokens
    When calculating the compaction threshold
    Then the summarization budget should be 10,000 tokens (using saturating subtraction)
    And the threshold should be 9,000 tokens (budget * 0.9)

  # Constants documentation

  Scenario: AUTOCOMPACT_BUFFER constant is defined
    Then AUTOCOMPACT_BUFFER should be defined as 50,000 tokens

  Scenario: COMPACTION_THRESHOLD_RATIO constant is defined
    Then COMPACTION_THRESHOLD_RATIO should be defined as 0.9 (90%)

  # Note: Behavioral scenario "Autocompact buffer leaves headroom after compaction"
  # is validated through threshold calculation tests - compaction itself tested in CLI-018.
