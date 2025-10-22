@COV-026
@COV-025
@COV-024
@COV-023
@COV-022
@COV-021
@COV-020
@cli
@command-registration
@critical
@project-management
Feature: Complete CLI Command Registration
  """
  Architecture notes:
  - ALL command functions in src/commands/*.ts MUST be registered in src/index.ts
  - Each command MUST have corresponding CLI registration with proper options/arguments
  - Commands are organized by category: spec, tags, foundation, query, project
  - Help text MUST include all registered commands
  - Missing commands prevent users from accessing implemented functionality

  Critical implementation requirements:
  - Register ALL 46 missing commands in src/index.ts
  - Add proper Commander.js command() registration for each
  - Update help text for each command category
  - Ensure consistent argument/option naming across commands
  - Test that all commands are accessible via CLI

  Categories requiring command registration:
  - Work Unit Management (prioritize, update, delete, repair, validate, export, import)
  - Example Mapping (add-example, add-question, add-rule, remove, answer, import, export, generate)
  - Dependencies (add-dependency, add-dependencies, remove, clear, export, query)
  - Metrics & Estimation (record-metric, record-tokens, record-iteration, query commands)
  - Workflow & Automation (auto-advance, board, workflow-automation)
  - Prefix & Epic Management (create-prefix, update-prefix, delete-epic)
  - Validation (validate-spec-alignment, validate-work-units)
  - Query & Reporting (query, generate-summary-report)
  """

  Background: User Story
    As a developer using fspec for project management and specification
    I want ALL implemented functionality to be accessible via CLI commands
    So that I can use the complete feature set without writing custom code

  @work-unit-management
  Scenario: Register prioritize-work-unit command
    Given the function "prioritizeWorkUnit" exists in "src/commands/prioritize-work-unit.ts"
    When I check if "fspec prioritize-work-unit" is registered in CLI
    Then the command should be registered in src/index.ts
    And the help text should include "fspec prioritize-work-unit"
    And the command should support options: --position, --before, --after
    When I run "fspec prioritize-work-unit AUTH-001 --position=top"
    Then the work unit AUTH-001 should be moved to top of backlog

  @work-unit-management
  Scenario: Register update-work-unit command
    Given the function "updateWorkUnit" exists in "src/commands/update-work-unit.ts"
    When I check if "fspec update-work-unit" is registered in CLI
    Then the command should be registered in src/index.ts
    And the help text should include "fspec update-work-unit"
    And the command should support options: --title, --description, --epic, --parent
    When I run "fspec update-work-unit AUTH-001 --title='New Title'"
    Then the work unit AUTH-001 title should be updated

  @work-unit-management
  Scenario: Register delete-work-unit command
    Given the function "deleteWorkUnit" exists in "src/commands/delete-work-unit.ts"
    When I check if "fspec delete-work-unit" is registered in CLI
    Then the command should be registered in src/index.ts
    And the help text should include "fspec delete-work-unit"
    When I run "fspec delete-work-unit AUTH-001"
    Then the work unit AUTH-001 should be deleted

  @work-unit-management
  Scenario: Register update-work-unit-status command
    Given the function "updateWorkUnitStatus" exists in "src/commands/update-work-unit-status.ts"
    When I check if "fspec update-work-unit-status" is registered in CLI
    Then the command should be registered in src/index.ts
    And the help text should include "fspec update-work-unit-status"
    When I run "fspec update-work-unit-status AUTH-001 implementing"
    Then the work unit AUTH-001 status should be "implementing"

  @example-mapping
  Scenario: Register example mapping commands
    Given example mapping functions exist in src/commands/
    When I check if example mapping commands are registered
    Then "fspec add-example" should be registered
    And "fspec add-question" should be registered
    And "fspec add-rule" should be registered
    And "fspec remove-example" should be registered
    And "fspec remove-question" should be registered
    And "fspec remove-rule" should be registered
    And "fspec answer-question" should be registered
    And "fspec import-example-map" should be registered
    And "fspec export-example-map" should be registered
    And "fspec generate-scenarios" should be registered

  @dependency-management
  Scenario: Register dependency management commands
    Given dependency functions exist in src/commands/
    When I check if dependency commands are registered
    Then "fspec add-dependency" should be registered
    And "fspec add-dependencies" should be registered
    And "fspec remove-dependency" should be registered
    And "fspec clear-dependencies" should be registered
    And "fspec dependencies" should be registered
    And "fspec export-dependencies" should be registered

  @metrics-estimation
  Scenario: Register metrics and estimation commands
    Given metrics functions exist in src/commands/
    When I check if metrics commands are registered
    Then "fspec record-metric" should be registered
    And "fspec record-tokens" should be registered
    And "fspec record-iteration" should be registered
    And "fspec update-work-unit-estimate" should be registered
    And "fspec query-metrics" should be registered
    And "fspec query-estimate-accuracy" should be registered
    And "fspec query-estimation-guide" should be registered

  @workflow-automation
  Scenario: Register workflow and automation commands
    Given workflow functions exist in src/commands/
    When I check if workflow commands are registered
    Then "fspec auto-advance" should be registered
    And "fspec board" should be registered
    And "fspec workflow-automation" should be registered

  @prefix-epic
  Scenario: Register prefix and epic management commands
    Given prefix and epic functions exist in src/commands/
    When I check if prefix and epic commands are registered
    Then "fspec create-prefix" should be registered
    And "fspec update-prefix" should be registered
    And "fspec delete-epic" should be registered

  @validation
  Scenario: Register validation commands
    Given validation functions exist in src/commands/
    When I check if validation commands are registered
    Then "fspec validate-spec-alignment" should be registered
    And "fspec validate-work-units" should be registered
    And "fspec repair-work-units" should be registered

  @query-reporting
  Scenario: Register query and reporting commands
    Given query functions exist in src/commands/
    When I check if query commands are registered
    Then "fspec query" should be registered
    And "fspec query-work-units" should be registered
    And "fspec query-dependency-stats" should be registered
    And "fspec query-example-mapping-stats" should be registered
    And "fspec generate-summary-report" should be registered
    And "fspec export-work-units" should be registered

  @assumption-management
  Scenario: Register assumption management command
    Given the function "addAssumption" exists in "src/commands/add-assumption.ts"
    When I check if "fspec add-assumption" is registered in CLI
    Then the command should be registered in src/index.ts
    And the help text should include "fspec add-assumption"
    When I run "fspec add-assumption feature-name 'assumption text'"
    Then the assumption should be added to the feature file

  @help-text
  Scenario: Update help text for all command categories
    Given all missing commands are registered in src/index.ts
    When I run "fspec help project"
    Then the help text should include all work unit commands
    And the help text should include prioritize-work-unit
    And the help text should include update-work-unit
    And the help text should include delete-work-unit
    And the help text should include all dependency commands
    And the help text should include all example mapping commands
    And the help text should include all metrics commands

  @integration
  Scenario: Verify all 46 missing commands are registered
    Given I have a list of 46 missing commands
    When I check src/index.ts for command registrations
    Then all 46 commands should be registered
    And each command should have proper argument/option definitions
    And each command should import the correct command function
    And the help system should document all commands

  Scenario: All commands registered and accessible
    Given I have the fspec CLI built and ready
    And all command functions exist in src/commands/
    When I check the CLI registration in src/index.ts
    Then all 84 command functions should be registered
    And each command should have proper Commander.js registration
    And each command should be accessible via "fspec <command-name>"
    And no implemented functionality should be missing from the CLI

  Scenario: Help system shows all commands
    Given I have the fspec CLI built and ready
    And all commands are registered in src/index.ts
    When I run "fspec --help"
    Then the help output should list all available commands
    And the help should be organized by category
    And each command should have a clear description
    And the help should include commands for: spec, tags, foundation, query, project management
    And no registered command should be missing from help output

  Scenario: Build succeeds with no errors
    Given I have made changes to command registration in src/index.ts
    And all command imports are correct
    And all TypeScript types are properly defined
    When I run "npm run build"
    Then the build should succeed with exit code 0
    And no TypeScript compilation errors should occur
    And the dist/index.js file should be created
    And all command registrations should be included in the bundle
