@done
@research-integration
@research
@cli
@bug
@high
@BUG-075
Feature: AST research tool fails with '--file is required' error even when --file flag is provided

  """
  The AST tool uses indexOf() to find '--file' flag, which fails when argument is '--file=value' format. The fix parses both '--flag value' and '--flag=value' formats by checking if arg starts with '--file=' and extracting value after equals sign.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The research command must correctly forward all arguments after --tool to the tool's execute() function
  #   2. The tool's execute() function must correctly parse --file argument from the forwarded args array
  #   3. AST tool must handle both --flag=value and --flag value argument formats
  #   4. When --file is not found in args array, check if any element starts with '--file='
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=ast --operation=list-functions --file=src/git/diff.ts', AST tool should receive ['--operation=list-functions', '--file=src/git/diff.ts'] as args
  #   2. User runs 'fspec research --tool=ast --operation=list-functions --file=src/git/diff.ts' (equals format), tool should parse --file=src/git/diff.ts correctly
  #   3. User runs 'fspec research --tool=ast --operation list-functions --file src/git/diff.ts' (space format), tool should parse arguments correctly
  #
  # ========================================

  Background: User Story
    As a developer using fspec research tool
    I want to use either --flag=value or --flag value syntax for all arguments
    So that the tool works reliably regardless of which argument format I choose

  Scenario: Parse --file argument in equals format
    Given I have the AST research tool
    When I run 'fspec research --tool=ast --operation=list-functions --file=src/git/diff.ts'
    Then the tool should successfully parse the --file argument
    And the tool should return JSON with matches


  Scenario: Parse --file argument in space-separated format
    Given I have the AST research tool
    When I run 'fspec research --tool=ast --operation list-functions --file src/git/diff.ts'
    Then the tool should successfully parse the --file argument
    And the tool should return JSON with matches

