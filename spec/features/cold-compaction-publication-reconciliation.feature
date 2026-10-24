@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Cold publication loss blocks inference until exact reconciliation
  """
  The shared committed-compaction-context-projection contract applies. Lost
  notification does not roll back durable promotion or permit unsafe inference.
  Only exact identity reconciliation releases the block without another commit.
  """

  Scenario: Cold notification loss blocks inference until exact identity reconciliation
    Given a durable generation whose completion publication failed
    When a fresh session restores the pending generation and attempts inference
    Then no physical provider request is sent
    And a different generation identity cannot release the block
    And exact identity reconciliation preserves the generation and permits the next request
