@done
@BUG-360
@cli
@validation
@workflow
Feature: Status gate feature file read errors
  """
  update-work-unit-status runs the step-docstring gate (rust/fspec-core/src/commands/update_work_unit_status/
  step_docstrings.rs) for non-task moves to implementing and validating. The gate finds the work unit's feature
  files with matching_features (step_docstrings/matching.rs), which scans the top-level spec/features/*.feature files.
  Before BUG-360 the scan returned no features when spec/features could not be read and skipped files that could
  not be read or parsed, so the gate passed vacuously. Now every read_dir, directory entry, file read and Gherkin
  parse error is a typed FeatureScanError naming the path, and the status move fails with it. A missing
  spec/features directory (NotFound) is the one case that means "no feature files", and the gate has nothing to
  check.
  AUDIT-005: the step-docstring gate is gone; the audit gate (update_work_unit_status/audit_gate.rs)
  audits the work unit's features. The prefill gate's scan (update_work_unit_status/prefill.rs,
  FeatureScanError) fails on an unreadable spec/features or feature file, naming the path, and a
  feature file that does not parse stays in the work unit's audit scope (audit.rs work_unit_features),
  where the coverage area reports it as unparsed-feature, naming the file.
  """

  Background: User Story
    As a developer moving a work unit through fspec status gates
    I want to have the gate refuse when a feature file or spec/features cannot be read or parsed, naming the path
    So that a broken feature file never lets a work unit pass its gates vacuously

  Scenario: An unparsable feature file fails the status move and names the file
    Given bug "BUG-1" is in implementing
    And "spec/features/broken.feature" contains "not a feature file"
    When the user moves "BUG-1" to validating
    Then the move fails with an error naming "spec/features/broken.feature"
    And "BUG-1" stays in implementing

  Scenario: A feature file that is not valid UTF-8 fails the status move and names the file
    Given bug "BUG-1" is in implementing
    And "spec/features/binary.feature" contains bytes that are not valid UTF-8
    When the user moves "BUG-1" to validating
    Then the move fails with an error naming "spec/features/binary.feature"
    And "BUG-1" stays in implementing

  Scenario: An unreadable spec/features directory fails the status move and names it
    Given bug "BUG-1" is in implementing
    And "spec/features" is a file instead of a directory
    When the user moves "BUG-1" to validating
    Then the move fails with an error naming "spec/features"
    And "BUG-1" stays in implementing
