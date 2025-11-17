@documentation
@event-storm
@help-system
@cli
@high
@EXMAP-015
Feature: Wire up comprehensive help for all Event Storm commands

  """
  Help files must follow the pattern established by existing commands (e.g., generate-scenarios-help.ts, add-scenario-help.ts). Each help file exports a function that returns formatted help text with sections for description, usage, AI-optimized guidance, examples, and related commands. The help content is integrated into src/help.ts under the appropriate group function (displayDiscoveryHelp for Event Storm commands).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All 7 Event Storm commands must have dedicated *-help.ts files following existing help file patterns
  #   2. Help files must include AI-optimized sections: WHEN TO USE, PREREQUISITES, TYPICAL WORKFLOW, COMMON ERRORS, COMMON PATTERNS
  #   3. Event Storm commands must be added to src/help.ts under the discovery group section
  #   4. Help content must cover all 7 Event Storm commands: add-command, add-domain-event, add-hotspot, add-policy, generate-example-mapping-from-event-storm, show-event-storm, show-foundation-event-storm
  #
  # EXAMPLES:
  #   1. Running 'fspec add-domain-event --help' shows comprehensive usage with examples and AI-optimized sections
  #   2. Running 'fspec help discovery' displays Event Storm commands section with add-domain-event, add-command, add-policy, add-hotspot, show-event-storm, show-foundation-event-storm, generate-example-mapping-from-event-storm
  #   3. File src/commands/generate-example-mapping-from-event-storm-help.ts exists and exports getGenerateExampleMappingFromEventStormHelp function
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec
    I want to access comprehensive help for Event Storm commands
    So that I can understand how to use Event Storm features without reading source code

  Scenario: View comprehensive help for individual Event Storm command
    Given I am using fspec in a project
    When I run "fspec add-domain-event --help"
    Then the output should contain comprehensive usage information
    And the output should include AI-optimized sections
    And the output should include "WHEN TO USE"
    And the output should include "PREREQUISITES"
    And the output should include "TYPICAL WORKFLOW"
    And the output should include "COMMON ERRORS"
    And the output should include "COMMON PATTERNS"
    And the output should include examples for the command

  Scenario: View Event Storm commands in discovery help group
    Given I am using fspec in a project
    When I run "fspec help discovery"
    Then the output should contain an Event Storm section
    And the section should list "add-domain-event"
    And the section should list "add-command"
    And the section should list "add-policy"
    And the section should list "add-hotspot"
    And the section should list "show-event-storm"
    And the section should list "show-foundation-event-storm"
    And the section should list "generate-example-mapping-from-event-storm"

  Scenario: Verify help file exists for generate-example-mapping-from-event-storm
    Given the fspec codebase
    When I check for the file "src/commands/generate-example-mapping-from-event-storm-help.ts"
    Then the file should exist
    And the file should export a help function
    And the function should follow the standard help file pattern

  Scenario: Verify all 7 Event Storm commands have help files
    Given the fspec codebase
    When I check for help files for all Event Storm commands
    Then "src/commands/add-command-help.ts" should exist
    And "src/commands/add-domain-event-help.ts" should exist
    And "src/commands/add-hotspot-help.ts" should exist
    And "src/commands/add-policy-help.ts" should exist
    And "src/commands/generate-example-mapping-from-event-storm-help.ts" should exist
    And "src/commands/show-event-storm-help.ts" should exist
    And "src/commands/show-foundation-event-storm-help.ts" should exist
