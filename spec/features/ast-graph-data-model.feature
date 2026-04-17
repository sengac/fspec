@KGRAPH-016
Feature: AST Graph Data Model & Nanograph Schema
  """
  Uses nanograph PG schema format for the AST graph schema definition
  Refactors existing graph/mod.rs singleton pattern into a reusable GraphDatabase struct that wraps nanograph::Database with init/open/close/load/query methods
  AST schema stored at codelet/napi/schemas/ast-code.pg, bundled via include_str! like the existing agent-memory schema
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The AST graph must use a separate nanograph database from the existing agent-memory graph and any future learnings graph
  #   2. The schema must define node types for: File, Module, Function, Type, and Dependency
  #   3. The schema must define edge types for structural relationships: Contains, ContainsType, Imports, Calls, Implements, Extends, TypeRef, DependsOn
  #   4. All node types must have a @key property (slug) for upsert/merge semantics
  #   5. The graph database must be project-scoped (stored under .fspec/graph/ast-code.nano/) not global
  #   6. The database lifecycle (init/open/close/reset) must use the same lazy singleton pattern as the existing graph module but as a separate instance
  #   7. Loading data must use batch JSONL loading (not per-entity) to avoid Lance version amplification
  #   8. Schema file must support both TypeScript/JavaScript and Rust AST entity types
  #   9. Database lifecycle code must be extracted into a reusable GraphDatabase abstraction that both AST and Learnings graphs can share
  #
  # EXAMPLES:
  #   1. Initialize AST graph database from project root, write ast-code.pg schema file, and verify database opens with the correct node/edge types by querying the catalog
  #   2. Load batch of File and Function nodes via JSONL, then query to verify they are stored with correct properties and slugs
  #   3. Load Contains and Calls edges between Function nodes, then traverse neighbors to verify structural connections
  #   4. Reuse the GraphDatabase abstraction for a second database instance with a different schema to prove separation
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to query a structural graph of the codebase's AST relationships
    So that I can understand code structure, find call chains, and assess impact of changes

  Scenario: Initialize AST graph database with schema
    Given the project root directory exists
    And no AST graph database has been initialized
    When the AST graph database is initialized
    Then the database should be created at ".fspec/graph/ast-code.nano/"
    And the schema catalog should contain node types "File, Module, Function, Type, Dependency"
    And the schema catalog should contain edge types "Contains, ContainsType, Imports, Calls, Implements, Extends, TypeRef, DependsOn"
    And all node types should have a "slug" key property

  Scenario: Load batch of File and Function nodes via JSONL
    Given the AST graph database is initialized
    When I load a batch of JSONL containing File and Function nodes
    Then querying for File nodes should return the loaded files with correct properties
    And querying for Function nodes should return the loaded functions with correct slugs
    And no Lance version amplification should occur from the batch load

  Scenario: Load structural edges and traverse neighbors
    Given the AST graph database is initialized
    And File and Function nodes have been loaded
    When I load Contains edges linking files to functions
    And I load Calls edges linking functions to other functions
    Then traversing neighbors of a function node should return its callers and callees
    And traversing neighbors of a file node should return its contained functions

  Scenario: Reusable GraphDatabase abstraction supports multiple instances
    Given a GraphDatabase abstraction exists
    When I create one instance with the AST code schema
    And I create another instance with a different schema
    Then both databases should initialize independently
    And data loaded into one should not appear in the other
    And both databases should support the same load and query operations

  Scenario: Re-open existing AST graph database
    Given the AST graph database was previously initialized with data
    When the database singleton is reset
    And the AST graph database is re-initialized
    Then the previously loaded data should still be available
    And the schema should match the original schema
