@done
@CONT-011
@agent-core
@completion
@integration
Feature: Completion snapshot delegation guards
  """
  NAPI reexports the shared agent-loop and BackgroundOutput rather than owning
  a second CompactionStarted arm. Follow this real delegation and retain the
  metrics accessor, all four logical writer checks and numeric basis assertions.
  Preserve the original BUG-265 source-shape witnesses and scope reconciliation.
  No compaction production policy is changed by these reader corrections.
  """

  Scenario: Shared snapshot delegation retains semantic and writer guards
    Given a background session whose token tracker reads 180000 tokens and whose cached display tokens were fed by the seed emit
    When the auto-compaction path snapshots the pre-compaction token count
    And the manual compaction path reads its tracker-based original token count
    Then both paths report the same 180000-token basis
    And both the agent-loop twin and the NAPI twin route the snapshot through the same shared session accessor
    And all four logical writers retain the shared snapshot and storage accessors
