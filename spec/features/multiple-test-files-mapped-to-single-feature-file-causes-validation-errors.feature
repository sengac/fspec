@done
@critical
@validator
@validation
@VAL-005
@BUG-357
Feature: Multiple test files mapped to single feature file causes validation errors
  """
  BUG-357 corrected the rule (product owner, 2026-11-04): one test file maps to one feature.
  A feature may be covered by several test files; a test file must not be mapped by more
  than one feature. update-work-unit-status (fspec-core
  commands/update_work_unit_status/step_docstrings.rs) collects the feature's test files and
  refuses the transition when another feature's coverage maps any of them, naming both;
  fspec audit reports the audited feature's shared test files. The shared lookup lives in
  fspec-core commands/coverage_sharing.rs.
  Tests: rust/fspec-core/tests/bug357_one_test_file_maps_to_one_feature.rs.
  AUDIT-005: the transition check is now the audit gate (update_work_unit_status/audit_gate.rs): the
  audit's coverage area reports a shared test file as 'coverage shared-test-file <feature> <file>:
  also mapped by <other feature>' and the move is refused with that line; step_docstrings.rs and
  coverage_sharing.rs are gone.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. One test file maps to one feature: a feature may have several test files
  #   2. A test file that another feature's coverage also maps blocks the transition, naming the other feature
  #   3. fspec audit reports the audited feature's test files that other features also map
  #
  # EXAMPLES:
  #   1. login.feature covered by tests/login_ui.rs and tests/login_api.rs moves on
  #   2. signup.feature also maps tests/login_api.rs: the transition names signup and tests/login_api.rs
  #
  # ========================================
  Background: User Story
    As a AI agent moving work unit through workflow
    I want each test file to belong to exactly one feature
    So that a test's steps are only ever checked against the feature it was written for

  Scenario: A feature covered by several test files moves on
    Given a work unit's feature is covered by two test files that no other feature maps
    When the work unit moves from testing to implementing
    Then the transition succeeds

  Scenario: A test file shared with another feature blocks the transition
    Given a work unit's feature maps a test file that another feature also maps
    When the work unit moves from testing to implementing
    Then the transition is refused
    And the error names the shared test file and the other feature

  Scenario: fspec audit reports a test file shared with another feature
    Given a feature maps a test file that another feature also maps
    When its coverage is audited
    Then the audit fails naming the shared test file and the other feature
