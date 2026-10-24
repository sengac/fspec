@agent-core
@phase-1
@context-management
@compaction
@CMPCT-056
Feature: Cross-session compaction ownership
  """
  Cross-target capture and commit never await another session mutex while a
  streaming caller owns its session. A busy owner returns an explicit typed
  failure without rebuilding absent-looking context or pinning salvage.
  Cancellation releases operation ownership and restores actual prior activity.
  A versioned capture must still match the target at the commit boundary.
  Tests use real session managers, scripted provider responses, barriers and
  harness deadlines; no sleeps synchronize reciprocal requests.
  """

  Scenario: Reciprocal streaming requests terminate without acquiring busy targets
    Given two streaming callers each owning their session lock
    When a barrier releases reciprocal cross-session compaction requests
    Then both requests return explicit busy failures within the harness deadline
    And committed messages and activity remain unchanged without completion

  Scenario: Successful cross-target compaction restores Idle activity
    Given an Idle target with committed context and a successful DAG builder
    When a different session compacts the target
    Then the new DAG is pinned and completion is published
    And a cold durable reload identifies the same admitted generation and covered originals
    And the target is Idle with no compaction progress or busy flag

  Scenario: Busy capture leaves the Idle target unchanged
    Given an Idle target whose session lock is owned
    When cross-target compaction requests capture before ownership is available
    Then the committed context and Idle activity remain unchanged without completion
    And no compaction progress or busy flag remains

  Scenario: Cancellation during generation releases target operation ownership
    Given an Idle target with a DAG builder waiting at a barrier
    When cross-target compaction is cancelled after capture
    Then the committed context and Idle activity remain unchanged without completion
    And no compaction progress or busy flag remains

  Scenario: Busy commit rejects the candidate without awaiting the target
    Given an Idle target captured by a DAG builder waiting at a barrier
    When the target lock becomes busy before the builder returns a valid DAG
    Then compaction returns an explicit busy failure within the harness deadline
    And the committed context and Idle activity remain unchanged without completion

  Scenario: Failed builder cannot publish a cross-target fallback
    Given an Idle target with committed context and a failing DAG builder
    When a different session compacts the target
    Then compaction returns an explicit failure without a substitute DAG
    And the committed context and Idle activity remain unchanged without completion

  Scenario: A second operation cannot take ownership from an active builder
    Given an Idle target captured by a DAG builder waiting at a barrier
    When a second cross-target compaction requests the same target
    Then the second operation returns an explicit busy failure within the harness deadline
    And the first operation retains its activity and progress until cancellation
    And cancellation restores the unchanged Idle target without completion
