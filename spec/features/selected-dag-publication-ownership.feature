@agent-core
@context-management
@compaction
@CMPCT-059
Feature: Selected DAG publication ownership
  """
  Preserves the acceptance scenarios from durable-versioned-dag-boundary.
  One real integration target owns this specification partition.
  """

  Scenario: Selected builder publication failure retains exact promoted ownership
    Given a selected builder returning an admissible DAG
    When its identity-bearing context publication fails
    Then the durable generation remains pending and ordinary inference stays blocked
    And exact identity reconciliation succeeds without another promotion or completion
