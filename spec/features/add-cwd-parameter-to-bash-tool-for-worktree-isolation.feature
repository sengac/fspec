@tools
@tool-execution
@codelet
@TOOL-013
Feature: Add cwd parameter to Bash tool for worktree isolation
  """
  Modifies rust/tools/src/bash.rs. Add optional cwd: Option<String> to BashArgs struct. Modify spawn_command() to call .current_dir(cwd) when provided. Validate directory exists before spawning command.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The cwd parameter is optional - if not provided, the tool behaves as before (inherits parent working directory)
  #   2. When cwd is provided, use std::process::Command::current_dir() to set the child process working directory
  #   3. If cwd path does not exist, return an error before executing the command
  #
  # EXAMPLES:
  #   1. Agent calls Bash with cwd='/path/to/worktree' and command='pwd' → output shows '/path/to/worktree'
  #   2. Agent calls Bash without cwd parameter → command runs in fspec's working directory (backward compatible)
  #   3. Agent calls Bash with cwd='/nonexistent/path' → returns error 'Directory not found: /nonexistent/path'
  #
  # ========================================
  Background: User Story
    As a AI agent working in an isolated git worktree
    I want to run Bash commands in my worktree without prefixing cd
    So that commands execute in the correct directory automatically

  Scenario: Execute command in specified working directory
    Given I have a valid directory path "/tmp/test-worktree"
    When I call the Bash tool with cwd="/tmp/test-worktree" and command="pwd"
    Then the output should show "/tmp/test-worktree"

  Scenario: Execute command without cwd parameter (backward compatible)
    Given I do not specify a cwd parameter
    When I call the Bash tool with command="pwd"
    Then the command should run in fspec's working directory
    And the behavior should match the previous implementation

  Scenario: Error when cwd directory does not exist
    Given I specify a non-existent directory "/nonexistent/path/to/dir"
    When I call the Bash tool with cwd="/nonexistent/path/to/dir" and command="pwd"
    Then the tool should return an error
    And the error message should contain "Directory not found: /nonexistent/path/to/dir"
    And the command should not be executed
