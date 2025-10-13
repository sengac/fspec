@phase1
@cli
@help-system
@documentation
Feature: Scalable help system for all commands
  """
  Architecture notes:
  - Process-level help interceptor runs BEFORE Commander.js parsing
  - Help configs stored in src/commands/*-help.ts files
  - Dynamic imports used for Vite compatibility (no static requires)
  - Registry Set tracks which commands have custom help
  - Graceful fallback to Commander.js default help for missing configs
  - Zero modifications to 91+ command registrations in index.ts
  - Uses existing CommandHelpConfig interface and help-formatter.ts
  - Preserves all custom formatting (WHEN TO USE, PREREQUISITES, etc.)

  Critical implementation requirements:
  - MUST intercept --help before Commander.js processes arguments
  - MUST use dynamic imports (await import()) for help configs
  - MUST maintain separate help-registry.ts with Set of command names
  - MUST NOT modify existing command registrations
  - MUST fall back gracefully when help config doesn't exist
  - Help formatter MUST preserve existing formatting standards
  - MUST work with any flag position (--help before/after args)
  """

  Background: User Story
    As an AI agent using fspec
    I want comprehensive --help support for all commands
    So that I can understand command usage without reading source code

  @CLI-006
  Scenario: Display custom help when help config exists
    Given I have a command "remove-tag-from-scenario" with help config at "src/commands/remove-tag-from-scenario-help.ts"
    And the command is registered in help-registry.ts
    When I run "fspec remove-tag-from-scenario --help"
    Then the process-level help interceptor should load the help config
    And I should see detailed custom help with WHEN TO USE section
    And I should see examples with expected output
    And I should see related commands
    And the process should exit with code 0

  @CLI-006
  Scenario: Fall back to Commander.js help when no custom help exists
    Given I have a command "some-command" without a help config file
    And the command is NOT registered in help-registry.ts
    When I run "fspec some-command --help"
    Then the process-level help interceptor should return false
    And Commander.js should display default help
    And the process should exit with code 0

  @CLI-006
  Scenario: Handle help flag in any position
    Given I have a command "list-work-units" with custom help
    When I run "fspec --help list-work-units"
    And I run "fspec list-work-units --help"
    And I run "fspec list-work-units --status=backlog --help"
    Then custom help should be displayed in all cases
    And the process should exit with code 0

  @CLI-006
  Scenario: Handle main program help without command
    Given I run "fspec --help" without specifying a command
    Then the help interceptor should return false
    And Commander.js should display main program help
    And the help should list all available commands

  @CLI-006
  Scenario: Gracefully handle missing help config file
    Given I have a command "broken-command" registered in help-registry.ts
    But the help config file "src/commands/broken-command-help.ts" does not exist
    When I run "fspec broken-command --help"
    Then the dynamic import should fail gracefully
    And the system should fall back to Commander.js default help
    And no error should be thrown to the user
