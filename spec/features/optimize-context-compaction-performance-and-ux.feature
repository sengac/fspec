@done
@feature-management
@cli
@performance
@PERF-002
Feature: Optimize Context Compaction Performance and UX

  """
  Batch anchor detection in ContextCompactor::compact() - single LLM call instead of per-turn calls
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Compaction must complete in under 30 seconds for typical conversations (up to 50 turns)
  #   2. Only one /compact handler should exist in the codebase to avoid execution conflicts
  #   3. Loading state must be visible during compaction with clear progress indication
  #   4. Anchor detection must be batched into a single LLM call instead of multiple sequential calls
  #   5. /compact command should work immediately without requiring a prior message to the LLM
  #
  # EXAMPLES:
  #   1. User types /compact in a 40-turn conversation, sees 'Compacting context...' with progress indicator, compaction completes in 15 seconds
  #   2. Developer starts new session, types /compact before sending any messages, command works immediately and compacts any existing conversation history
  #   3. Developer types message during compaction, input is disabled with message 'Compacting context... please wait' until completion
  #   4. Developer runs /compact but LLM provider is unavailable, sees error message 'Compaction failed: Provider unavailable. Please try again later.'
  #   5. Developer runs /compact, sees 'Analyzing anchors... 15/32 turns', then 'Generating summary...', completes successfully with 'Context compacted: 8500→3200 tokens'
  #   6. Compaction fails after 'Analyzing anchors... 23/47 turns', shows dialog with three options: 'Retry' | 'Continue without compacting' | 'Cancel', user selects option
  #   7. Developer runs /compact, auto-retry succeeds after network blip, user sees 'Retrying...', then normal completion flow
  #
  # QUESTIONS (ANSWERED):
  #   Q: What is the maximum acceptable time for compaction to complete? Is 30 seconds the hard requirement or can we accept up to 60 seconds for very large conversations?
  #   A: Just do it as fast as possible - don't worry about specific time metrics
  #
  #   Q: During the loading state, should users be completely blocked from typing, or should they be able to type but messages queue up until compaction completes?
  #   A: Yes, we need a separate COMPACTING state in the Rust SessionStatus enum that syncs to TypeScript. The InputTransition component should show compaction progress percentage instead of the generic thinking indicator. This reuses the existing state architecture: Rust enum -> NAPI -> useRustSessionState -> UI components.
  #
  #   Q: What kind of progress indication do you want during compaction? A simple spinner, percentage progress, or detailed progress showing 'Analyzing anchors... 23/47 turns'?
  #   A: Show detailed progress: 'Analyzing anchors... 23/47 turns' which indicates current phase and specific turn progress rather than just percentage
  #
  #   Q: If compaction fails partway through, should the system retry automatically, let the user retry manually, or fall back to the original context without compaction?
  #   A: Use existing confirmation dialog system to show retry options when compaction fails. Leverage pre-existing dialog components instead of custom error handling UI.
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to run /compact command to optimize conversation context
    So that I get fast compaction (under 30 seconds) with clear loading feedback instead of waiting 5+ minutes without knowing what's happening

  Scenario: Fast compaction with detailed progress indication
    Given I have a conversation with 40 turns
    When I type "/compact" and press Enter
    Then I should see "Analyzing anchors... 15/32 turns"
    And I should see "Generating summary..."
    And compaction should complete successfully
    And I should see "Context compacted: 8500→3200 tokens"

  Scenario: Compaction works immediately in new session
    Given I start a new session
    And there is existing conversation history available
    When I type "/compact" before sending any messages to the LLM
    Then the command should work immediately
    And any existing conversation history should be compacted

  Scenario: Input disabled during compaction with clear feedback
    Given I start a compaction process
    When I try to type a message during compaction
    Then input should be disabled
    And I should see "Compacting context... please wait" message
    And input should remain disabled until compaction completes

  Scenario: Error handling with retry dialog for provider unavailable
    Given I have a conversation that can be compacted
    When I run "/compact"
    And the LLM provider is unavailable
    Then I should see a dialog with options:
      | Retry                     |
      | Continue without compacting |
      | Cancel                    |
    And I can select any option to proceed

  Scenario: Auto-retry succeeds after transient network issue
    Given I run "/compact"
    When a network blip occurs during anchor analysis
    Then the system should auto-retry once
    And I should see "Retrying..." message
    And compaction should continue with normal completion flow

  Scenario: Manual retry after persistent failure
    Given I run "/compact"
    And compaction fails after "Analyzing anchors... 23/47 turns"
    When the auto-retry also fails
    Then I should see a dialog for manual retry options
    And I can choose to retry, continue without compacting, or cancel
