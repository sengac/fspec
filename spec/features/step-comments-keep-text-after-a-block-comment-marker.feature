@done
@BUG-499
@cli
@coverage-tracking
@rust
Feature: Step comments keep text after a block-comment marker
  """
  extract_step_comments in rust/fspec-core/src/commands/link_coverage/step.rs
  cut a @step comment's text at the first block-comment terminator, even in a
  line comment, so a step that names a glob (two stars, a slash and a star)
  could never match its test (found by the COV-069 prep). Now only a
  terminator that ends the line is cut (the end of a one-line block comment);
  the text of a line comment is the whole rest of the line. The matcher is
  shared by link-coverage, fspec audit and the work-unit status check.
  Tests: rust/fspec-core/tests/bug499_step_comments_keep_block_comment_markers.rs.
  """

  Background: User Story
    As a maintainer writing step comments
    I want to have a @step comment whose step text contains a block-comment terminator (such as a glob) match its feature step
    So that steps that name globs can be covered without rewording them

  Scenario: A line comment whose step names a glob matches the step
    Given a scenario step that names a glob pattern containing a block-comment terminator
    And a test whose line @step comment repeats that step exactly
    When the coverage is audited
    Then the audit reports all mappings valid

  Scenario: A one-line block comment still matches without its terminator
    Given a test whose @step comment is a one-line block comment for the step "Given a user"
    When the coverage is audited
    Then the audit reports all mappings valid
