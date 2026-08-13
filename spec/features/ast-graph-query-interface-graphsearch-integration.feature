@KGRAPH-019
Feature: AST Graph Query Interface & GraphSearch Integration
  """
  AST dispatch in rust/napi/src/graph/ast_dispatch.rs. Query source in rust/napi/schemas/ast-queries.gq (loaded via include_str!). New action variants: AstSearch, AstNeighbors, AstStats added to GraphSearchAction enum. Handler routes AST-prefixed actions to ast_dispatch.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. New AST action types must be added to the GraphSearchAction enum in tools/src/graph_search/types.rs
  #   2. AST queries must use the AST graph database (via database.rs GraphDatabase), NOT the existing agent-memory graph
  #   3. AST dispatch functions must be in a separate file (ast_dispatch.rs) from the existing dispatch.rs
  #   4. PG queries for AST data must be in a separate file (ast-queries.gq or bundled constant) from existing agent-memory queries
  #   5. Must support at minimum: search entities by name/type, navigate neighbors of a node, and get codebase statistics
  #   6. The existing dispatch handler must route AST actions to AST dispatch and learnings/memory actions to the existing dispatch — transparent to the caller
  #
  # EXAMPLES:
  #   1. Search for a function by name 'login' and get back Function node with slug, file path, line range, and parameter count
  #   2. Get neighbors of a Function node to see which files contain it and which other functions it calls, traversing Contains and Calls edges
  #   3. Get AST codebase statistics showing counts of File, Function, Type, and Dependency nodes plus total edge counts
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to query the AST graph for code structure information like function lookups, call chains, and dependency trees through the existing GraphSearch tool
    So that I can navigate code structure, assess change impact, find related functions, and understand module dependencies without reading every file

  Scenario: Search for a function by name using AstSearch action
    Given the AST graph database is initialized with File and Function nodes
    When I execute an AstSearch action with query "login"
    Then the result should contain a Function node matching "login"
    And the result should include the function's slug, name, and qualifiedName
    And the result should include lineStart and lineEnd positions
    And the result should include paramCount

  Scenario: Get neighbors of a Function node using AstNeighbors action
    Given the AST graph database contains Function nodes with Contains and Calls edges
    When I execute an AstNeighbors action for a Function node slug
    Then the result should include the File node that contains the function via Contains edge
    And the result should include other Function nodes linked by Calls edges

  Scenario: Get AST codebase statistics using AstStats action
    Given the AST graph database contains various node and edge types
    When I execute an AstStats action
    Then the result should include counts for File, Function, Type, and Dependency nodes
    And the result should include total edge counts
