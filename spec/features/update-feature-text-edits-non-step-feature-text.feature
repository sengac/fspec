@BUG-582
@done
@BUG-507
@cli
@feature-management
@rust
Feature: update-feature-text edits non-step feature text
  """
  fspec could update scenarios and steps, but nothing changed a feature's
  title, description, # comments, Rule blocks or data table / Examples cells,
  so stale text stayed after COV-069 reconciled the steps unless someone
  hand-edited the .feature file. New command
  'fspec update-feature-text <feature> --find <text> --replace <text>'
  (rust/fspec-core/src/commands/update_feature_text.rs) replaces exactly one
  occurrence of the text, which may span lines. It refuses no match or
  several matches, a match touching a step line or a scenario/background
  header (update-step and update-scenario keep @step comments and coverage in
  sync), and a result that no longer parses or changes the scenarios and
  steps. The stale text listed on BUG-507 is then fixed with it.
  Tests: rust/fspec-core/tests/bug507_update_feature_text.rs.
  """

  Background: User Story
    As a maintainer keeping feature files true to the code
    I want to change a feature's title, description, comments, rules and table cells with an fspec command
    So that stale text can be fixed without hand-editing .feature files

  Scenario: Description, comment and table text are replaced
    Given a feature with a description, a comment rule and an Examples table
    When I replace a description sentence, a comment rule and an Examples cell with update-feature-text
    Then each text is replaced
    And the scenarios and steps are unchanged

  Scenario: Text that occurs more than once is refused
    Given a feature in which the text "the board" occurs twice
    When I replace "the board" with update-feature-text
    Then update-feature-text fails naming 2 matches
    And the file is unchanged

  Scenario: Text that is not in the feature is refused
    Given a feature without the text "nowhere"
    When I replace "nowhere" with update-feature-text
    Then update-feature-text fails saying the text was not found

  Scenario: Step lines and scenario headers are refused
    Given a feature with the step "Given a user" in scenario "Log in"
    When I replace "Given a user" and then "Scenario: Log in" with update-feature-text
    Then both replacements fail pointing at update-step and update-scenario
    And the file is unchanged

  Scenario: A replacement that breaks the Gherkin is refused
    Given a feature titled "Login"
    When I replace "Feature: Login" with text that drops the Feature keyword
    Then update-feature-text fails saying the result is not valid Gherkin
    And the file is unchanged

  Scenario: The listed stale feature text is fixed
    Given the stale texts listed on BUG-507
    When the repository's feature files are read
    Then none of those stale texts remains
