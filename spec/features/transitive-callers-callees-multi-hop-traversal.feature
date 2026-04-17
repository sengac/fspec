@done
@KGRAPH-061
Feature: Transitive Callers / Callees (Multi-Hop Traversal)
  """
  Implement ast_callers and ast_callees as two new GraphSearchAction variants sharing the BFS traversal module from KGRAPH-060. ast_callers traverses reversed adjacency list (incoming Calls edges), ast_callees traverses forward adjacency list (outgoing Calls edges). Wire in graph_search_handler.rs and types.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. New action_type 'ast_callers' accepts function slug/name (as node_id), optional max_depth (default 5), and optional limit (default 50); returns all direct and transitive callers
  #   2. New action_type 'ast_callees' accepts function slug/name (as node_id), optional max_depth (default 5), and optional limit (default 50); returns all direct and transitive callees
  #   3. Results include the hop distance from the source function, so agents can see which are direct (depth 1) vs transitive (depth 2+)
  #   4. Reuses BFS infrastructure from KGRAPH-060 (call chain) — same Rust BFS module, different output format (flat list with depth vs ordered chains)
  #   5. Each result includes function slug, name, file path, lineStart, lineEnd, and depth from source — matching CGC's output format
  #
  # EXAMPLES:
  #   1. Agent asks ast_callers for 'dispatch_ast_search' — returns dispatch_action (depth 1), create_handler closure (depth 2), and any higher-level callers
  #   2. Agent asks ast_callees for a high-level entry point — returns all functions it transitively calls, each annotated with their depth from the entry point
  #   3. Agent asks ast_callers for a leaf function with no callers — returns empty results array
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to find all transitive callers or callees of a function across the entire call graph
    So that I can understand the full blast radius of changes and identify all affected code paths

  @happy-path
  Scenario: Find all transitive callers of a function
    Given I have a codebase indexed with multi-level call chains
    When I request ast_callers for a deeply-called function
    Then I should receive a list of all direct and transitive callers
    And each caller should include its depth from the target function
    And depth 1 callers should be the direct callers
    And depth 2+ callers should be the transitive callers

  @happy-path
  Scenario: Find all transitive callees of an entry point
    Given I have a codebase indexed with multi-level call chains
    When I request ast_callees for a high-level entry point function
    Then I should receive a list of all functions it transitively calls
    And each callee should include slug, name, file path, line numbers, and depth

  @edge-case
  Scenario: Function with no callers returns empty results
    Given I have a codebase indexed in the AST graph
    When I request ast_callers for a function that is never called
    Then I should receive an empty results array

  @edge-case
  Scenario: Function with no callees returns empty results
    Given I have a codebase indexed in the AST graph
    When I request ast_callees for a leaf function that calls nothing
    Then I should receive an empty results array

  @edge-case
  Scenario: Max depth limits transitive traversal
    Given I have a codebase indexed with a call chain of depth 4
    When I request ast_callees with max_depth 2
    Then I should only receive callees within 2 hops
    And functions at depth 3 and beyond should not appear in results

  @error
  Scenario: Non-existent function returns error
    Given I have a codebase indexed in the AST graph
    When I request ast_callers for a non-existent function slug
    Then I should receive an error indicating the function was not found
