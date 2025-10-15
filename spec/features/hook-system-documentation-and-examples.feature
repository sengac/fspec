@done
@scaffolding
@documentation
@hooks
@phase1
@HOOK-009
Feature: Hook system documentation and examples

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a fspec user writing hook scripts
  #   I want to have comprehensive documentation and working examples
  #   So that I can quickly create custom hooks without trial and error
  #
  # BUSINESS RULES:
  #   1. Documentation explains hook configuration JSON schema with all fields
  #   2. Example hooks provided for bash, python, and node.js
  #   3. Documentation explains hook context JSON passed via stdin
  #   4. Best practices documented for stdout/stderr usage and exit codes
  #   5. Common use case examples: linting, testing, notifications, validation
  #   6. Documentation includes troubleshooting section for common errors
  #
  # EXAMPLES:
  #   1. Configuration doc shows complete fspec-hooks.json with global defaults and multiple hooks
  #   2. Bash hook reads JSON context from stdin and validates work unit has feature file
  #   3. Python hook parses JSON stdin and runs pytest on work unit tests
  #   4. Node.js hook reads stdin and sends Slack notification about status change
  #   5. Lint hook example shows proper exit codes: 0 for success, 1 for lint errors
  #   6. Troubleshooting doc explains 'Hook command not found' error and solution
  #
  # ========================================
  Background: User Story
    As a fspec user writing hook scripts
    I want to have comprehensive documentation and working examples
    So that I can quickly create custom hooks without trial and error

  Scenario: Configuration documentation shows complete JSON schema
    Given I am reading the hook system documentation
    When I look at the configuration section
    Then I should see a complete fspec-hooks.json example
    And the example should include global defaults section
    And the example should include multiple hook definitions
    And the example should document all configuration fields
    And the example should show hooks object with event names as keys
    And the example should show hook properties: name, command, blocking, timeout, condition

  Scenario: Bash hook example reads context and validates feature file
    Given I am looking at bash hook examples
    When I read the validation hook example
    Then I should see how to read JSON from stdin
    And I should see how to parse workUnitId from context
    And I should see how to check if feature file exists
    And I should see proper exit code 0 for success
    And I should see proper exit code 1 for validation failure
    And I should see stderr output for error messages

  Scenario: Python hook example parses stdin and runs tests
    Given I am looking at python hook examples
    When I read the test runner hook example
    Then I should see how to import json and sys modules
    And I should see how to read stdin with sys.stdin.read()
    And I should see how to parse JSON context
    And I should see how to run pytest subprocess
    And I should see how to capture test output
    And I should see proper exit code handling from pytest

  Scenario: Node.js hook example reads stdin and sends notifications
    Given I am looking at node.js hook examples
    When I read the notification hook example
    Then I should see how to read stdin asynchronously
    And I should see how to parse JSON with JSON.parse()
    And I should see how to extract workUnitId and event from context
    And I should see how to send HTTP request to Slack webhook
    And I should see proper error handling for network failures
    And I should see process.exit() with appropriate codes

  Scenario: Lint hook example demonstrates proper exit codes
    Given I am looking at common use case examples
    When I read the linting hook example
    Then I should see exit code 0 for clean lint results
    And I should see exit code 1 for lint errors found
    And I should see lint errors written to stderr
    And I should see summary written to stdout
    And the example should show blocking: true for lint enforcement

  Scenario: Troubleshooting section explains common errors
    Given I am reading the troubleshooting documentation
    When I look for "Hook command not found" error
    Then I should see explanation of the error cause
    And I should see solution: check file path is relative to project root
    And I should see solution: verify file has execute permissions
    And I should see example of correct vs incorrect file paths
    And I should see how to test hook script manually
