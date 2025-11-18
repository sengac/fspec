@research
@cli
@bug-fix
@BUG-083
Feature: AST research tool fails with relative file paths

  """
  AST research tool uses tree-sitter for code analysis. Path resolution should use Node.js path.resolve() to convert relative paths to absolute paths based on process.cwd(). Implementation is in src/research-tools/ast.ts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AST research tool must resolve relative file paths from current working directory
  #   2. Absolute paths must continue to work as before
  #
  # EXAMPLES:
  #   1. Run 'fspec research --tool=ast --operation=list-functions --file=src/index.ts' and it successfully analyzes the file
  #   2. Run 'fspec research --tool=ast --operation=list-functions --file=/Users/rquast/projects/fspec/src/index.ts' and it successfully analyzes the file
  #
  # ========================================

  Background: User Story
    As a developer using fspec research tools
    I want to use relative file paths with AST research tool
    So that I can analyze code without specifying full absolute paths

  Scenario: AST research tool resolves relative file paths
    Given I am in the project root directory
    And the file "src/index.ts" exists
    When I run "fspec research --tool=ast --operation=list-functions --file=src/index.ts"
    Then the command should succeed
    And the output should contain function names from the file
    And no "ENOENT" or "file not found" errors should occur

  Scenario: AST research tool works with absolute file paths
    Given I am in the project root directory
    And the file "src/index.ts" exists
    When I run "fspec research --tool=ast --operation=list-functions --file=/Users/rquast/projects/fspec/src/index.ts"
    Then the command should succeed
    And the output should contain function names from the file
    And the behavior should be identical to using relative paths
