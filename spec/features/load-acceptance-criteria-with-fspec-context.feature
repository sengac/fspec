@done
@CLI-019
@rust
@querying
@cli
Feature: Load acceptance criteria with fspec context
  """
  fspec context replaces show-acceptance-criteria, get-scenarios and search-scenarios (product owner, 2026-11-08); the three commands are removed with no alias.
  Core: rust/fspec-core/src/commands/context.rs run(args_json, project_root) returns one rendered page; its submodules select scenarios (a cucumber tag expression parsed by gherkin::tagexpr::TagOperation and evaluated over the scenario's own, Rule and Feature tags, plus the exact --text, --regex and --feature filters), split them into pages by a character budget and render markdown, text or json.
  Scenario, Background and description text come from io::gherkin_format's canonical formatter, so a criterion reads exactly like Gherkin.
  CLI: rust/fspec/src/context.rs holds a clap::Args + Serialize Flags struct flattened into Mode::Context (the CLI-017 pattern); the LLM dispatcher sends the same JSON to the same run function.
  The init docs, their fixtures and the bootstrap doc have a 'Loading acceptance criteria' section; every mention of the removed commands names fspec context.
  """

  Background: User Story
    As a AI agent or developer working on an fspec card
    I want to load the acceptance criteria I need with fspec context, selected by a cucumber tag expression and split into pages I can read one at a time
    So that I start every card from the exact scenarios, steps and tags without reading feature files by hand or running three different commands

  Scenario: A tag expression with and and not selects scenarios
    Given the login and billing fixture features
    When I run "fspec context --tags '@auth and not @wip'"
    Then the page lists only the scenario "Valid login"

  Scenario: A tag expression with or and parentheses lists scenarios by feature path then line
    Given the login and billing fixture features
    When I run "fspec context --tags '(@smoke or @billing) and not @wip'"
    Then the page lists "Valid login", "Invoice totals" and "Refund a paid invoice" in that order

  Scenario: Scenarios inherit the tags of their Feature and Rule
    Given the login and billing fixture features
    When I run "fspec context --tags @money"
    Then the page lists only the scenario "Refund a paid invoice"
    When I run "fspec context --tags @auth"
    Then the page lists "Valid login" and "Wrong password" in that order

  Scenario: A criterion carries its location, tags, steps, doc strings and tables under its feature's header
    Given the login and billing fixture features
    When I run "fspec context --tags '@smoke or @billing'"
    Then the Login feature's tags, name, description and Background are printed once, above "Valid login"
    And "Valid login" shows "spec/features/a-login.feature:14", its tag @smoke and the inherited tags @auth @CLI-001
    And "Invoice totals" shows its data table
    And "Refund a paid invoice" shows its Rule, its doc string and the inherited tags @billing @money

  Scenario: The text format prints the same criteria without markdown
    Given the login and billing fixture features
    When I run "fspec context --tags @smoke --format text"
    Then the page shows the Login feature's header and "Valid login" with its location and inherited tags
    And the page has no markdown heading or code fence

  Scenario: The LLM dispatcher runs context with the same arguments
    Given the login and billing fixture features
    When the dispatcher runs the command context with the arguments {"tags": "@auth and not @wip"}
    Then the result is the same page "fspec context --tags '@auth and not @wip'" prints

  Scenario: An invalid tag expression is an exact error
    Given the login and billing fixture features
    When I run "fspec context --tags '@a not @b'"
    Then the command fails with an error starting "invalid tag expression '@a not @b':"

  Scenario: The text filter matches scenario names and steps, doc strings and table cells, ignoring case
    Given the login and billing fixture features
    When I run "fspec context --text 'WRONG PASSWORD'"
    Then the page lists only the scenario "Wrong password"
    When I run "fspec context --text r-7"
    Then the page lists only the scenario "Refund a paid invoice"
    When I run "fspec context --text PEN"
    Then the page lists only the scenario "Invoice totals"

  Scenario: The regex filter matches scenario names and steps, and an invalid pattern is an exact error
    Given the login and billing fixture features
    When I run "fspec context --regex 'Refund (a|an) paid'"
    Then the page lists only the scenario "Refund a paid invoice"
    When I run "fspec context --regex '('"
    Then the command fails with an error starting "invalid regex '(':"

  Scenario: The feature filter selects one feature and an unknown feature is an exact error
    Given the login and billing fixture features
    When I run "fspec context --feature b-billing"
    Then the page lists "Invoice totals" and "Refund a paid invoice" in that order
    When I run "fspec context --feature missing"
    Then the command fails with the error "feature 'missing' does not exist in spec/features/"

  Scenario: A page holds exactly as many characters as the page size allows
    Given the login and billing fixture features
    And the page body of "fspec context --feature b-billing" is N characters
    When I run "fspec context --feature b-billing --page-size N"
    Then both billing scenarios are on page 1 of 1
    When I run "fspec context --feature b-billing --page-size N-1"
    Then "Invoice totals" is on page 1 of 2 and "Refund a paid invoice" is on page 2 of 2

  Scenario: A scenario larger than the page size is alone on its page
    Given an import feature with the scenarios "Short one", "Long import" and "Short two" where "Long import" is longer than 300 characters
    When I run "fspec context --feature c-import --page-size 300"
    Then "Short one" is on page 1 of 3, "Long import" on page 2 of 3 and "Short two" on page 3 of 3

  Scenario: Every page ends with its number and the exact command for the next page
    Given the login and billing fixture features
    When I run "fspec context --tags '@auth or @smoke' --page-size 300"
    Then the page lists only the scenario "Valid login"
    And it ends with "page 1 of 2" and "Next page: fspec context --tags '@auth or @smoke' --page-size 300 --page 2"
    When I run "fspec context --tags '@auth or @smoke' --page-size 300 --page 2"
    Then the page lists only the scenario "Wrong password"
    And it ends with "page 2 of 2" and no next page command

  Scenario: A page past the last page is an exact error
    Given the login and billing fixture features
    When I run "fspec context --tags @auth --page-size 300 --page 3"
    Then the command fails with the error "page 3 does not exist: there are 2 pages"

  Scenario: The json format prints one page with its number, the page count, the next command and the items
    Given the login and billing fixture features
    When I run "fspec context --tags @auth --format json --page-size 300"
    Then the JSON has page 1, pages 2 and nextCommand "fspec context --tags @auth --format json --page-size 300 --page 2"
    And its features hold the Login header and its items hold "Valid login" with its file, line, tags, inherited tags and Gherkin text
    When I run "fspec context --tags @auth --format json --page-size 300 --page 2"
    Then the JSON has page 2, pages 2 and a null nextCommand

  Scenario: No matching scenario is one empty page
    Given the login and billing fixture features
    When I run "fspec context --tags @nothing"
    Then the page says "No scenarios match." and ends with "page 1 of 1" and no next page command

  Scenario: The three replaced commands are unknown commands
    Given the fspec command line
    When I run the removed commands show-acceptance-criteria, get-scenarios and search-scenarios --query login
    Then each fails as an unknown command
    And the dispatcher rejects each as an unknown command

  Scenario: Help and the workflow docs teach fspec context
    Given the fspec command line
    When I run "fspec context --help"
    Then it lists --tags, --text, --regex, --feature, --format, --page and --page-size with examples
    And every init doc and the bootstrap doc have a "Loading acceptance criteria" section that runs fspec context
    And the research reminder of create-bug suggests "fspec context --text"
