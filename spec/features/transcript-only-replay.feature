@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Decode transcript-only records without constructing model messages
  """
  The shared decoder admits typed private transcript records into display only.
  Unknown tool outcomes are inert notices, never revived active operations.
  """

  Scenario: Transcript replay remains display only and preserves truthful interruption
    Given an archived incomplete prefix and typed terminal outcomes
    When the shared replay decoder reads transcript-only records
    Then display chunks retain the prefix and outcome while model messages and envelopes stay empty
