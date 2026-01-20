@cli
@done
@CMPCT-002
Feature: Handle Gemini continuation + compaction gracefully

  """
  Pattern follows OpenCode's approach: signal at turn boundary, compact in outer loop, retry
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When compaction is triggered during Gemini continuation, the system MUST break out of the continuation loop gracefully instead of erroring
  #   2. Any partial text accumulated during continuation MUST be saved to the session before signaling compaction
  #   3. Token tracker MUST be updated with cumulative billing before signaling compaction
  #   4. The stream loop MUST return a distinct NeedsCompaction result variant so callers can trigger compaction and retry
  #
  # EXAMPLES:
  #   1. Given Gemini model is in continuation (empty tool_use received), When compaction threshold is exceeded (PromptCancelled error), Then system saves partial continuation_text, updates token tracker, returns NeedsCompaction
  #   2. Given session has accumulated 180K tokens (90% of 200K context window), When Gemini continuation triggers next request, And request is cancelled due to token limit, Then session does NOT fail, And user sees 'Compacting context...' message, And session continues after compaction
  #   3. Given Gemini model has produced partial response text during continuation, When compaction is triggered, Then partial text is preserved in session history, And user does not lose any model output
  #
  # ========================================

  Background: User Story
    As a developer using Gemini models
    I want to have context compaction handled gracefully during model continuation
    So that my session doesn't fail and lose work when context overflows mid-continuation

  @unit
  Scenario: Graceful handling when compaction triggered during continuation
    Given a Gemini model session is in continuation mode
    And the model has received an empty tool_use response
    When the compaction threshold is exceeded during continuation
    Then the system should break out of the continuation loop
    And the system should save any partial continuation text
    And the system should update the token tracker with cumulative billing
    And the system should return a NeedsCompaction result

  @integration
  Scenario: Session continues after compaction during continuation
    Given a session has accumulated 90% of the context window tokens
    And the Gemini model is processing a continuation request
    When the next request is cancelled due to token limit
    Then the session should not fail with an error
    And the user should see a compaction status message
    And the session should continue after compaction completes

  @unit
  Scenario: Partial model output is preserved during compaction
    Given a Gemini model has produced partial response text during continuation
    And the partial text contains valuable information
    When compaction is triggered mid-continuation
    Then the partial text should be saved to session history
    And the user should not lose any model output
