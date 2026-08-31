@cli
@done
@querying
@research-tools
@CLI-015
Feature: Research registry drops the ast tool

  """
  The `fspec research` command's static registry no longer includes the `ast`
  tool (CLI-015): AST code search moved to the native AstGrep tool (harness
  mode) and the `fspec astgrep` subcommand (CLI mode). The LIST mode
  enumerates the remaining four bundled tools, and `--tool=ast` is rejected
  like any unknown tool with the error pointing at `fspec astgrep`.
  """

  Background: User Story
    As a user listing or selecting research tools
    I want the ast tool removed from the research registry
    So that the registry only offers tools that actually exist

  Scenario: research listing no longer offers the ast tool
    Given an empty project root tempdir with no spec/fspec-config.json and no research env vars
    When I dispatch research with no flags
    Then the dispatcher returns success
    And the result lists the tool "perplexity"
    And the result lists the tool "stakeholder"
    And the result does not list the tool "ast"

  Scenario: research rejects the ast tool as unknown
    Given an empty project root tempdir
    When I dispatch research with tool="ast"
    Then the dispatcher returns an error
    And the error message contains "Research tool not found: ast"
    And the error message lists "perplexity, jira, confluence, stakeholder" as the available bundled tools
