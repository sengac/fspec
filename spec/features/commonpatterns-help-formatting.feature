@done
@help-system
@bug-fix
@formatting
@cli
@phase1
@BUG-023
Feature: commonPatterns displays [object Object] in help output

  """
  Uses CommandHelpConfig interface in src/utils/help-formatter.ts. Type safety enforced via TypeScript. Formatter must handle union type: string[] | CommonPattern[] where CommonPattern = { pattern: string; example: string; description: string }. Backward compatibility required for existing help files using string[] format.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Help formatter must support both string[] and object[] formats for commonPatterns (backward compatibility)
  #   2. Object format commonPatterns must display pattern name, example command, and description
  #   3. Interface must accept both formats: string[] | Array<{pattern,example,description}>
  #
  # EXAMPLES:
  #   1. Running 'fspec add-virtual-hook --help' displays formatted patterns with name, example, and description instead of [object Object]
  #   2. All 20+ affected help commands display COMMON PATTERNS section correctly after fix
  #   3. Help commands still using string[] format continue to work (backward compatibility)
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec CLI
    I want to view properly formatted COMMON PATTERNS in help output
    So that I can understand usage patterns without seeing [object Object]

  Scenario: Display formatted patterns for object-style commonPatterns
    Given I have a help file using object-style commonPatterns with pattern, example, and description fields
    When I run "fspec add-virtual-hook --help"
    Then the COMMON PATTERNS section should display formatted pattern information
    And each pattern should show the pattern name
    And each pattern should show the example command
    And each pattern should show the description
    And the output should NOT contain "[object Object]"

  Scenario: All affected commands display COMMON PATTERNS correctly
    Given there are 20+ help files using object-style commonPatterns
    When I run any affected command with --help flag
    Then the COMMON PATTERNS section should display formatted content
    And no command should show "[object Object]" in help output

  Scenario: Backward compatibility with string array format
    Given I have a help file using string[] format for commonPatterns
    When I run the command with --help flag
    Then the COMMON PATTERNS section should display as before
    And each pattern should show as a bulleted string
    And the formatter should not break existing string[] usage
