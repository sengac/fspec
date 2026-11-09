@done
@BUG-525
@cli
@coverage-tracking
@rust
Feature: fspec audit checks test mapping ranges against the file length
  """
  link-coverage refuses a test range that runs past the end of the test file,
  but audit-coverage accepted one such as 10-20 in a 19-line file (found in
  COV-069 validation): the drift check
  (rust/fspec-core/src/commands/audit_coverage_drift.rs) simply read fewer
  lines. BUG-515 added the length check for implementation mappings only. Now
  a test mapping whose range (any part of a comma list) ends past the file's
  last line is reported with the file's line count, and the audit exits 1.
  Tests: rust/fspec-core/tests/bug525_audit_coverage_checks_test_ranges.rs.
  Since AUDIT-003 this check is part of fspec audit's coverage area; audit-coverage was removed.
  """

  Background: User Story
    As a maintainer relying on coverage audits
    I want to have fspec audit refuse test mapping ranges that run past the end of the test file
    So that the audit and link-coverage agree on which test ranges are valid

  Scenario: A test range past the end of the file is reported
    Given a test file of 19 lines whose @step comments are on lines 10 to 12
    And scenario "Log in" maps that file's lines 10-20
    When the coverage is audited
    Then the audit fails saying the mapping lies outside the file of 19 lines

  Scenario: A comma-list part past the end of the file is reported
    Given a test file of 19 lines whose @step comments are on lines 10 to 12
    And scenario "Log in" maps that file's lines "10-12,18-25"
    When the coverage is audited
    Then the audit fails saying the mapping lies outside the file of 19 lines

  Scenario: A test range inside the file passes
    Given a test file of 19 lines whose @step comments are on lines 10 to 12
    And scenario "Log in" maps that file's lines 10-19
    When the coverage is audited
    Then the audit reports all mappings valid
