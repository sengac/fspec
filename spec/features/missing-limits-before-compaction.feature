@testing
@cli
@context-management
@compaction
Feature: Missing limits stop before compaction recovery
  """
  Selected request preparation validates model capacity before installing the
  streaming compaction hook or preparing a replacement conversation. Missing
  capabilities are configuration errors, not evidence of context overflow.
  """

  Background: User story
    As a session owner
    I want missing model limits reported without compaction retries
    So that my conversation is preserved until valid limits are configured

  Scenario: Unavailable or unusable model limits fail before streaming changes history
    Given a selected model with missing context, missing output, or no input budget
    And a fresh or existing conversation
    When the user submits a prompt
    Then a typed admission configuration error is returned
    And no provider request or compaction lifecycle event occurs
    And conversation messages and token counters remain unchanged
