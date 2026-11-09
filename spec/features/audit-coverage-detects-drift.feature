@done
@validator
@BUG-375
@coverage-tracking
@rust
Feature: fspec audit detects drifted test mappings
  """
  Coverage line mappings are never updated when files change, and
  audit-coverage (rust/fspec-core/src/commands/audit_coverage.rs) only checked
  that mapped files exist, so ranges drifted silently (unified-exec-tool mapped
  run scenarios onto Tool::call, exec-stdin-prompt mapped 29-98 while its test
  spans 31-104). audit-coverage now also reads the feature's scenarios and
  reports every test mapping whose line range lacks an @step comment for one of
  its scenario's steps (audit_coverage_drift.rs); a drifted mapping makes the
  audit exit 1 with a re-link recommendation. The drifted mappings of the two
  named features are re-pointed.
  Tests: rust/fspec-core/tests/bug375_audit_coverage_detects_drift.rs.
  Since AUDIT-003 this check is part of fspec audit's coverage area; audit-coverage was removed.
  """

  Background: User Story
    As a developer relying on coverage mappings
    I want to have fspec audit report test mappings that no longer cover their scenario's steps
    So that drifted line ranges are found instead of silently naming the wrong code

  Scenario: A test mapping that lost its steps is reported as drifted
    Given a feature whose coverage maps a scenario to lines that do not hold its @step comments
    When the coverage is audited
    Then the audit fails naming the mapping and the missing step

  Scenario: A test mapping that holds its steps passes the audit
    Given a feature whose coverage maps a scenario to the lines holding its @step comments
    When the coverage is audited
    Then the audit reports all mappings valid
