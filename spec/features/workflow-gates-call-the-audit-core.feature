@done
@AUDIT-005
@cli
@coverage-tracking
@workflow
@rust
Feature: Workflow gates call the audit core
  """
  update-work-unit-status runs the audit core fspec audit uses over the work
  unit's features (its linked features, else the features tagged with its ID):
  the coverage area when a card moves to implementing or validating (tasks are
  exempt, as before), the coverage and spec areas when it moves to done. Any
  problem blocks the move, and the error lists the problems in fspec audit's
  line format with their fix commands, so the gate and fspec audit never
  disagree. The old whole-file @step check (step_docstrings) and the separate
  stale-entry and uncovered-scenario checks (coverage_gate) are gone. The only
  rule the gate adds is that every live scenario's test mappings have an
  implementation mapping before validating, checked on the coverage the audit
  parsed. Uncovered scenarios block (product owner, 2026-11-07). The audit gate
  runs before the post-status virtual hooks (supervisor's conservative choice).
  A feature file that cannot be read or parsed stays in a work unit's scope
  (it may carry the tag), and the coverage area reports a feature that does not
  parse as unparsed-feature, so no gate passes it (BUG-360). Core: rust/fspec-core/src/commands/audit.rs (work_unit_audit) and
  rust/fspec-core/src/commands/update_work_unit_status/audit_gate.rs. Tests:
  rust/fspec-core/tests/audit005_workflow_gates_call_the_audit_core.rs.
  """

  Background: User Story
    As a maintainer of an fspec project
    I want to have update-work-unit-status block a card with every problem fspec audit reports for its features
    So that a card never moves forward with drift that fspec audit would find

  Scenario: A drifted test mapping blocks the move to validating with fspec audit's problem lines
    Given AUTH-001 is implementing and its feature "login" maps scenario "Log in" to tests/login_test.rs:10-20
    And the @step comments of "Log in" sit on lines 30-40
    When AUTH-001 is moved to validating
    Then the move fails with the "coverage drifted login" line and its fix command
    And the error holds exactly the problem lines that fspec audit --work-unit AUTH-001 --area coverage prints
    And AUTH-001 stays implementing

  Scenario: Step text outside the mapped lines blocks the move to implementing
    Given AUTH-001 is testing and its test file holds the steps of "Log in" only outside the mapped lines
    When AUTH-001 is moved to implementing
    Then the move fails with the "coverage drifted login" line

  Scenario: An uncovered scenario blocks the move to implementing
    Given AUTH-001 is testing and its scenario "Log in" has no test mappings
    When AUTH-001 is moved to implementing
    Then the move fails with the "coverage uncovered login" line for "Log in"

  Scenario: A scenario without an implementation mapping blocks the move to validating
    Given AUTH-001 is implementing and its scenario "Log in" maps its exact @step block but no implementation lines
    When AUTH-001 is moved to validating
    Then the move fails naming "Log in" as a scenario without an implementation mapping

  Scenario: A coverage entry for a deleted scenario blocks the move to done
    Given AUTH-001 is validating and its coverage file keeps an entry for the deleted scenario "Log out"
    When AUTH-001 is moved to done
    Then the move fails with the "coverage stale-entry login" line for "Log out"

  Scenario: A feature without scenarios blocks the move to done
    Given AUTH-001 is validating and its linked feature "empty" has no scenarios
    When AUTH-001 is moved to done
    Then the move fails with the "spec no-scenarios empty" line

  Scenario: A clean audit with every scenario mapped moves the card to done
    Given AUTH-001 is validating and fspec audit finds no problem in its features
    And every scenario maps its exact @step block and implementation lines
    When AUTH-001 is moved to done
    Then the move succeeds and AUTH-001 is done

  Scenario: A scenario mapping its exact @step block moves the card to implementing
    Given AUTH-001 is testing and its scenario "Log in" maps its exact @step block
    When AUTH-001 is moved to implementing
    Then the move succeeds and AUTH-001 is implementing

  Scenario: A test mapping without the Then @step blocks the move to validating and names the missing step
    Given AUTH-001 is implementing and the mapped lines of "Log in" hold the Given and When @step comments but not the Then one
    When AUTH-001 is moved to validating
    Then the move fails with the "coverage drifted login" line naming the step 'Then they see the board'
    And AUTH-001 stays implementing

  Scenario: An uncovered scenario of a feature found by its tag blocks the move to done
    Given AUTH-001 is validating, links no features, and the feature "login" tagged @AUTH-001 has the scenario "Log in" with no test mappings
    When AUTH-001 is moved to done
    Then the move fails with the "coverage uncovered login" line for "Log in"
    And AUTH-001 stays validating

  Scenario: A feature without a coverage file blocks the move to done
    Given AUTH-001 is validating and its feature "login" has no coverage file
    When AUTH-001 is moved to done
    Then the move fails with the "coverage uncovered login" line for "Log in"
    And AUTH-001 stays validating

  Scenario: Linked features replace the features found by the work unit's tag
    Given AUTH-001 is validating and links only the feature "custom", whose scenario maps its exact @step block and implementation lines
    And the feature "login" is tagged @AUTH-001 and its scenario "Log in" has no test mappings
    When AUTH-001 is moved to done
    Then the move succeeds and AUTH-001 is done, because "login" is not audited
