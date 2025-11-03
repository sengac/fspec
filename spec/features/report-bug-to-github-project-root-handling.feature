@done
@high
@cli
@bug-fix
@report-bug
@BUG-058
Feature: report-bug-to-github crashes with 'path must be string' error
  """
  gatherContext function in reportBugToGitHub must receive valid string path to avoid 'path must be string' error when using join()
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. findProjectRoot() requires a cwd parameter of type string
  #   2. When no projectRoot option is provided, the command must use process.cwd() as the current working directory
  #   3. The gatherContext function must receive a valid string path for projectRoot
  #
  # EXAMPLES:
  #   1. User runs 'fspec report-bug-to-github' without --project-root flag, command crashes with 'path must be string. Received undefined'
  #   2. User runs 'fspec report-bug-to-github --bug-description "test"', command crashes before gathering context
  #   3. User runs 'fspec report-bug-to-github --project-root /path/to/project', command works correctly because projectRoot is provided
  #   4. After fix, user runs 'fspec report-bug-to-github' from any directory, command gathers context successfully using process.cwd()
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to run report-bug-to-github command without errors
    So that I can successfully report bugs to GitHub

  Scenario: Command crashes when run without --project-root flag
    Given I am in a project directory
    When I run "fspec report-bug-to-github" without --project-root flag
    Then the command should crash with error "The path argument must be of type string. Received undefined"
    And the error should occur in the gatherContext function

  Scenario: Command crashes with options but no --project-root
    Given I am in a project directory
    When I run "fspec report-bug-to-github --bug-description 'test'"
    Then the command should crash before gathering system context
    And the error should be "The path argument must be of type string. Received undefined"

  Scenario: Command works when --project-root is explicitly provided
    Given I am in a project directory
    When I run "fspec report-bug-to-github --project-root /path/to/project"
    Then the command should gather system context successfully
    And the command should not crash
    And the browser should open with pre-filled bug report

  Scenario: After fix - command works without --project-root using process.cwd()
    Given I am in a project directory
    And findProjectRoot() is called with process.cwd() parameter
    When I run "fspec report-bug-to-github"
    Then the command should gather system context successfully
    And the command should use process.cwd() to find project root
    And the browser should open with pre-filled bug report
