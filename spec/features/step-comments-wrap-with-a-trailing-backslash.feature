@done
@BUG-500
@cli
@coverage-tracking
@rust
Feature: Step comments wrap with a trailing backslash
  """
  The @step matcher (extract_step_comments in
  rust/fspec-core/src/commands/link_coverage/step.rs, shared by link-coverage,
  fspec audit and the work-unit status check) read one line per comment, so
  a @step comment wrapped onto a second comment line never matched its step
  (many COV-069 cosmetic drifts). The supported continuation form is a
  trailing backslash: a @step line ending in a backslash continues on the next
  comment line; the comment marker (//, #, --, or a block comment's leading *)
  and surrounding spaces are dropped and the parts are joined with one space.
  A comment line after a @step line without the backslash is not part of the
  step, so explanatory comments stay separate. Documented in the bootstrap
  (@step Comment Matching Rules) and the link-coverage help notes.
  Tests: rust/fspec-core/tests/bug500_step_comments_wrap_with_a_backslash.rs.
  """

  Background: User Story
    As a maintainer writing step comments
    I want to wrap a long @step comment over several comment lines
    So that long steps match without over-long lines or showing as drift

  Scenario: A step comment wrapped with a trailing backslash matches the step
    Given a test whose @step comment for a long step ends with a backslash and continues on the next comment line
    When the coverage is audited
    Then the audit reports all mappings valid

  Scenario: A hash comment can continue twice
    Given a Python test whose @step comment continues over three comment lines with backslashes
    When the coverage is audited
    Then the audit reports all mappings valid

  Scenario: A comment line after a step comment without a backslash is not joined
    Given a test whose @step comment holds only the start of a long step and the rest is on the next comment line without a backslash
    When the coverage is audited
    Then the audit fails naming the long step
