@done
@BUG-386
@cli
@validation
@workflow
Feature: Temporal gate read errors
  """
  check_temporal_ordering (rust/fspec-core/src/commands/update_work_unit_status/temporal.rs)
  compares feature and test file modification times with the time the work
  unit entered the owning state. It used to skip unreadable directories,
  directory entries and test files, files whose mtime could not be read, and
  an unparseable state-entry timestamp, so the gate passed vacuously. Each is
  now an error naming the path or the work unit (the BUG-360/BUG-378
  pattern). Tests: rust/fspec-core/tests/bug386_temporal_gate_read_errors.rs.
  """

  Background: User Story
    As a developer relying on the temporal ordering gate
    I want it to fail when it cannot read what it checks
    So that retroactive work never passes because a file was unreadable

  Scenario: An unreadable test file fails the temporal gate and names the file
    Given story "STORY-1" is in testing since a known time
    And "src/__tests__/broken.test.ts" mentions "STORY-1" in bytes that are not valid UTF-8
    When the user moves "STORY-1" to implementing
    Then the move fails with an error naming "src/__tests__/broken.test.ts"

  Scenario: An unparseable state-entry timestamp fails the temporal gate
    Given story "STORY-1" is in testing with the testing entry timestamp "yesterday"
    When the user moves "STORY-1" to implementing
    Then the move fails with an error naming "STORY-1" and "yesterday"
