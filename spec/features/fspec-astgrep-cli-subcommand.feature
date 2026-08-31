@querying
@done
@cli
@astgrep
@CLI-015
Feature: fspec astgrep CLI subcommand

  """
  The `fspec astgrep` clap subcommand gives one-shot CLI users the same AST
  code search the native agent gets from the AstGrep tool. The bridge
  (rust/fspec/src/astgrep.rs) is JSON marshalling + stdout/stderr rendering
  only — it delegates to codelet_tools::AstGrepTool::execute with a nil
  session id (no worktree isolation), so CLI and the harness tool share one
  implementation. Required --pattern/--lang, optional --path. Exit 0 with
  matches on stdout (file:line:column:text); exit 1 with the error on stderr.
  """

  Background: User Story
    As a shell user working with fspec
    I want a working `fspec astgrep` command for AST code search
    So that I can run ast-grep searches natively from the CLI

  Scenario: fspec astgrep runs an AST search and prints matches
    Given a temp project root containing a Rust source file with a top-level `fn main() { ... }`
    When I run `fspec astgrep --pattern "fn $NAME($$$ARGS) { $$$BODY }" --lang rust --path src/` in that directory
    Then the command exits with code 0
    And stdout contains a match line in `file:line:column:text` format for the source file

  Scenario: fspec astgrep requires pattern and lang
    Given an empty project root tempdir
    When I run `fspec astgrep` without a `--pattern` argument
    Then the command exits with code 1
    And stderr mentions the missing required argument `--pattern`
    And when I run `fspec astgrep --pattern "fn $NAME($$$ARGS) { $$$BODY }"` without a `--lang` argument
    Then the command exits with code 1
    And stderr mentions the missing required argument `--lang`

  Scenario: fspec astgrep reports an invalid pattern to stderr and exits 1
    Given a temp project root containing at least one Rust source file
    When I run `fspec astgrep --pattern "fn $NAME() { $$$BODY } fn $NAME() { $$$BODY }" --lang rust --path lib.rs`
    Then the command exits with code 1
    And stderr contains "Error: Invalid AST pattern"
