@research-tools
@help-system
@cli
@critical
@BUG-074
Feature: Fix 'fspec research --tool=ast --help' command - should display tool-specific help instead of failing
  """
  Uses formatResearchToolHelp() utility in help-formatter.ts for consistent formatting. ResearchTool interface requires getHelpConfig() method returning ResearchToolHelpConfig. Help flag check moved BEFORE getResearchTool() to fix BUG-074. Works for both bundled tools (src/research-tools/) and custom tools (spec/research-tools/).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Help display must work for both bundled tools (src/research-tools/) and custom tools (spec/research-tools/)
  #   2. Help output must follow standardized format with AI-optimized sections (WHEN TO USE, PREREQUISITES, COMMON ERRORS)
  #   3. All research tools must use structured ResearchToolHelpConfig instead of hand-crafted strings
  #   4. Help formatting logic must be centralized in help-formatter.ts to avoid duplication
  #   5. The help flag must be checked BEFORE tool validation to show help even if tool has loading issues
  #   6. Help formatter must support custom sections beyond standard ones for advanced documentation
  #   7. TypeScript compiler provides validation for ResearchToolHelpConfig - no runtime JSON Schema validation needed
  #   8. CONFIGURATION section is optional and shown only when tool provides it in config
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=ast --help' and sees structured help with USAGE, OPTIONS, EXAMPLES, WHEN TO USE sections
  #   2. User runs 'fspec research --tool=jira --help' and sees help with CONFIGURATION section showing required credentials
  #   3. User runs 'fspec research --tool=custom --help' where custom is in spec/research-tools/ and sees same formatted help as bundled tools
  #   4. User runs 'fspec research --tool=nonexistent --help' and sees error with list of available tools
  #   5. Developer refactors ast.ts help from string-based help() method to structured getHelpConfig() returning ResearchToolHelpConfig
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the help formatter support custom sections beyond the standard ones (USAGE, OPTIONS, EXAMPLES, etc.)?
  #   A: true
  #
  #   Q: Should we validate ResearchToolHelpConfig with JSON Schema to enforce required fields?
  #   A: true
  #
  #   Q: Should the CONFIGURATION section be optional or required for tools that need config files?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent or developer using fspec research tools
    I want to get help for a specific research tool using --help flag
    So that I can understand tool usage without trial-and-error or reading source code

  Scenario: Display help for bundled tool with standard sections
    Given the ast research tool is bundled in src/research-tools/
    When I run 'fspec research --tool=ast --help'
    Then the help output should contain a USAGE section
    And the help output should contain an OPTIONS section
    And the help output should contain an EXAMPLES section
    And the help output should contain a WHEN TO USE section

  Scenario: Display help for tool requiring configuration
    Given the jira research tool requires configuration in ~/.fspec/fspec-config.json
    When I run 'fspec research --tool=jira --help'
    Then the help output should contain a CONFIGURATION section
    And the CONFIGURATION section should show the required credentials

  Scenario: Display help for custom tool in spec/research-tools/
    Given a custom tool named 'custom' exists in spec/research-tools/custom.js
    When I run 'fspec research --tool=custom --help'
    Then the help output should be formatted identically to bundled tools
    And the custom tool implements getHelpConfig() method
    And the help output should contain all standard sections

  Scenario: Error when requesting help for nonexistent tool
    Given no research tool named 'nonexistent' exists
    When I run 'fspec research --tool=nonexistent --help'
    Then the command should display an error message
    And the error should list all available research tools
    And the command should exit with code 1

  Scenario: Refactor tool from string-based help to structured config
    Given the ast tool has a help() method returning a hand-crafted string
    When I refactor ast.ts to use getHelpConfig() returning ResearchToolHelpConfig
    Then the help output should match the previous format
    And the formatting should be handled by formatResearchToolHelp()
    And TypeScript should validate the config structure at compile time
