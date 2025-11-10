@feature-management
@done
@research-tools
@cli
@high
@RES-011
Feature: Tool-Specific Help System

  """
  Tool scripts located in spec/research-scripts/ directory with executable permissions
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When --help flag is used with --tool specified, forward --help to the tool script
  #   2. When --help flag is used without --tool, show generic research help
  #   3. When tool does not exist, show error with available tools list
  #   4. All research tool scripts must implement --help flag with standardized format
  #   5. Tool help must include: usage, options, examples, configuration, exit codes
  #
  # EXAMPLES:
  #   1. Run 'fspec research --help' → shows generic research help with list of available tools
  #   2. Run 'fspec research --tool=perplexity --help' → shows Perplexity-specific help (query, model, format options)
  #   3. Run 'fspec research --tool=nonexistent --help' → error: tool not found, shows available tools
  #   4. Tool script without --help implementation → warning message with fallback to file location
  #
  # ========================================

  Background: User Story
    As a AI agent or developer using fspec research tools
    I want to get tool-specific help documentation when running --help flag
    So that I can learn tool options without reading source code or generic help

  Scenario: Show generic research help when no tool specified
    Given I am using fspec research command
    When I run 'fspec research --help'
    Then I should see generic research help documentation
    And the output should contain a list of available research tools


  Scenario: Forward help to tool script when tool specified
    Given the research tool 'perplexity' exists and implements --help
    When I run 'fspec research --tool=perplexity --help'
    Then I should see Perplexity-specific help documentation
    And the output should contain tool-specific options like --query and --model
    And the output should NOT contain generic research help


  Scenario: Show error when tool does not exist
    Given I am using fspec research command
    When I run 'fspec research --tool=nonexistent --help'
    Then I should see an error message indicating the tool was not found
    And the output should contain a list of available tools
    And the command should exit with code 1


  Scenario: Show warning when tool lacks help implementation
    Given a research tool exists but does not implement --help flag
    When I run 'fspec research --tool=legacy-tool --help'
    Then I should see a warning that the tool does not implement --help
    And the output should show generic usage instructions for the tool
    And the output should indicate where to find the tool script

