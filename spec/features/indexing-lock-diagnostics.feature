@phase-1
@persistence
@session-management
@testing
Feature: Isolated indexing and lock diagnostics
  """
  Synthetic fixtures call real MessageStore code. Original pre-fix sources and red logs are retained in the task validation evidence; post-fix checks now assert compatible layouts and verified completeness. Historical codec functions stay verbatim. No live data or daemon access.
  """

  Scenario: Valid saved indexes avoid repeated publication
    Given a valid synthetic compatible index
    When stores reopen and read cached messages
    Then no index publication occurs

  Scenario: An unknown index layout reaches real rebuild publication
    Given a synthetic index with an unknown layout
    When the current reader opens it with an index-publication failure control
    Then the control fails without replacing the incompatible index
    And a successful open republishes a compatible index once

  Scenario: Alternating codec versions share a compatible index
    Given synthetic V1 and V2 readers sharing one fixture
    When they alternate four times
    Then neither version republishes the unchanged index

  Scenario: A valid peer generation invalidates cache without rebuilding
    Given a reader with cached messages
    When a V2 peer appends a committed message
    Then the reader invalidates its cache and uses the saved index

  Scenario: Verified completeness avoids repeated absence scans
    Given a valid index and an absent message identity
    When a reader repeats lookup and a peer then advances the generation
    Then verified absence does not republish the index
    And repeated misses in one generation do not

  Scenario: Stale recorded size triggers full-history recovery
    Given a synthetic index with a stale recorded size
    When an existing original is requested
    Then the index is republished once without changing message bytes

  Scenario: Every append republishes the growing full index
    Given a synthetic message store
    When fixed-size messages are appended
    Then each append publishes an index containing every current entry

  Scenario: Strict rebuild validation differs from historical UUID scanning
    Given a synthetic message with an invalid content hash
    When historical and current scanners inspect it
    Then historical UUID scanning accepts its identity
    And the current rebuild rejects the content hash

  Scenario: Bounded fixture timings report different scan work honestly
    Given a small synthetic history
    When UUID scanning and verified rebuilding run
    Then durations are reported without a speed-ratio assertion

  Scenario: Shared leases block unrelated messages within one root
    Given two readers and an exclusively held fixture sidecar lease
    When an independent root is read
    Then the independent root progresses before the lease is released
    And both shared-root readers finish after release

  Scenario: Synchronous lock waiting stalls a single worker executor
    Given an exclusively held synthetic message-store lease
    When the real synchronous getter runs on one async worker
    Then an unrelated task waits until the external controller releases the lease
    And the unlocked executor control progresses

  Scenario: Same-version peers reuse verified completeness
    Given two V2 stores with a valid shared synthetic index
    When they alternate appending fresh message identities
    Then each write publishes only the append index
    And a consecutive write by the warm writer publishes only the append index
