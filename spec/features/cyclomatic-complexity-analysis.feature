@done
@KGRAPH-062
Feature: Cyclomatic Complexity Analysis
  """
  Two-phase approach: (1) Add cyclomaticComplexity to ast-code.pg schema + calculate via shared text-based keyword matcher (complexity.rs) in all 14 extractors during ast_index, (2) Add ast_complexity query action via ast_complexity.rs. Schema change requires database reset.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Cyclomatic complexity must be calculated during AST extraction and stored as a property on Function nodes in the graph schema
  #   2. Complexity is calculated by counting decision points: if, else if, while, for, case/match arms, catch, &&, || — formula: 1 + decision_points
  #   3. New action_type 'ast_complexity' accepts optional node_id (function slug for single-function lookup), optional limit (default 20 for top-N mode), optional min_threshold, and optional path filter
  #   4. The ast-code.pg schema adds a 'cyclomaticComplexity: I32?' optional property to the Function node
  #   5. All 14 language extractors calculate complexity during extraction via a shared text-based keyword matcher (complexity.rs) — each language has a data-driven config of decision-point keywords and operators
  #
  # EXAMPLES:
  #   1. Agent asks for top 10 most complex functions — receives list sorted by cyclomatic complexity descending with function name, file path, line numbers, and complexity score
  #   2. Agent asks for complexity of a specific function — receives the function's complexity score, e.g. 'process_data has cyclomatic complexity 8'
  #   3. A simple getter function with no branches has complexity 1. A function with 5 if/else branches has complexity 6.
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to query the cyclomatic complexity of functions and find the most complex functions in a codebase
    So that I can identify code hotspots that need refactoring and prioritize code review

  @happy-path
  Scenario: Find top N most complex functions in a codebase
    Given I have a codebase indexed with cyclomatic complexity calculated for all functions
    When I request ast_complexity with limit 10
    Then I should receive a list of functions sorted by complexity descending
    And each result should include function name, file path, line numbers, and complexity score

  @happy-path
  Scenario: Query complexity of a specific function
    Given I have a codebase indexed with cyclomatic complexity calculated
    When I request ast_complexity for a specific function slug
    Then I should receive that function's cyclomatic complexity score
    And the response should include the function name, file path, and line numbers

  @happy-path
  Scenario: Simple function has complexity 1
    Given I have indexed a function with no branches or decision points
    When I query its cyclomatic complexity
    Then the complexity score should be 1

  @happy-path
  Scenario: Function with multiple branches has correct complexity
    Given I have indexed a function with 5 if/else branches
    When I query its cyclomatic complexity
    Then the complexity score should be 6

  @integration
  Scenario: Complexity is populated during ast_index
    Given I have a codebase that has not been indexed
    When I run ast_index on the codebase
    Then the Function nodes in the graph should have cyclomaticComplexity values
    And functions with decision points should have complexity greater than 1

  @error
  Scenario: Non-existent function returns error
    Given I have a codebase indexed in the AST graph
    When I request ast_complexity for a non-existent function slug
    Then I should receive an error indicating the function was not found
