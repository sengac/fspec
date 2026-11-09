@done
@BUG-435
@cli
@validation
@workflow
Feature: Temporal gate checks coverage-mapped test files
  """
  The testing-to-implementing temporal check compares the work unit's test
  files with the time it entered testing. find_test_files only walked
  src/**/__tests__/**/*.test.ts (the TypeScript port), so in Rust and other
  non-TS projects it found nothing and passed vacuously. Candidate test
  files are now the files mapped in the linked features' .feature.coverage
  (any language) plus the legacy TypeScript glob; as before, only files
  that mention the work-unit id are checked. A coverage file that cannot be
  read or parsed is an error naming it (BUG-386). Tests:
  rust/fspec-core/tests/bug435_temporal_gate_mapped_test_files.rs.
  """

  Background: User Story
    As a developer in a non-TypeScript project using fspec's workflow gates
    I want the temporal ordering gate to see my test files
    So that tests written before the testing phase are caught in any language

  Scenario: A mapped test file written before testing blocks the move to implementing
    Given story "STORY-1" is in testing since a known time
    And the linked feature's coverage maps "rust/tests/gate.rs"
    And "rust/tests/gate.rs" mentions "STORY-1" and was last modified before STORY-1 entered testing
    When the user moves "STORY-1" to implementing
    Then the move fails with a temporal ordering violation naming "rust/tests/gate.rs"

  Scenario: A mapped test file that does not mention the work unit is not checked
    Given story "STORY-1" is in testing since a known time
    And the linked feature's coverage maps "rust/tests/other.rs"
    And "rust/tests/other.rs" does not mention "STORY-1" and was last modified before STORY-1 entered testing
    When the user moves "STORY-1" to implementing
    Then the move succeeds
