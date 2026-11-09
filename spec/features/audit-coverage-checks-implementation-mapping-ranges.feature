@done
@BUG-515
@cli
@coverage-tracking
@rust
Feature: fspec audit checks implementation mapping ranges
  """
  audit-coverage checked that implementation files exist but never looked at
  the lines an implementation mapping lists, so mappings left behind when a
  file shrank passed silently (found in COV-069 B23-B19). Now every
  implementation mapping's lines (a number list, or a range string read with
  audit_coverage_drift::line_ranges) must be non-empty and lie between line 1
  and the file's last line; otherwise the audit reports the mapping with the
  file's line count and exits 1. Mappings do not record the symbol they name,
  so an in-range mapping that now points at other code cannot be detected
  without a mapping format change (follow-up).
  Code: rust/fspec-core/src/commands/audit_coverage.rs,
  rust/fspec-core/src/commands/audit_coverage_impl.rs.
  Tests: rust/fspec-core/tests/bug515_audit_coverage_checks_impl_ranges.rs.
  Since AUDIT-003 this check is part of fspec audit's coverage area; audit-coverage was removed.
  """

  Background: User Story
    As a maintainer relying on coverage audits
    I want to have fspec audit check that every implementation mapping's lines lie inside its file
    So that implementation mappings left behind by edits that shortened a file are reported instead of passing silently

  Scenario: An implementation mapping past the end of its file is reported
    Given an implementation file of 20 lines mapped to lines 18 to 25
    When the coverage is audited
    Then the audit fails saying the mapping lies outside the file of 20 lines

  Scenario: An implementation mapping inside its file passes
    Given an implementation file of 20 lines mapped to the range string "5-9"
    When the coverage is audited
    Then the audit reports all mappings valid

  Scenario: An implementation mapping that lists no lines is reported
    Given an implementation mapping with an empty line list
    When the coverage is audited
    Then the audit fails saying the mapping lists no lines
