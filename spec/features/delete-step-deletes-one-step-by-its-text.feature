@BUG-582
@done
@BUG-505
@cli
@feature-management
@rust
Feature: delete-step deletes one step by its text
  """
  'fspec delete-step --help' (copied from the TypeScript help config) said the
  command takes a 0-based step index and a --force flag, but the command
  matches the step by its text, deletes the first match, and rejects --force
  (found in COV-069 B05-B08). It also removed only the step line, leaving the
  step's data table or doc string under the previous step.
  Now the help documents the step text argument (with or without its
  keyword) and has no --force; text that matches more than one step of the
  scenario is refused, naming the matching lines, so the wrong step is never
  deleted; deleting a step takes its data table or doc string with it. There
  is no index-based deletion: steps are addressed by text, like update-step.
  Code: rust/fspec-core/src/commands/delete_step.rs,
  rust/fspec-core/src/help/configs/delete_step.rs.
  Tests: rust/fspec-core/tests/bug505_delete_step_deletes_one_step_by_text.rs.
  """

  Background: User Story
    As a maintainer editing feature files with fspec
    I want to delete a step by its text, as the help describes, without ever deleting the wrong one
    So that delete-step does exactly what its help says

  Scenario: Text that matches two steps is refused
    Given a scenario with the steps "Given a user" and "And a user"
    When I delete the step "a user"
    Then delete-step fails naming both matching lines
    And both steps are still in the scenario

  Scenario: Keyword-qualified text deletes the one step it names
    Given a scenario with the steps "Given a user" and "And a user"
    When I delete the step "And a user"
    Then only "Given a user" is left

  Scenario: Deleting a step deletes its data table
    Given a scenario whose step "Given rows" has a data table
    When I delete the step "Given rows"
    Then no row of that table is left in the file

  Scenario: The help describes the step text argument
    When I read the delete-step help
    Then it describes the step argument as the step text with or without its keyword
    And it mentions neither a step index nor --force
