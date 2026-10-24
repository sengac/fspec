@done
@CMPCT-070
@compaction
@agent-core
@phase-1
@validation
Feature: DAG cold boundary validation
  """
  Cold replay and reconciliation validate the authoritative committed DAG and
  generation metadata before exposing memory or acknowledging publication.
  """

  Scenario: Reject malformed committed memory during cold replay
    Given a committed generation with corrupted DAG syntax provenance coverage identity or admission
    When a cold reader loads the committed replay envelopes
    Then replay fails explicitly without presenting invalid memory or changing the durable revision

  Scenario: Invalid promoted memory cannot be reconciled
    Given a promoted generation whose committed DAG has become invalid
    When its exact identity requests reconciliation
    Then reconciliation fails without clearing the pending publication or changing durable bytes
