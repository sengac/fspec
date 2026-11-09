@done
@BUG-501
@cli
@coverage-tracking
@rust
Feature: fspec audit reads comma-list line ranges
  """
  link-coverage accepts (and its help documents) comma-separated test line
  ranges such as 10-20,30-40 or 45,46,47 and validates them part by part, but
  audit_coverage_drift::line_range only read "a-b" or "a", so audit-coverage
  reported those mappings as unreadable (2 at HEAD, found by the COV-069 prep).
  Now audit_coverage_drift::line_ranges parses the comma list for both
  commands; the parts of a mapping together must hold the scenario's steps,
  and a part that is not a number or range is still unreadable.
  Code: rust/fspec-core/src/commands/audit_coverage_drift.rs,
  rust/fspec-core/src/commands/link_coverage/step.rs.
  Tests: rust/fspec-core/tests/bug501_audit_coverage_reads_comma_list_ranges.rs.
  Since AUDIT-003 this check is part of fspec audit's coverage area; audit-coverage was removed.
  """

  Background: User Story
    As a maintainer auditing coverage
    I want to have fspec audit read every line range form link-coverage accepts, including comma lists
    So that a mapping link-coverage accepted is never reported as unreadable by the audit

  Scenario: A comma-list mapping whose parts hold the steps passes the audit
    Given scenario "Log in" maps test lines "4-4,5-6" which together hold its @step comments
    When the coverage is audited
    Then the audit reports all mappings valid

  Scenario: A comma-list mapping that lacks a step is reported as drifted
    Given scenario "Log in" maps test lines "4-4,1-2" which lack its When step
    When the coverage is audited
    Then the audit fails naming the missing When step

  Scenario: A comma list with a part that is not a range is unreadable
    Given scenario "Log in" maps test lines "4-6,x"
    When the coverage is audited
    Then the audit fails reporting an unreadable line range
