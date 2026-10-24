@BUG-210
@phase-1
@agent-core
@context-management
@compaction
Feature: Automatic compaction preserves the owning session lifecycle
  """
  Committed completion clears progress without declaring unfinished work idle.
  Recovery restores its owning activity before resuming inference. Applying a
  manually submitted summary to an already idle session never starts inference.
  """

  Background: User Story
    As a user running an agent session
    I want session status to reflect its actual compaction and inference activity
    So that I can distinguish unfinished work from an idle conversation

  Scenario: Automatic completion keeps the unfinished task active
    Given an unfinished agent task requires automatic compaction
    When the committed summary is followed by resumed inference
    Then no idle state is announced before the remaining task finishes
    And the resumed task announces its running state

  Scenario: Applying a manual summary leaves an idle session idle
    Given an idle session has a manually submitted valid summary
    When that summary is durably applied
    Then the session remains idle without starting inference
