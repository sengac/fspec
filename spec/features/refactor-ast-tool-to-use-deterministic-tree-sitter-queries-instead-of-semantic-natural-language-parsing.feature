@utils
@RES-017
@high
@research-tools
@ast-analysis
@tree-sitter
@refactoring
Feature: Refactor AST tool to use deterministic tree-sitter queries instead of semantic natural language parsing
  """
  Architecture notes:
  - Query library organized by language (src/utils/ast-queries/{javascript,typescript,python,go,rust}/)
  - Each .scm file contains predefined tree-sitter S-expression queries
  - Query executor loads .scm files, substitutes parameters, executes via tree-sitter Query API
  - Predicate system for parametric filtering (gt-count, gte-count, name-eq, name-matches)
  - CLI uses --operation flag for predefined queries, --query-file for custom queries
  - Breaking change: --query flag completely removed, only --operation and --query-file supported
  - Dependencies: tree-sitter (existing), tree-sitter language grammars (existing)
  - Implementation removes all semantic natural language parsing from ast-parser.ts lines 421-487
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AST tool MUST NOT use semantic natural language parsing (e.g., queryLower.includes('find all'))
  #   2. All AST operations MUST use deterministic tree-sitter query language (S-expressions)
  #   3. CLI MUST use --operation flag instead of --query for predefined operations
  #   4. Query library MUST be organized by language (javascript/, typescript/, python/, go/, rust/)
  #   5. Each operation MUST map to a predefined tree-sitter S-expression query stored in .scm files
  #   6. Power users MUST be able to provide custom queries via --query-file flag pointing to .scm files
  #   7. File flag MUST be required for all AST operations (no implicit glob search)
  #   8. Operations MUST support parametric predicates (e.g., min-params, pattern, name filters)
  #   9. Structural operations MUST include: list-functions, list-classes, list-imports, list-exports, find-class, find-function
  #
  # EXAMPLES:
  #   1. User runs: fspec research --tool=ast --operation=list-functions --file=src/auth.ts, receives JSON with all function declarations, expressions, and arrow functions
  #   2. User runs: fspec research --tool=ast --operation=find-class --name=AuthController --file=src/auth.ts, receives class definition with line numbers
  #   3. User runs: fspec research --tool=ast --operation=find-functions --min-params=5 --file=src/api.ts, receives functions with parameter count >= 5
  #   4. Power user runs: fspec research --tool=ast --query-file=queries/custom-pattern.scm --file=src/utils.ts, executes custom tree-sitter query from .scm file
  #   5. User runs: fspec research --tool=ast --operation=find-identifiers --pattern="^[A-Z][A-Z_]+" --file=src/constants.ts, receives all CONSTANT_CASE identifiers
  #   6. User runs: fspec research --tool=ast --operation=find-async-functions --file=src/api.ts, receives all async function declarations
  #   7. User runs: fspec research --tool=ast --operation=find-exports --export-type=default --file=src/index.ts, receives default export statement
  #
  # ========================================
  Background: User Story
    As a developer using fspec for code analysis
    I want to use deterministic tree-sitter query operations instead of ambiguous natural language queries
    So that I get predictable, composable AST analysis without language-dependent semantic parsing

  Scenario: List all functions using deterministic operation flag
    Given I have a TypeScript file "src/auth.ts" with function declarations, expressions, and arrow functions
    When I run "fspec research --tool=ast --operation=list-functions --file=src/auth.ts"
    Then the output should be valid JSON
    And the JSON should contain an array of function matches
    And each match should include function type (declaration, expression, arrow, method, generator)
    And each match should include line numbers and function names

  Scenario: Find specific class by name
    Given I have a TypeScript file "src/auth.ts" containing class "AuthController"
    When I run "fspec research --tool=ast --operation=find-class --name=AuthController --file=src/auth.ts"
    Then the output should be valid JSON
    And the JSON should contain the class definition
    And the class definition should include line numbers
    And the class definition should include the class body structure

  Scenario: Find functions with parametric predicate filter
    Given I have a TypeScript file "src/api.ts" with functions of varying parameter counts
    When I run "fspec research --tool=ast --operation=find-functions --min-params=5 --file=src/api.ts"
    Then the output should be valid JSON
    And the JSON should contain only functions with 5 or more parameters
    And each match should include the parameter count
    And each match should include line numbers

  Scenario: Execute custom tree-sitter query from file
    Given I have a custom query file "queries/custom-pattern.scm" with tree-sitter S-expression query
    And I have a TypeScript file "src/utils.ts"
    When I run "fspec research --tool=ast --query-file=queries/custom-pattern.scm --file=src/utils.ts"
    Then the tool should load the .scm file content
    And the tool should execute the custom tree-sitter query
    And the output should be valid JSON with matches based on the custom query

  Scenario: Find identifiers matching pattern
    Given I have a TypeScript file "src/constants.ts" with CONSTANT_CASE and camelCase identifiers
    When I run "fspec research --tool=ast --operation=find-identifiers --pattern=\"^[A-Z][A-Z_]+\" --file=src/constants.ts"
    Then the output should be valid JSON
    And the JSON should contain only CONSTANT_CASE identifiers
    And each match should include the identifier name and line number

  Scenario: Find async functions
    Given I have a TypeScript file "src/api.ts" with both async and sync function declarations
    When I run "fspec research --tool=ast --operation=find-async-functions --file=src/api.ts"
    Then the output should be valid JSON
    And the JSON should contain only async function declarations
    And each match should include the function name and line numbers

  Scenario: Find exports by type
    Given I have a TypeScript file "src/index.ts" with default and named exports
    When I run "fspec research --tool=ast --operation=find-exports --export-type=default --file=src/index.ts"
    Then the output should be valid JSON
    And the JSON should contain only the default export statement
    And the match should include line numbers and exported identifier
