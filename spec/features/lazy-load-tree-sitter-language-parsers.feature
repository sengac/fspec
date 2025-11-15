@done
@research
@high
@utils
@ast
@research-tools
@tree-sitter
@refactor
@performance
@PERF-001
Feature: Lazy Load Tree-Sitter Language Parsers
  """
  Implement parser caching using Map to avoid re-importing same language parser
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Language parsers must be loaded on-demand using dynamic imports, not eagerly at module load time
  #   2. Only the parser needed for the target file's language should be loaded
  #   3. Loaded parsers should be cached to avoid re-importing the same language parser multiple times
  #   4. Refactoring must not change the external API of ast-parser.ts or research-tools/ast.ts
  #
  # EXAMPLES:
  #   1. Developer analyzes a TypeScript file using 'fspec research --tool=ast --file=src/auth.ts' and only the TypeScript parser is loaded (not all 15 parsers)
  #   2. Developer analyzes multiple TypeScript files and the TypeScript parser is loaded once and cached for subsequent analyses
  #   3. CLI startup time improves by 85-93% when analyzing a single TypeScript file (from ~750-3000ms to ~50-200ms)
  #   4. All existing tests pass after refactoring without modification
  #
  # ========================================
  Background: User Story
    As a developer using fspec AST research tools
    I want to have fast CLI startup time when analyzing code
    So that I can quickly analyze single files without loading all 15 language parsers unnecessarily

  Scenario: Load only required language parser for single file analysis
    Given I have a TypeScript file at src/auth.ts
    When I run fspec research --tool=ast --file=src/auth.ts
    Then only the TypeScript parser should be dynamically imported
    And the other 14 language parsers should NOT be loaded

  Scenario: Cache parser for multiple analyses of same language
    Given I have multiple TypeScript files
    When I analyze the first TypeScript file
    Then the TypeScript parser should be imported only once
    And I analyze a second TypeScript file
    And the cached parser should be reused for the second analysis

  Scenario: Improve CLI startup time by 85-93% for single language
    Given the baseline startup time with eager loading is 750-3000ms
    When I analyze a single TypeScript file with lazy loading
    Then the startup time should be reduced to 50-200ms
    And the performance improvement should be 85-93%

  Scenario: Maintain backward compatibility with existing tests
    Given the existing AST parser tests are passing
    When I refactor to use lazy loading
    Then all existing tests should still pass
    And the external API of ast-parser.ts should remain unchanged
    And the external API of research-tools/ast.ts should remain unchanged
