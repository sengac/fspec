@done
@medium
@validation
@code-quality
@refactor
@validator
Feature: Mermaid validation has global cleanup inconsistency and edge cases
  """
  Improvements to mermaid validation code quality. Fix 1: Error path should restore originalWindow/originalDocument like success path (not unconditionally delete). Fix 2: Use matchAll() to validate ALL subgraphs, not just first one. Fix 3: Add proper TypeScript global declarations to reduce 'as any' usage.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Global cleanup in error path must match success path logic (restore originals if existed, delete if didn't)
  #   2. Multiple subgraphs in a single diagram should all be validated for invalid identifiers
  #   3. TypeScript code should minimize 'as any' usage by using proper global declarations
  #
  # EXAMPLES:
  #   1. Error path cleanup should check originalWindow/originalDocument before deleting or restoring globals
  #   2. Diagram with first valid subgraph and second invalid subgraph 'subgraph INVALID\!\!\!' should be rejected
  #   3. Global window/document/navigator should use proper TypeScript declarations instead of 'as any' casts
  #
  # ========================================
  Background: User Story
    As a developer maintaining mermaid validation code
    I want to have consistent error handling and comprehensive validation
    So that the code is robust and catches all edge cases

  Scenario: Error path cleanup matches success path logic
    Given the mermaid validation function encounters an error during rendering
    When the error cleanup code executes
    Then it should check if originalWindow exists before restoring or deleting
    And it should check if originalDocument exists before restoring or deleting
    And it should match the success path cleanup logic exactly

  Scenario: Validate all subgraphs in diagram not just first
    Given I have a mermaid diagram with multiple subgraphs
    And the first subgraph has a valid identifier "ValidSubgraph"
    And the second subgraph has an invalid identifier "INVALID!!!"
    When I validate the diagram
    Then the validation should reject the diagram
    And the error message should indicate the invalid subgraph identifier

  Scenario: Use proper TypeScript global declarations
    Given the mermaid validation code needs to manipulate global objects
    When TypeScript type checking runs
    Then global window/document/navigator should be properly typed
    And the code should minimize use of 'as any' type assertions
    And TypeScript should not show 'implicit any' warnings
