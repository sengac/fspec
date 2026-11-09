@done
@BUG-378
@cli
@validation
@workflow
Feature: Prefill gate feature file read errors
  """
  update-work-unit-status runs the prefill gate (check_prefill in rust/fspec-core/src/commands/update_work_unit_status/
  prefill.rs) for every forward move except to blocked. The gate, the scenario-existence and bug linked-feature checks
  (review.rs) and the temporal gate (temporal.rs) find the work unit's feature files with linked_feature_files. Before
  BUG-378 that scan returned no features when spec/features could not be read and skipped directory entries and
  feature files that could not be read, so the prefill gate passed vacuously. Now it returns BUG-360's typed
  FeatureScanError naming the path and every caller fails the status move with it. A missing spec/features
  directory (NotFound) still means "no feature files".
  """

  Background: User Story
    As a developer moving a work unit through fspec status gates
    I want to have the prefill gate refuse when a feature file or spec/features cannot be read, naming the path
    So that an unreadable feature never lets placeholder text slip past the gate

  Scenario: A feature file that is not valid UTF-8 fails the prefill gate and names the file
    Given bug "BUG-1" is in backlog
    And "spec/features/binary.feature" contains bytes that are not valid UTF-8
    When the user moves "BUG-1" to specifying
    Then the move fails with an error naming "spec/features/binary.feature"
    And "BUG-1" stays in backlog

  Scenario: An unreadable spec features directory fails the prefill gate and names it
    Given bug "BUG-1" is in backlog
    And "spec/features" is a file instead of a directory
    When the user moves "BUG-1" to specifying
    Then the move fails with an error naming "spec/features"
    And "BUG-1" stays in backlog
