@cli
@context-management
@compaction
@CMPCT-079
Feature: Compaction recovery preparation isolation
  """
  Preserves the acceptance scenarios from runtime-compaction-no-substitute-success.
  One real integration target owns this specification partition.
  """

  Scenario: Recovery preparation preserves the authoritative conversation
    Given a durable session with original conversation turns and provider state
    When overflow recovery prepares an in-view DAG construction request
    Then original messages turns token counters and durable envelopes are unchanged
    And ordinary inference stays blocked until a candidate is durably applied

  Scenario: Recovery preparation does not persist an empty Codex checkpoint
    Given a durable Codex session with a bound provider persistence writer
    When overflow recovery prepares an in-view DAG construction request
    Then the persisted provider checkpoint and original envelopes remain unchanged
