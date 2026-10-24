@done
@CMPCT-067
@compaction
@context-management
@agent-core
@phase-1
@bug
Feature: Context header distinguishes estimated unknown and measured occupancy
  """
  The shared context contract in coherent-request-context-fill.feature applies.
  Estimated committed occupancy and unavailable measurements must never look
  like measured usage or authorize inference through a guessed fit percentage.
  """

  Scenario: Header distinguishes estimated and unknown context from measured usage
    Given estimated committed context followed by an unavailable request measurement
    When the session header renders each context snapshot
    Then estimated occupancy is labelled estimated
    And unavailable occupancy is labelled unknown rather than zero percent
