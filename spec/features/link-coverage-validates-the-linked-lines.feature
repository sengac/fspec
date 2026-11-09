@done
@BUG-452
@cli
@coverage-tracking
@rust
Feature: link-coverage validates the steps in the linked lines
  """
  rust/fspec-core/src/commands/link_coverage/step.rs validate_step_consistency
  returned Ok when the feature file could not be read ("forward planning") or
  the scenario was not in it, read the test file with unwrap_or_default (an
  unreadable file validated as empty), and matched @step comments against the
  WHOLE test file, so a mapping to lines holding none of the scenario's steps
  was accepted when the steps appeared anywhere in the file (how coverage
  ranges drift). The matcher was also duplicated in
  update_work_unit_status/step_docstrings/steps.rs.
  Now the linked --test-lines range (with the scenario's existing mappings in
  the same test file — steps may be spread over several functions) must hold
  every step; unreadable files, unparsable ranges and a scenario missing from
  the feature are errors; the status check calls link_coverage::step::missing_steps.
  Ranges use audit_coverage_drift::line_range (BUG-375).
  Tests: rust/fspec-core/tests/bug452_link_coverage_validates_the_linked_lines.rs.
  AUDIT-005: the status check's copy (step_docstrings) is gone; the workflow gates run fspec audit's
  coverage area, which uses the same matcher.
  """

  Background: User Story
    As a maintainer linking test coverage
    I want to have link-coverage check that the linked lines hold the scenario's @step comments
    So that coverage mappings cannot point at the wrong lines or at files it could not read

  Scenario: Linking lines that hold none of the scenario's steps is refused
    Given a test file whose @step comments for scenario "Log in" are on lines 4 to 6
    When lines 1-2 are linked to "Log in"
    Then link-coverage fails naming the missing steps

  Scenario: Steps spread over a new range and an existing mapping are accepted
    Given "Log in" already maps lines 4-4 holding its Given step
    When lines 5-6 holding its When and Then steps are linked
    Then the link succeeds

  Scenario: A scenario missing from the feature file is refused
    Given the coverage file lists scenario "Log in" but the feature file no longer has it
    When a test range is linked to "Log in"
    Then link-coverage fails saying the scenario is not in the feature file
