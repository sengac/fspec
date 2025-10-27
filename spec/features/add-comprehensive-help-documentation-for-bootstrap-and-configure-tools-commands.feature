@done
@help-system
@documentation
@cli
@HELP-004
Feature: Add comprehensive help documentation for bootstrap and configure-tools commands

  """
  Architecture notes:
  - Help files follow CommandHelpConfig pattern from src/utils/help-formatter
  - Bootstrap command aggregates help from getSpecsHelpContent, getWorkHelpContent, etc.
  - Help content must be consistent across CLI help, README.md, and docs directory
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each command must have a comprehensive help file following the CommandHelpConfig pattern
  #   2. Help files must include whenToUse, prerequisites, typicalWorkflow, commonErrors, and relatedCommands sections
  #   3. Bootstrap command output must be auto-generated from help content functions
  #   4. README.md must document both commands with usage examples and options
  #   5. Documentation in docs directory must be synchronized with help content
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec bootstrap --help' and sees comprehensive help with whenToUse, prerequisites, typicalWorkflow sections
  #   2. Developer runs 'fspec configure-tools --help' and sees comprehensive help with platform-agnostic examples
  #   3. AI agent runs 'fspec bootstrap' and receives complete workflow documentation with all command help sections
  #   4. Developer searches README.md for configure-tools and finds complete documentation with all options and examples
  #   5. Documentation in docs/commands/bootstrap.md matches help content with no inconsistencies
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should bootstrap command help be added to src/help.ts main help output, or only accessible via --help flag?
  #   A: true
  #
  #   Q: Do we need separate documentation files in docs/ directory, or should we rely on generated help from the help config files?
  #   A: true
  #
  #   Q: Should configure-tools help include specific examples for Node.js, Python, Rust, and Go, or keep it generic?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer or AI agent using fspec
    I want to access comprehensive help documentation for bootstrap and configure-tools commands
    So that I understand how to use these commands effectively without consulting source code

  Scenario: Display comprehensive help for bootstrap command
    Given the bootstrap command exists
    When I run "fspec bootstrap --help"
    Then I should see the command description
    And I should see the "WHEN TO USE" section
    And I should see the "PREREQUISITES" section
    And I should see the "TYPICAL WORKFLOW" section
    And I should see usage examples

  Scenario: Display comprehensive help for configure-tools command
    Given the configure-tools command exists
    When I run "fspec configure-tools --help"
    Then I should see the command description
    And I should see platform-agnostic examples
    And I should see examples for Node.js with npm test
    And I should see examples for Python with pytest
    And I should see examples for Rust with cargo test
    And I should see examples for Go with go test

  Scenario: Bootstrap command outputs complete workflow documentation
    Given the bootstrap help content functions exist
    When I run "fspec bootstrap"
    Then I should see all command help sections
    And the output should include specs help content
    And the output should include work help content
    And the output should include discovery help content
    And the output should include metrics help content
    And the output should include setup help content
    And the output should include hooks help content

  Scenario: README.md documents configure-tools command
    Given the README.md file exists
    When I search for "configure-tools" in README.md
    Then I should find complete documentation
    And I should see all command options listed
    And I should see usage examples with test-command option
    And I should see usage examples with quality-commands option

  Scenario: Documentation consistency across all sources
    Given bootstrap-help.ts exists
    And configure-tools-help.ts exists
    When I compare help content with docs directory
    Then bootstrap documentation should be consistent
    And configure-tools documentation should be consistent
    And README.md should match help content
    And no inconsistencies should exist
