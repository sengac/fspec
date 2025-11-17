@done
@tree-sitter
@ast
@research-tools
@cli
@high
@RES-014
Feature: Language-Agnostic AST Tool for AI Analysis
  """
  Uses tree-sitter for language-agnostic AST parsing. Integrates with research command system (RES-010) via --tool=ast flag. Supports JavaScript, TypeScript, Python, Go, Rust, and Java grammars. Returns structured JSON output with file paths, line numbers, and code snippets for AI agent consumption. Supports queries for function definitions, class definitions, imports, and exports.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AST tool must use tree-sitter for language-agnostic parsing
  #   2. Tool must support JavaScript, TypeScript, Python, Go, Rust, and Java
  #   3. AST queries must support finding function definitions, class definitions, imports, and exports
  #   4. Results must include file path, line number, and code snippet for each match
  #   5. Tool must integrate with research command system (RES-010) using --tool=ast flag
  #   6. AST results must be formatted for AI agent consumption with structured JSON output
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=ast --query="function definitions"' and sees list of all functions in codebase with file paths and line numbers
  #   2. User queries 'class AuthService' and AST tool finds AuthService class definition in src/auth/service.ts:42-150
  #   3. User searches for 'import statements' across TypeScript files and gets JSON output with import paths, line numbers, and imported symbols
  #   4. AST tool parses Python file and finds all class definitions with their methods using tree-sitter Python grammar
  #   5. User queries 'export default' pattern and AST finds all default exports across JavaScript and TypeScript files
  #
  # ========================================
  Background: User Story
    As a developer using research tools
    I want to analyze code patterns across multiple programming languages
    So that I can understand codebase structure without language-specific tools

  Scenario: Find all function definitions across codebase
    Given I have a codebase with multiple programming languages
    When I run "fspec research --tool=ast --query=\"function definitions\""
    Then I should see a list of all functions in the codebase
    And each result should include file path and line number

  Scenario: Find specific class definition by name
    Given I have a TypeScript file with class AuthService
    When I query for "class AuthService"
    Then the AST tool should find AuthService class definition
    And the result should include file path src/auth/service.ts
    And the result should include line range 42-150

  Scenario: Search for import statements with JSON output
    Given I have TypeScript files with import statements
    When I search for "import statements" across TypeScript files
    Then I should receive JSON output with import paths
    And the JSON should include line numbers for each import
    And the JSON should include imported symbols

  Scenario: Parse Python file for class definitions and methods
    Given I have a Python file with class definitions
    When the AST tool parses the Python file
    Then it should find all class definitions
    And it should find all methods within each class
    And it should use tree-sitter Python grammar

  Scenario: Find export default patterns across JavaScript and TypeScript
    Given I have JavaScript and TypeScript files with export default statements
    When I query for "export default" pattern
    Then the AST tool should find all default exports
    And the results should include both JavaScript and TypeScript files
