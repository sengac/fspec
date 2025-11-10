@done
@bug
@research-tools
@cli
@critical
@RES-016
Feature: AST tool returns stub instead of actual parsing

  """
  Architecture notes:
  - AST parser exists in src/utils/ast-parser.ts using tree-sitter library
  - src/research-tools/ast.ts needs to import and use parseFile() instead of returning stub
  - tree-sitter parsers already installed: JavaScript, TypeScript, Python, Go
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AST parser implementation EXISTS in src/utils/ast-parser.ts with full tree-sitter support (JavaScript, TypeScript, Python, Go)
  #   2. src/research-tools/ast.ts returns stub instead of calling parseFile() from ast-parser.ts
  #   3. Fix: Import parseFile from ../utils/ast-parser and call it instead of returning stub JSON
  #
  # EXAMPLES:
  #   1. BEFORE: fspec research --tool=ast --query 'find async functions' → Returns stub JSON
  #   2. AFTER: fspec research --tool=ast --query 'find async functions' → Returns actual AST matches using tree-sitter
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec
    I want to use AST research tool to analyze code
    So that I can find patterns and analyze code structure during Example Mapping

  Scenario: AST tool executes actual tree-sitter parsing instead of returning stub
    Given the AST parser implementation exists in src/utils/ast-parser.ts
    And tree-sitter is installed with JavaScript, TypeScript, Python, and Go parsers
    When I run "fspec research --tool=ast --query 'find async functions'"
    Then the tool should call parseFile() from ast-parser.ts
    And the tool should return actual AST matches using tree-sitter
    And the output should NOT contain "stub-implementation"
    And the output should contain function matches from the codebase
