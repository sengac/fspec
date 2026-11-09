@done
@BUG-506
@cli
@feature-management
@rust
Feature: update-step text can carry the new keyword
  """
  The update-step help says --text may start with a keyword to override the
  step's keyword ("New step text (without keyword, or with keyword to
  override)"), but rust/fspec-core/src/commands/update_step.rs stripped a
  leading keyword from --text and kept the old keyword; only --keyword changed
  it (found in COV-069 B05-B08). Now a keyword at the start of --text becomes
  the step's keyword when --keyword is not given; a --keyword that differs
  from the keyword in --text is refused.
  Tests: rust/fspec-core/tests/bug506_update_step_text_carries_the_keyword.rs.
  """

  Background: User Story
    As a maintainer editing feature files with fspec
    I want to change a step's keyword by starting update-step --text with the new keyword, as the help says
    So that update-step does what its help documents

  Scenario: A keyword at the start of the new text replaces the step keyword
    Given a scenario with the step "Given I log in"
    When I update that step with the text "And I log in again"
    Then the step reads "And I log in again"

  Scenario: Text without a keyword keeps the current keyword
    Given a scenario with the step "Given I log in"
    When I update that step with the text "I log in again"
    Then the step reads "Given I log in again"

  Scenario: A keyword option that contradicts the keyword in the text is refused
    Given a scenario with the step "Given I log in"
    When I update that step with the keyword "When" and the text "Then I log in again"
    Then update-step fails naming both keywords
    And the step still reads "Given I log in"
