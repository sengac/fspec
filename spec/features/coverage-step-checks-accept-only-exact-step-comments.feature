@done
@BUG-509
@cli
@coverage-tracking
@rust
Feature: Coverage step checks accept only exact step comments
  """
  The step matcher in rust/fspec-core/src/commands/link_coverage/step.rs
  (shared by link-coverage, fspec audit and the update-work-unit-status
  step check) counted plain '// Given ...' comments without @step and matched
  step text by a weighted similarity score (Jaro-Winkler, token set, trigram,
  Jaccard) with thresholds of 0.70-0.85, so mappings that were too short,
  pointed at the wrong test or carried stale step text passed the audit (found
  in COV-069 B14-B27; the helpers checked exact matches with their own script).
  Now a step is covered only by a @step comment whose keyword and text equal
  the step; runs of whitespace count as one space and backslash-continued
  comments (BUG-500) are joined first. The similarity code is removed. The
  link-coverage help notes and the bootstrap state the exact rule.
  Tests: rust/fspec-core/tests/bug509_coverage_step_checks_are_exact.rs.
  """

  Background: User Story
    As a maintainer relying on coverage audits
    I want to have a step count as covered only by a @step comment with its exact text
    So that mappings that are too short, point at the wrong test or carry stale step text fail the audit instead of hiding coverage gaps

  Scenario: A plain comment without the step marker does not cover a step
    Given the mapped lines hold the comment "// Given a user" without the step marker
    When the coverage is audited
    Then the audit reports the step "Given a user" as missing

  Scenario: A step comment with extra words does not cover a step
    Given the mapped lines hold a step comment for "When they open the session list and pick the newest session again" while the step is "When they open the session list and pick the newest session"
    When the coverage is audited
    Then the audit reports the step "When they open the session list and pick the newest session" as missing

  Scenario: A step comment that differs only in letter case does not cover a step
    Given the mapped lines hold a step comment for "Then they see the Board" while the step is "Then they see the board"
    When the coverage is audited
    Then the audit reports the step "Then they see the board" as missing

  Scenario: An exact step comment after any documented comment marker covers the step
    Given the mapped lines hold exact step comments after the markers "//", "#" and "--"
    When the coverage is audited
    Then the audit reports all mappings valid

  Scenario: link-coverage refuses a range whose step comment is only similar
    Given a test whose step comment for "When they open the session list and pick the newest session" reads "When they open the session list and pick the newest session again"
    When that range is linked to the scenario
    Then link-coverage fails naming the step "When they open the session list and pick the newest session"
