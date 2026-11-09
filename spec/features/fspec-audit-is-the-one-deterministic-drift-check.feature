@done
@AUDIT-003
@cli
@coverage-tracking
@rust
Feature: fspec audit is the one deterministic drift check
  """
  One language-agnostic fspec audit replaces fspec check and fspec
  audit-coverage. It reports only exact facts over the areas gherkin, tags,
  format, coverage and spec, one problem per line with the command that repairs
  or re-checks it, and --json gives the same problems. Gherkin and tag facts
  come from the functions validate and validate-tags use; coverage facts reuse
  the BUG-375/501/509/515/525 checks, the BUG-357 sharing check and the
  BUG-485/557 coverage sync check, reading every coverage sidecar once. @step
  counts only as the first token after a comment marker at the start of a line.
  Core: rust/fspec-core/src/commands/audit.rs. Tests:
  rust/fspec-core/tests/audit003_one_deterministic_drift_check.rs.
  """

  Background: User Story
    As a maintainer of an fspec project
    I want to run one deterministic fspec audit that reports every exact coverage, spec, Gherkin, tag and format problem with the command that fixes it
    So that drift is found by one command instead of check, audit-coverage and looser copies

  Scenario: A drifted test mapping is reported with the command that relinks it
    Given scenario "Log in" maps tests/login_test.rs:10-20 but its @step comments sit on lines 14-24
    When fspec audit runs
    Then it prints a "coverage drifted login" line for tests/login_test.rs:10-20 ending in the fspec audit --fix command
    And it exits 1

  Scenario: A coverage file without its feature is reported
    Given spec/features/old.feature.coverage exists without old.feature
    When fspec audit runs
    Then it prints a "spec orphan-coverage old" line
    And it exits 1

  Scenario: A feature with no scenarios is reported
    Given a feature "empty" with no scenarios
    When fspec audit runs
    Then it prints a "spec no-scenarios empty" line

  Scenario: Step text inside a string literal does not cover a step
    Given a test file whose only "@step Then they see the board" text is inside a string literal
    When fspec audit runs
    Then it reports the step "Then they see the board" as missing from the mapped lines

  Scenario: The area option limits the areas checked
    Given a project whose only problem is a feature file that needs formatting
    When fspec audit runs with --area tags
    Then it prints no problems
    And it exits 0

  Scenario: A clean project prints an empty problem list as JSON
    Given a project with no problems
    When fspec audit runs with --json
    Then it prints {"problems":[],"exitCode":0}

  Scenario: The feature option limits the scope
    Given features "login" and "other" both have problems
    When fspec audit runs with --feature login
    Then it reports only problems of "login"

  Scenario: A test file mapped by two features is reported for each
    Given features "login" and "admin" both map tests/shared_test.rs
    When fspec audit runs
    Then it prints a "coverage shared-test-file" line for each feature naming the other

  Scenario: check and audit-coverage are no longer commands
    Given the fspec command line
    When the removed check and audit-coverage commands are parsed
    Then each is an unknown command
