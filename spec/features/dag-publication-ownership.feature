@done
@agent-core
@context-management
@compaction
@CMPCT-055
Feature: DAG publication ownership
  """
  Publication and acknowledgement retain the manifest writer transaction.
  Recursive access fails explicitly, and irreversible promotion retains the
  operation identity even when directory durability acknowledgement fails.
  """

  Scenario: Publication callbacks cannot reenter session ownership
    Given a promoted generation awaiting publication
    When its callback attempts nested manifest reads or writes
    Then nested ownership is rejected explicitly without waiting
    And the original pending identity remains available for reconciliation

  Scenario: Post-promotion durability failure retains the compaction identity
    Given an admitted candidate with an exact operation and generation
    When directory acknowledgement fails after atomic promotion
    Then the result distinguishes the promoted identity from a pre-promotion failure
    And a cold reader retains that pending generation without another commit
