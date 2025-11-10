@querying
@discovery
@high
@research
@cli
@ast
@integration
@RES-007
Feature: AST code researcher for pattern detection and deep analysis

  """
  Uses tree-sitter Node.js bindings (node-tree-sitter or web-tree-sitter) for AST parsing across 40+ languages
  Implements as Node.js script (#!/usr/bin/env node) in spec/research-scripts/ast with executable permissions for auto-discovery
  Uses tree-sitter S-expression queries for pattern matching (similar to AST-grep but more powerful)
  Supports --query, --file, --format (json/markdown/text), --language flags for CLI interface
  Returns JSON schema: {matches: [{file, startLine, endLine, code, nodeType, metadata}], stats: {filesScanned, matchCount}}
  Includes FSPEC_TEST_MODE=1 support for testing without actual AST parsing (returns mock data)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Use tree-sitter as the AST parsing foundation (supports 40+ languages including TypeScript, JavaScript, Python, Go, Rust, Java, C++)
  #   2. Use tree-sitter query language for pattern matching (find specific code constructs across syntax trees)
  #   3. Support error-tolerant parsing (analyze incomplete or broken code without failing)
  #   4. Integrate as 'ast' research backend in 'fspec research' command (auto-discovered from spec/research-scripts/)
  #   5. Support common pattern queries: function definitions, class declarations, imports, exports, variable assignments, conditionals, loops
  #   6. Analyze code complexity metrics: cyclomatic complexity, nesting depth, function length, parameter count
  #   7. Find structurally similar code using AST pattern matching (not text-based regex)
  #   8. Return structured JSON results with file paths, line numbers, matched code snippets, and AST node types
  #
  # EXAMPLES:
  #   1. AI agent runs 'fspec research --tool=ast --query="find all async functions"', gets JSON results with file paths and line numbers for every async function in the codebase
  #   2. Developer researches 'functions with more than 5 parameters', AST tool analyzes complexity and returns list of functions exceeding parameter count threshold
  #   3. During Example Mapping, AI needs to find similar authentication code, queries 'find code patterns similar to src/auth/login.ts', gets structurally similar functions across Python, TypeScript, and Go files
  #   4. AI agent analyzes incomplete code file with syntax errors, tree-sitter parses what it can and returns partial AST for the valid portions
  #   5. Developer queries 'classes implementing interface UserRepository', AST tool uses tree-sitter queries to find all implementing classes with file locations and code snippets
  #   6. AI searches for functions with high cyclomatic complexity (>10) to identify refactoring candidates, gets JSON with file paths and complexity scores
  #   7. Developer queries for all exported functions to generate API documentation, AST tool extracts function signatures and JSDoc comments
  #   8. AI agent looks for unused imports across TypeScript files, tree-sitter parses import statements and checks for usage in AST
  #
  # ========================================

  Background: User Story
    As a AI agent or developer using fspec
    I want to analyze code structure using AST parsing during Example Mapping
    So that I can discover patterns, find similar code, and get deep insights without manual code review

  Scenario: Find all async functions in codebase
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And the codebase contains TypeScript files with async functions
    When I run "fspec research --tool=ast --query='find all async functions'"
    Then the ast script should execute with the query
    And the output should be in JSON format
    And the JSON should contain an array of matches
    And each match should include file path, line numbers, and code snippet
    And each match should have nodeType "async_function"

  Scenario: Analyze functions exceeding parameter count threshold
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And the codebase contains functions with varying parameter counts
    When I run "fspec research --tool=ast --query='functions with more than 5 parameters'"
    Then the ast script should analyze function complexity
    And the output should list functions exceeding the threshold
    And each result should show file path, function name, and parameter count
    And functions with 6 or more parameters should be included

  Scenario: Find structurally similar code across multiple languages
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And the codebase contains authentication code in TypeScript, Python, and Go
    When I run "fspec research --tool=ast --query='find code patterns similar to src/auth/login.ts'"
    Then the ast script should use AST pattern matching
    And the output should include matches from Python files
    And the output should include matches from TypeScript files
    And the output should include matches from Go files
    And each match should show structural similarity score

  Scenario: Parse incomplete code with syntax errors
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And a code file contains syntax errors
    When I run "fspec research --tool=ast --file='src/broken.ts'"
    Then tree-sitter should perform error-tolerant parsing
    And the output should contain partial AST for valid portions
    And the output should indicate which parts failed to parse
    And the command should not fail with exit code 1

  Scenario: Query for classes implementing specific interface
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And the codebase contains classes implementing UserRepository interface
    When I run "fspec research --tool=ast --query='classes implementing interface UserRepository'"
    Then the ast script should execute tree-sitter queries
    And the output should list all implementing classes
    And each result should show file location
    And each result should include code snippet with class declaration

  Scenario: Identify high-complexity functions for refactoring
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And the codebase contains functions with various complexity levels
    When I run "fspec research --tool=ast --query='functions with cyclomatic complexity > 10'"
    Then the ast script should calculate complexity metrics
    And the output should include complexity scores
    And only functions with complexity > 10 should be returned
    And each result should show file path and exact complexity value

  Scenario: Extract exported functions for API documentation
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And the codebase contains exported functions with JSDoc comments
    When I run "fspec research --tool=ast --query='all exported functions' --format=markdown"
    Then the ast script should extract function signatures
    And the output should be formatted as markdown
    And each function should include JSDoc comments
    And each function should show parameter types and return type

  Scenario: Detect unused imports in TypeScript files
    Given the ast research tool exists in spec/research-scripts/
    And the ast tool has executable permissions
    And TypeScript files contain both used and unused imports
    When I run "fspec research --tool=ast --query='unused imports' --language=typescript"
    Then tree-sitter should parse import statements
    And the ast script should check for usage in the AST
    And the output should list only unused imports
    And each result should show import statement and file location
