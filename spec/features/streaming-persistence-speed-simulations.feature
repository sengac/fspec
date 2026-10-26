@done
@agent-core
@session-management
@persistence
@performance
@testing
@BUG-270
Feature: Diagnose persistence overhead independently of provider generation speed
  """
  Research-only simulations use prebuilt text and thinking fixtures, real temporary stores, the production BackgroundOutput sink and durability observations. Report elapsed time without machine-specific latency thresholds. Reopening a writer per delta models the historical cold-validation cost, not a checkout benchmark of the original commit. A controlled filesystem lock establishes executor starvation independently of noisy wall-clock throughput. No production behavior is changed. Passing characterization tests establish the bottleneck, not a production repair. Any production repair requires failing regression acceptance tests before implementation.
  """

  Scenario: Fixture streams separate publication cost from provider latency
    Given prebuilt text and thinking deltas with no network or generation delay
    When identical streams use direct publication and durable background output
    Then the report includes throughput latency and per-delta durability work
    And cold recovery contains every durably published delta exactly once

  Scenario: Cold admission reproduces historical replay amplification
    Given fixed-size journal fixtures in growing batches
    When identical records use retained writers and cold writers per delta
    Then cold historical reads follow the triangular growth formula
    And retained writers avoid repeated historical reads

  Scenario: A stalled admission prevents cooperative executor progress
    Given a background stream on a single-thread executor and a locked journal writer
    When a ready sibling task waits while a fixture delta is emitted
    Then the sibling cannot progress before an external thread releases the writer
    And publication happens only after durable admission
