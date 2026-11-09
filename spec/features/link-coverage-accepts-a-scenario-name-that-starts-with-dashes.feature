@done
@BUG-496
@cli
@coverage-tracking
@rust
Feature: link-coverage accepts a scenario name that starts with dashes
  """
  clap parsed the value of 'fspec link-coverage <feature> --scenario <name>' as
  an option when the scenario name began with '--' (seen during BUG-477), so
  only '--scenario=<name>' worked. The --scenario option of link-coverage and
  unlink-coverage now takes any value (allow_hyphen_values), so the separated
  form accepts names starting with '-' or '--'.
  Code: rust/fspec/src/cli_registration/mode_08.rs (link-coverage) and
  mode_07.rs (unlink-coverage).
  Tests: rust/fspec/tests/bug496_scenario_names_starting_with_dashes.rs.
  """

  Background: User Story
    As a maintainer linking coverage
    I want to pass any scenario name to fspec link-coverage --scenario, including one that starts with dashes
    So that scenarios whose names begin with '--' can be linked with the separated option form

  Scenario: Linking a scenario whose name starts with two dashes
    Given a feature with scenario "--all removes everything" and a test file holding its @step comments
    When I run fspec link-coverage with --scenario followed by "--all removes everything" as a separate argument
    Then the command succeeds
    And the scenario's coverage maps the test lines

  Scenario: Unlinking a scenario whose name starts with two dashes
    Given scenario "--all removes everything" has a test mapping
    When I run fspec unlink-coverage with --scenario followed by "--all removes everything" as a separate argument and --all
    Then the command succeeds
    And the scenario has no test mappings
