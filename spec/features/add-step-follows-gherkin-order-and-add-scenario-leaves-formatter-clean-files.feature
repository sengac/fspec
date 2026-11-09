@done
@cli
@feature-management
@bug-fix
@rust
@BUG-620
Feature: add-step follows Gherkin order and add-scenario leaves formatter-clean files
  """
  add-step (rust/fspec-core/src/commands/add_step.rs) places a step by the scenario's blocks, read from the gherkin crate's step types (an And or But step carries the type of the block before it): a given, when or then step replaces that keyword's placeholder, else follows the last step of its block, else precedes the first step of a later block, else goes last; an and or but step follows the last real (non-placeholder) step. Every insertion point skips the data table or doc string of the step it follows. add-scenario (rust/fspec-core/src/commands/add_scenario.rs) joins the new scenario with exactly one blank line before it and one newline at the end of the file, and inserts before a Scenario Outline's tag lines, so fspec's Gherkin formatter (io::gherkin_format::format_feature) leaves the file unchanged. No --position option: the card allows Gherkin-order placement instead, and a new flag would change the frozen REFAC-020 CLI declarations.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A given, when or then step replaces that keyword's placeholder step (the bracketed text create-feature and add-scenario write) when the scenario has one
  #   2. Otherwise a given, when or then step goes right after the last step of its block (Given, When, Then; an And or But step belongs to the block before it), or before the first step of a later block when its block is empty, or at the end
  #   3. An and or but step goes right after the scenario's last real (non-placeholder) step; with no real step it fails with an exact error and the file is unchanged
  #   4. A step is inserted after the data table or doc string of the step it follows, never between a step and its argument
  #   5. Background steps never count as the scenario's steps
  #   6. add-scenario leaves a formatter-clean file (fspec's own Gherkin formatter output equals the file): one blank line before the new scenario, one newline at the end of the file, and a tagged Scenario Outline keeps its tags
  #   7. No new --position option: placement follows Gherkin order (the card allows either; adding a flag would change the frozen REFAC-020 CLI declarations)
  #
  # ========================================
  Background: User Story
    As a AI agent writing Gherkin with fspec commands
    I want to add steps and scenarios that land exactly where Gherkin order puts them
    So that I never hand-fix step order, detached data tables or blank lines after fspec edits a feature

  Scenario: A second Given step goes after the first one and before the When block
    Given a feature whose Background has the step 'Given the app is running' and whose scenario 'Log in' has the three placeholder steps
    When I add the given step 'I am on the login page' and then the given step 'I have an account'
    Then the steps of 'Log in' are 'Given I am on the login page', 'Given I have an account', the When placeholder, the Then placeholder
    And the Background still has only the step 'Given the app is running'

  Scenario: An And step continues the last real step, not the placeholder Then
    Given the scenario 'Log in' has the steps 'Given I am on the login page', the When placeholder, the Then placeholder
    When I add the and step 'I have an account'
    Then the steps of 'Log in' are 'Given I am on the login page', 'And I have an account', the When placeholder, the Then placeholder

  Scenario: A When step goes after the When block's last step even when that step is a But
    Given the scenario 'Log in' has the steps 'Given a', 'When b', 'But c', 'Then d'
    When I add the when step 'e'
    Then the steps of 'Log in' are 'Given a', 'When b', 'But c', 'When e', 'Then d'

  Scenario: A step never lands between a step and its data table
    Given the scenario 'Log in' has the step 'Given these users:' with a data table, then the Then placeholder
    When I add the given step 'I am an admin'
    Then the steps of 'Log in' are 'Given these users:', 'Given I am an admin', the Then placeholder
    And 'Given these users:' still has its data table

  Scenario: A Given step goes before the When block when the scenario has no Given step
    Given the scenario 'Log in' has the steps 'When b', 'Then d'
    When I add the given step 'a'
    Then the steps of 'Log in' are 'Given a', 'When b', 'Then d'

  Scenario: An And step with no real step to continue fails with an exact error
    Given the scenario 'Log in' has the three placeholder steps
    When I add the and step 'x'
    Then add-step fails with 'Cannot add an And step to scenario "Log in": it has no real step to continue. Add a given, when or then step first.'
    And the feature file is unchanged

  Scenario: add-scenario at the end of a file leaves it formatter-clean
    Given a formatter-clean feature file that ends with one newline
    When I add the scenario 'New one'
    Then the file equals fspec's formatted form of it
    And the file ends with exactly one newline

  Scenario: add-scenario before a tagged Scenario Outline keeps the outline's tags
    Given a formatter-clean feature file whose last scenario is a Scenario Outline tagged @slow
    When I add the scenario 'New one'
    Then the scenario 'New one' has no tags and the Scenario Outline still has the tag @slow
    And the file equals fspec's formatted form of it
