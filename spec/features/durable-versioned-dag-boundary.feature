@done
@agent-core
@context-management
@compaction
@CMPCT-059
Feature: Durable versioned DAG boundary
  """
  One manifest transaction promotes the exact admitted DAG, coverage prefix,
  generation, model/protocol, token provenance and compatible checkpoint.
  Promotion is irreversible. Failed publication preserves the new identity and
  blocks inference until that exact durable identity is reconciled. Cold replay
  validates the boundary before returning the DAG and structured suffix.
  """

  Scenario: Cold resume restores the exact committed DAG and suffix
    Given an admitted DAG and structured work persisted after its source snapshot
    When a fresh process opens the promoted session boundary
    Then it restores the same generation checkpoint tokens exact DAG and complete structured suffix payload
    And every original record remains unchanged and covered originals are not replayed

  Scenario: Failure before promotion preserves the old generation
    Given an existing committed generation and a candidate whose promotion cannot complete
    When persistence rejects the candidate before promotion
    Then cold resume still restores the old generation without a successful acknowledgement

  Scenario: Failed publication retains the promoted identity until reconciliation
    Given a candidate durably promoted as a new generation
    When its completion publication fails
    Then the failure reports the new durable commit identity and inference remains blocked
    And reconciling that exact identity permits inference without a second promotion

  Scenario: Incompatible durable coverage blocks cold replay
    Given a promoted boundary whose covered message identity is incompatible
    When a cold reader loads the model replay
    Then replay fails explicitly without raw original fallback
