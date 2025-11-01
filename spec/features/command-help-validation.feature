@CLI-013
@critical @cli @validation @development-tools
Feature: Command Help File Validation Script

  """
  Architecture notes:
  - Standalone JavaScript validation script (NOT integrated into fspec CLI)
  - Validates that every command has corresponding help file in src/commands/*-help.ts
  - Checks help file format consistency and completeness
  - Returns exit code 0 for success, non-zero for failures
  - Can run manually during development or automatically in CI/CD

  Critical implementation requirements:
  - MUST check every registered command has corresponding help file
  - MUST detect orphaned help files (help file with no command)
  - MUST validate help files export correct function signature
  - MUST validate help content is non-empty and properly formatted
  - MUST validate consistent formatting across all help files
  - Error output MUST include file paths and specific issues

  References:
  - Commander.js command registration
  - Help system in src/help.ts
  """

  Background: User Story
    As a fspec developer
    I want automated validation of command-help file linkage
    So that I catch missing or inconsistent help documentation before users encounter errors

  @smoke @critical
  Scenario: Detect missing help file for registered command
    Given a command "add-diagram" is registered in the CLI
    And no file "src/commands/add-diagram-help.ts" exists
    When I run the validation script
    Then the script should exit with non-zero code
    And the output should contain "Missing help file for command: add-diagram"
    And the output should include the expected file path "src/commands/add-diagram-help.ts"

  @regression
  Scenario: Detect orphaned help file with no command
    Given a file "src/commands/delete-foo-help.ts" exists
    And no command "delete-foo" is registered in the CLI
    When I run the validation script
    Then the script should exit with non-zero code
    And the output should contain "Orphaned help file: delete-foo-help.ts"
    And the output should suggest removing the file or registering the command

  @smoke @critical
  Scenario: Validate all commands and help files are properly linked
    Given all registered commands have corresponding help files
    And all help files have corresponding registered commands
    And all help files export the correct function signature
    When I run the validation script
    Then the script should exit with code 0
    And the output should contain "All commands validated successfully"
    And the output should display the total number of commands checked

  @validation @format-check
  Scenario: Detect help file with empty content
    Given a command "validate" is registered
    And the file "src/commands/validate-help.ts" exists but is empty
    When I run the validation script
    Then the script should exit with non-zero code
    And the output should contain "Help content is empty: validate-help.ts"

  @validation @format-check
  Scenario: Detect help file with inconsistent formatting
    Given multiple help files exist with different formatting structures
    And the file "src/commands/add-rule-help.ts" has an empty USAGE section
    When I run the validation script
    Then the script should exit with non-zero code
    And the output should contain "Inconsistent help format in add-rule-help.ts: USAGE section is empty"

  @validation @function-signature
  Scenario: Detect help file with incorrect export signature
    Given a file "src/commands/foo-help.ts" exists
    And the file does not export a function with correct signature
    When I run the validation script
    Then the script should exit with non-zero code
    And the output should contain "Invalid help file export: foo-help.ts"
    And the output should describe the expected function signature
