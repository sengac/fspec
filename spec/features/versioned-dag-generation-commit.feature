@done
@agent-core
@context-management
@compaction
@CMPCT-055
Feature: Versioned DAG generation commit
  """
  Candidates own an operation and immutable durable source prefix, generation,
  model and protocol. Appending original envelopes does not invalidate the
  captured prefix. Replacing it or advancing the committed generation does.
  Every rejection preserves the authoritative boundary and original envelopes.
  """

  Scenario: Appended work survives a captured prefix commit
    Given a candidate captured before additional tool work is persisted
    When the candidate commits against the unchanged source generation
    Then one new generation covers only the captured prefix
    And the verified uncovered suffix and immutable originals survive

  Scenario: An older candidate cannot replace a newer generation
    Given two candidates captured from the same durable generation
    When one commits before the other
    Then the second is explicitly rejected as stale
    And the newer generation and operation identity remain unchanged

  Scenario: Changed prefix or model identity rejects a candidate
    Given a candidate with captured durable source and model identity
    When its prefix or selected model no longer matches
    Then commit is explicitly rejected without promotion or completion

  Scenario: Obsolete publication cannot announce a replaced generation
    Given an acknowledged generation and a newer promoted generation
    When an obsolete or already acknowledged snapshot requests publication
    Then publication is rejected before notifying subscribers
    And the authoritative manifest bytes remain unchanged
