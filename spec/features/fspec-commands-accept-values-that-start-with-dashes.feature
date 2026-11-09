@done
@interactive-cli
@BUG-521
@cli
@rust
Feature: fspec commands accept values that start with dashes
  """
  BUG-496 let link-coverage and unlink-coverage take a --scenario value
  starting with '--', but every other fspec command still parsed a free-text
  value that began with '-' or '--' as an option, e.g.
  'fspec add-rule ID "--replace ..."' failed with "unknown option"; only the
  '--' separator worked. Every free-text argument in
  rust/fspec/src/cli_registration (rules, examples, questions, answers,
  notes, step and scenario text, titles, descriptions, names, user-story
  parts, search patterns) now sets clap's allow_hyphen_values. Id and file
  positionals do not, so an unknown option given in their place is still
  reported as unknown (the REFAC-020 legacy error contract), and
  multi-value arguments do not, so they never swallow the next flag; flags
  without values, --help/-h and the '--' separator are unchanged.
  Tests: rust/fspec/tests/bug521_values_starting_with_dashes.rs.
  """

  Background: User Story
    As a maintainer recording rules, examples and step text with fspec
    I want to pass free text that starts with dashes to any fspec command
    So that texts such as '--replace keeps implementation mappings' are recorded instead of failing as unknown options

  Scenario: Work unit texts may start with dashes
    Given a project with a bug work unit
    When I add a rule, an example, a question, an architecture note and an answer that each start with "--"
    Then every command succeeds
    And the work unit shows each text verbatim

  Scenario: Step text and titles may start with dashes
    Given a feature file with scenario "S" and step "Given a"
    When I add a step, update a step's text and rename the work unit with texts that start with "--"
    Then every command succeeds
    And the feature file and the board hold each text verbatim

  Scenario: Flags and help keep working
    Given a project with a bug work unit
    When I run a command with a flag that takes no value and run another with --help
    Then the flag is applied and the help is shown
