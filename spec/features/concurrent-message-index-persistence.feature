@phase-1
@persistence
@session-management
@bug-fix
Feature: Concurrent message index persistence
  """
  MessageStore instances share an on-disk message index across sessions and processes.
  Index publication must use an isolated staging file per writer and an atomic
  replacement so concurrent index rebuilds cannot consume another writer's file.
  """

  Scenario: Concurrent session stores rebuild a shared message index
    Given stored messages with no saved message index
    When multiple session stores open the shared message history concurrently
    Then every store opens without an index persistence error
    And every store can read the stored messages
    And the saved index can be reopened without rebuilding
    And no temporary index files remain
