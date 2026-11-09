@done
@BUG-486
@cli
@feature-management
@rust
Feature: delete-scenario removes the whole scenario block
  """
  delete-scenario (rust/fspec-core/src/commands/delete_scenario.rs) removed
  from the scenario header to the first blank line after its last step, plus
  only the tag lines directly above the header (BUG-443). A tag line separated
  from the header by a comment stayed behind and attached itself to the next
  scenario (failing with 'invalid Gherkin' when it was the last scenario), and
  the data table of the last step, the doc string, and the Examples of an
  outline were left in the file under the previous scenario.
  Now a scenario's block runs from the tag and comment lines directly above
  its header to just before the next scenario-level header's own tag and
  comment lines, and the whole block is removed; the next scenario keeps its
  own tags and comments, and deleting the last scenario leaves the previous
  content followed by one newline.
  Tests: rust/fspec-core/tests/bug486_delete_scenario_removes_the_whole_block.rs.
  """

  Background: User Story
    As a maintainer editing feature files with fspec
    I want to delete a scenario together with everything that belongs to it
    So that the neighbouring scenarios keep exactly their own tags, comments, tables and examples

  Scenario: Tags and comments above the deleted scenario go with it
    Given a feature where scenario "Two" has a tag line and a comment line above it and scenario "Three" has its own tag
    When I delete scenario "Two"
    Then the tag and comment lines of "Two" are gone
    And scenario "Three" still has exactly its own tag

  Scenario: The data table of the last step goes with the scenario
    Given a feature where the last step of scenario "Two" has a data table
    When I delete scenario "Two"
    Then no row of that table is left in the file

  Scenario: The Examples of a deleted outline go with it
    Given a feature with a Scenario Outline "Three" followed by scenario "Four"
    When I delete scenario "Three"
    Then the Examples table is gone
    And scenario "Four" is unchanged

  Scenario: Deleting the last scenario leaves no dangling lines
    Given a feature whose last scenario has tags interleaved with comments above it and a trailing comment
    When I delete the last scenario
    Then the file ends with the previous scenario's last step followed by one newline

  Scenario: Deleting scenarios by tag also removes comments between their tags
    Given a feature where scenario "Two" has a comment line between its tag lines
    When I delete the scenarios tagged with the tag of "Two"
    Then neither the comment nor the tag lines of "Two" are left in the file
