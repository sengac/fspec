@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Runtime context snapshots derive from the authoritative durable boundary
  """
  The shared committed-compaction-context-projection contract applies. Context
  occupancy, verified durable coverage and generation travel coherently, with
  estimated occupancy distinct from measured fit and cumulative billing.
  """

  Scenario: Runtime snapshot derives occupancy and coverage from the authoritative durable generation
    Given a durably promoted DAG with an estimated whole-request occupancy and verified suffix
    When the runtime prepares its committed context event
    Then tokens coverage and generation match the authoritative durable boundary
    And trigger-relative fill remains distinct from absolute model capacity
    And stale or uncommitted manifests cannot produce a committed context event
