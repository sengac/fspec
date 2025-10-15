@done
@configuration
@cli
@workflow-automation
@hooks
@phase1
@HOOK-007
Feature: Hook management CLI commands

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a fspec user
  #   I want to manage hooks via CLI commands
  #   So that I can list, validate, add, and remove hooks without manually editing JSON
  #
  # BUSINESS RULES:
  #   1. list-hooks command displays all configured hooks grouped by event
  #   2. validate-hooks command checks config syntax and verifies hook scripts exist
  #   3. add-hook command adds a hook to config and creates the event array if needed
  #   4. remove-hook command removes a hook by name and event
  #   5. Commands fail gracefully if spec/fspec-hooks.json does not exist
  #   6. add-hook command requires: name, event, command (path to script)
  #
  # EXAMPLES:
  #   1. list-hooks displays: post-implementing: lint, test
  #   2. validate-hooks passes - all scripts exist, JSON valid
  #   3. validate-hooks fails - missing script spec/hooks/missing.sh
  #   4. add-hook --name lint --event post-implementing --command spec/hooks/lint.sh --blocking
  #   5. remove-hook --name lint --event post-implementing - hook removed from config
  #   6. list-hooks when no config file - displays friendly message
  #
  # ========================================
  Background: User Story
    As a fspec user
    I want to manage hooks via CLI commands
    So that I can list, validate, add, and remove hooks without manually editing JSON

  Scenario: List hooks displays all configured hooks
    Given I have a hook configuration file with hooks for "post-implementing"
    And the hooks are "lint" and "test"
    When I run "fspec list-hooks"
    Then the output should display hooks grouped by event
    And the output should show "post-implementing: lint, test"

  Scenario: Validate hooks passes when all scripts exist
    Given I have a hook configuration with valid JSON
    And all hook script files exist
    When I run "fspec validate-hooks"
    Then the command should exit with code 0
    And the output should indicate validation passed

  Scenario: Validate hooks fails when script is missing
    Given I have a hook configuration referencing "spec/hooks/missing.sh"
    And the file "spec/hooks/missing.sh" does not exist
    When I run "fspec validate-hooks"
    Then the command should exit with non-zero code
    And the output should contain "Hook command not found: spec/hooks/missing.sh"

  Scenario: Add hook to configuration
    Given I have a hook configuration file
    When I run "fspec add-hook --name lint --event post-implementing --command spec/hooks/lint.sh --blocking"
    Then a new hook named "lint" should be added to "post-implementing"
    And the hook should have blocking set to true
    And the config file should be updated

  Scenario: Remove hook from configuration
    Given I have a hook configuration with hook "lint" in "post-implementing"
    When I run "fspec remove-hook --name lint --event post-implementing"
    Then the hook "lint" should be removed from "post-implementing"
    And the config file should be updated

  Scenario: List hooks when no config file exists
    Given the file "spec/fspec-hooks.json" does not exist
    When I run "fspec list-hooks"
    Then the output should display a friendly message
    And the message should indicate no hooks are configured
