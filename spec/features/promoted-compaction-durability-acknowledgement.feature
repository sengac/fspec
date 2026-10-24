@agent-core
@context-management
@compaction
@CMPCT-059
Feature: Promoted compaction durability acknowledgement
  """
  Preserves the runtime acknowledgement acceptance from durable-versioned-dag-boundary.
  A failed directory synchronization after rename retains the promoted identity.
  """

  Scenario: Runtime retains ownership after a promoted durability acknowledgement failure
    Given a selected runtime candidate whose manifest rename succeeds but directory synchronization fails
    When durable application reports the promoted identity
    Then ordinary inference remains blocked and that exact identity can be reconciled without another promotion
