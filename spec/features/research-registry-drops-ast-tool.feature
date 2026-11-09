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

  COV-075 (product owner, 2026-11-05: duplicates merge into the owning feature): the listing scenario 'research listing no longer offers the ast tool' was removed here; research-rust-port's 'List mode enumerates the bundled tool registry without ast' already asserts it on the same test.
  """

  Background: User Story
    As a user listing or selecting research tools
    I want the ast tool removed from the research registry
    So that the registry only offers tools that actually exist

  Scenario: research rejects the ast tool as unknown
    Given an empty project root tempdir
    When I dispatch research with tool="ast"
    Then the dispatcher returns an error
    And the error message contains "Research tool not found: ast"
    And the error message lists "perplexity, jira, confluence, stakeholder" as the available bundled tools
