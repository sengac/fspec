@KGRAPH-060
Feature: Call Chain / Path Tracing Between Two Functions
  """
  Add AstCallChain variant to GraphSearchAction enum in types.rs. Implement BFS traversal in new ast_call_chain module under rust/napi/src/graph/. Wire dispatch in graph_search_handler.rs. Reuse existing function_calls .gq query for single-hop adjacency at each BFS level. Pre-fetch all function data into a GraphSnapshot to avoid redundant queries. Register in tool description for LLM discovery.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. New action_type 'ast_call_chain' must accept 'from' (source function slug or name), 'to' (target function slug or name), and optional 'max_depth' (default 5)
  #   2. Returns an array of chains, each chain being an ordered list of function slugs/names from source to target
  #   3. Chains are ordered by length (shortest first), limited to max 20 results to prevent explosion on deeply connected graphs
  #   4. Implementation uses BFS in Rust over iterative single-hop Calls edge queries — nanograph doesn't support variable-length path traversal
  #   5. If no path exists between source and target within max_depth, returns empty chains array with a message
  #   6. If 'from' or 'to' doesn't resolve to an existing function node, returns an error with 'not found' message
  #   7. Each chain result includes both function_chain (node metadata) and call_details (edge metadata per hop) — each call_detail includes from/to slugs, callCount, and isConditional from the Calls edge schema
  #   8. Each chain includes a chain_length integer representing the number of hops — matching CGC's 'length(path) as chain_length'
  #   9. Successful response includes a summary string 'Found N call chain(s) from X to Y (max depth: D)' — matching CGC handler summary format
  #
  # EXAMPLES:
  #   1. Agent asks GraphSearch for call chain from 'func_a' to 'func_b' — returns a single chain of length 1 (direct call)
  #   2. Agent asks for call chain from function A to function D where there's a 3-hop path (A→B→C→D) — returns the full chain with all intermediate functions
  #   3. Agent asks for call chain between two unconnected functions — returns empty chains array with message 'No call path found within depth 5'
  #   4. Agent asks for call chain with a non-existent function slug — returns error 'Function not found: nonexistent_function'
  #   5. Agent asks for call chain with max_depth=2 but the shortest path is 3 hops — returns empty chains, but with max_depth=3 it finds the path
  #   6. Agent asks for call chain and gets back both function_chain with metadata and call_details with edge info for each hop
  #   7. Agent gets response with summary 'Found 2 call chain(s) from entry to target (max depth: 5)' — human-readable summary included
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to trace the call chain between two functions to understand how function A reaches function B
    So that I can perform impact analysis and understand unfamiliar code paths

  @happy-path
  Scenario: Direct call chain between two functions
    Given I have a codebase indexed in the AST graph with Calls edges
    When I request ast_call_chain from "func_a" to "func_b"
    Then I should receive a chains array containing one chain of length 1
    And the chain should list both functions in order from source to target

  @happy-path
  Scenario: Multi-hop call chain with intermediate functions
    Given I have a codebase indexed where function A calls B, B calls C, and C calls D
    When I request ast_call_chain from "func_a" to "func_d"
    Then I should receive a chains array containing a chain of length 3
    And the chain should include all intermediate functions in order A, B, C, D

  @edge-case
  Scenario: No path exists between two unconnected functions
    Given I have a codebase indexed with two functions that have no call path between them
    When I request ast_call_chain from "func_a" to "isolated"
    Then I should receive an empty chains array
    And the response should include a message indicating no path was found within the depth limit

  @error
  Scenario: Non-existent source function slug
    Given I have a codebase indexed in the AST graph
    When I request ast_call_chain from "nonexistent_function" to "func_b"
    Then I should receive an error indicating the source function was not found

  @error
  Scenario: Non-existent target function slug
    Given I have a codebase indexed in the AST graph
    When I request ast_call_chain from "func_a" to "nonexistent_function"
    Then I should receive an error indicating the target function was not found

  @edge-case
  Scenario: Max depth limits path discovery
    Given I have a codebase indexed where the shortest path between two functions is 3 hops
    When I request ast_call_chain with max_depth 2
    Then I should receive an empty chains array
    When I request ast_call_chain with max_depth 3
    Then I should receive a chains array containing the 3-hop path

  @happy-path
  Scenario: Multiple paths returned ordered by length
    Given I have a codebase indexed with both a 2-hop and a 3-hop path between two functions
    When I request ast_call_chain between those two functions
    Then the chains array should contain the shorter path first
    And results should be limited to at most 20 chains

  @happy-path
  Scenario: Chain results include function metadata and call details per hop
    Given I have a codebase indexed in the AST graph with Calls edges
    When I request ast_call_chain from "func_a" to "func_b"
    Then each chain should contain a function_chain array with node metadata for each function
    And each chain should contain a call_details array with edge metadata for each hop
    And each chain should include a chain_length integer

  @happy-path
  Scenario: Successful response includes human-readable summary
    Given I have a codebase indexed in the AST graph with Calls edges
    When I request ast_call_chain from "func_a" to "func_b"
    Then the response should include a summary string describing the number of chains found
