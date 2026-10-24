@phase-1
@persistence
@session-management
@bug-fix
Feature: Session deletion allows orphan cleanup without closing the store
  """
  Successful deletion releases the store's retained session writer lease even
  when the manifest was already absent. The deletion transaction retains its
  own lease until its in-flight gate guard is dropped. Cleanup can then probe
  idle session locks without requiring a store reset or process restart.
  """

  Scenario: Cleanup reclaims deleted session messages while the store remains alive
    Given an open session store owns a committed session referencing a stored message
    When the store deletes that session twice without being dropped
    Then orphan cleanup removes the message without closing the store
    And the deleted session is absent from disk and the store cache

  Scenario: Cleanup reclaims deleted session messages while the singleton remains alive
    Given the session facade owns a committed session referencing a stored message
    When the facade deletes that session twice without resetting its store
    Then orphan cleanup removes the message without resetting the singleton
    And no last session remains for the project
