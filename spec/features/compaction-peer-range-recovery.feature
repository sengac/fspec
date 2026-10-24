@cli
@done
@BUG-233
@phase-1
@agent-core
@compaction
@validation
Feature: Compaction peer range recovery
  """
  Persisted turn bounds are inclusive. Same-depth peers must be disjoint after
  promotion as well as initial generation, while cross-depth nesting remains valid.
  Rejected candidates are never edited or partially installed by the runtime.
  Bounded correction must return a complete newly validated candidate.
  """

  Scenario: Generation explains inclusive peer bounds and promotion conflicts
    Given a live session with persisted originals
    When fresh and incremental compaction requests reach the provider
    Then both requests distinguish same-depth conflicts from valid cross-depth overlap
    And incremental instructions require checking conflicts after depth promotion

  Scenario: Conflicting peer ranges receive precise bounded correction
    Given generated duplicate intersecting shared-endpoint or unsorted same-depth ranges
    When the production builder receives a valid hierarchical replacement
    Then correction names the conflicting depth and both inclusive ranges
    And only the complete replacement with disjoint peers and cross-depth nesting is committed

  Scenario: Exhausted peer conflicts never promote partial memory
    Given every generated candidate contains conflicting peer ranges
    When the production summary handoff exhausts three candidate attempts
    Then failure identifies the conflicting depth ranges target operation and attempt
    And durable originals and committed memory remain unchanged with ordinary inference blocked

  Scenario: Arbitrary inclusive peers preserve strict hierarchy validation
    Given arbitrary persisted inclusive range endpoints
    When whole candidates contain disjoint peers nested depths partial intersections or shared endpoints
    Then disjoint peers and cross-depth nesting are accepted unchanged while shared endpoints and partial intersections are rejected
