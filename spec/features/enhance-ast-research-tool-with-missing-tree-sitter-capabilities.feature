@research-tools
@code-analysis
@research
@ast
@tree-sitter
@query-api
@high
@RES-021
Feature: Enhance AST Research Tool with Missing Tree-Sitter Capabilities
  """
  Phase 1 implementation uses tree-sitter Query API for S-expression pattern matching. Query operations support both inline --query and external --query-file parameters. Field-based access (childForFieldName) replaces type-based child searches for reliability. closest() method enables context-aware navigation. All changes maintain backward compatibility with existing operations (list-functions, find-class, etc.). Query predicates support #eq?, #match?, #any-of? for filtering captures. Output transformed to QueryMatch interface for consistency.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Phase 1 implementation must include Query System (S-expression pattern matching), Field-Based Access, and closest() method
  #   2. Query operations must support inline queries via --query parameter and external .scm files via --query-file parameter
  #   3. Field-based access (childForFieldName) must be used internally instead of type-based child searches for reliability
  #   4. New operations must be backward compatible - existing operations (list-functions, find-class, etc.) continue to work
  #   5. Query predicates must support at minimum: #eq?, #match?, #any-of? for filtering captures
  #   6. Defer TreeCursor to Phase 2 (v0.10.0) - Phase 1 focuses on Query System, Field Access, and closest() for high-impact features first
  #   7. Transform to QueryMatch interface for consistency with existing operations - include captures metadata in extended format
  #   8. Separate work unit - fspec review enhancement is RES-022, Phase 1 provides the tools (query operations) that review can use later
  #
  # EXAMPLES:
  #   1. Developer uses --operation=query --query='(function_declaration name: (identifier) @name)' to find all function names declaratively instead of manual traversal
  #   2. Developer uses --operation=find-context --row=42 --column=10 --context-type=function to find the containing function at a specific position (uses closest() internally)
  #   3. Developer creates queries/anti-patterns.scm with pattern for empty catch blocks and runs --query-file=queries/anti-patterns.scm to detect code smells
  #   4. Existing operation 'fspec research --tool=ast --operation=list-functions' continues to work exactly as before (backward compatibility verified)
  #   5. Query with predicate: '(function_declaration name: (identifier) @name (#match? @name "^handle"))' finds only functions starting with 'handle'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should Phase 1 include TreeCursor performance optimization or defer to Phase 2?
  #   A: true
  #
  #   Q: Should query operations return raw tree-sitter captures or transform them into the existing QueryMatch interface format?
  #   A: true
  #
  #   Q: Should fspec review command be enhanced with anti-pattern detection in Phase 1, or is that a separate work unit?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec AST research tools
    I want to leverage advanced tree-sitter capabilities for powerful code analysis
    So that I can perform complex pattern matching, improve performance, and analyze code more effectively

  Scenario: Use inline query to find function names declaratively
    Given I have a TypeScript file with multiple function declarations
    When I run 'fspec research --tool=ast --operation=query --query="(function_declaration name: (identifier) @name)" --file=src/test.ts'
    Then the output should contain all function names as captures
    And the output should be in QueryMatch interface format

  Scenario: Find containing function using find-context operation
    Given I have a TypeScript file with nested functions
    When I run 'fspec research --tool=ast --operation=find-context --row=42 --column=10 --context-type=function --file=src/test.ts'
    Then the output should return the function declaration containing position 42:10
    And the operation should use closest() method internally

  Scenario: Use external query file to detect anti-patterns
    Given I have created a file queries/anti-patterns.scm with a pattern for empty catch blocks
    When I run 'fspec research --tool=ast --query-file=queries/anti-patterns.scm --file=src/test.ts'
    Then the output should list all empty catch blocks detected
    And I have a TypeScript file with try-catch blocks
    And each result should include line number and code snippet

  Scenario: Maintain backward compatibility with existing operations
    Given I have been using 'fspec research --tool=ast --operation=list-functions' in my workflow
    When I run 'fspec research --tool=ast --operation=list-functions --file=src/test.ts'
    Then the operation should work exactly as before
    And the output format should be unchanged
    And all existing tests should pass without modification

  Scenario: Filter query results using predicates
    Given I have a TypeScript file with various function names
    When I run a query with #match? predicate: 'fspec research --tool=ast --operation=query --query="(function_declaration name: (identifier) @name (#match? @name \"^handle\"))" --file=src/test.ts'
    Then the output should only include functions whose names start with 'handle'
    And functions with other names should be excluded
