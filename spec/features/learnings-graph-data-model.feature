@KGRAPH-020
Feature: Learnings Graph Data Model & Schema
  """
  Uses nanograph PG schema format. Schema stored at codelet/napi/schemas/learnings.pg, bundled via include_str!. Registry in registry.rs manages the singleton.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Learnings graph must use a separate nanograph database from the AST graph and agent-memory graph
  #   2. Schema must define node types: Learning, Exploration, Convention, Decision, CodePattern
  #   3. Schema must define edge types: Discovered, Eliminates, Supersedes, RelatesTo, InformedBy, Applies, Contradicts
  #   4. All node types must have a slug @key property for upsert/merge semantics
  #   5. The database must be global scope stored at ~/.fspec/graph/learnings.nano/ (shared across projects on same machine)
  #   6. Must reuse the GraphDatabase abstraction from database.rs and register via registry.rs
  #   7. Learning nodes must categorize: convention, pattern, anti_pattern, decision, discovery, constraint, reformulation
  #   8. Schema must use Bool (not Boolean) for nanograph PG type compatibility
  #
  # EXAMPLES:
  #   1. Initialize Learnings graph database, write learnings.pg schema file, and verify database opens with correct node/edge types
  #   2. Load batch of Learning and Exploration nodes via JSONL, then query to verify storage with correct categories and properties
  #   3. Load relationship edges (Discovered, Supersedes, RelatesTo) between nodes and traverse to verify structural connections
  #   4. Verify Learnings graph is registered in registry.rs as a named instance alongside agent-memory and ast-code
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to persist and query accumulated learnings (conventions, decisions, patterns, anti-patterns) across sessions
    So that I can benefit from past explorations, avoid repeating failures, and apply proven patterns to new work

  Scenario: Initialize Learnings graph database with schema
    Given the global data directory exists
    And no Learnings graph database has been initialized
    When the Learnings graph database is initialized
    Then the database should be created at "~/.fspec/graph/learnings.nano/"
    And the schema catalog should contain node types "Learning, Exploration, Convention, Decision, CodePattern"
    And the schema catalog should contain edge types "Discovered, Eliminates, Supersedes, RelatesTo, InformedBy, Applies, Contradicts"
    And all node types should have a "slug" key property

  Scenario: Load batch of Learning and Exploration nodes via JSONL
    Given the Learnings graph database is initialized
    When I load a batch of JSONL containing Learning and Exploration nodes
    Then querying for Learning nodes should return the loaded learnings with correct categories
    And querying for Exploration nodes should return the loaded explorations with correct properties
    And no Lance version amplification should occur from the batch load

  Scenario: Load relationship edges and traverse connections
    Given the Learnings graph database is initialized
    And Learning and Exploration nodes have been loaded
    When I load Discovered edges linking explorations to learnings
    And I load Supersedes edges between learnings
    And I load RelatesTo edges between learnings
    Then traversing neighbors of a learning node should return related learnings
    And traversing neighbors of an exploration should return its discovered learnings

  Scenario: Learnings graph registered as named instance in registry
    Given the GraphDatabase registry exists
    When I request the Learnings graph by name "learnings"
    Then the registry should return a valid GraphDatabase instance
    And the instance should be separate from the "ast-code" graph
    And the database path should be under the global data directory
