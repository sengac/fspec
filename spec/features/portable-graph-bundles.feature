@done
@KGRAPH-069
Feature: Portable Graph Bundles — Export/Import
  """
  entities_to_jsonl() and jsonl_to_entities() live in graph_entities.rs. export_all_entities(), export_bundle(), and import_bundle() live in bundle.rs. AstExport/AstImport variants dispatched via graph_search_handler.rs to bundle.rs methods. ZIP handling uses the zip crate with deflate compression.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Bundle format is .astbundle (ZIP archive) containing: entities.jsonl (all nodes and edges in nanograph JSONL format), metadata.json (version, timestamp, node_count, edge_count), schema.pg (nanograph schema source for compatibility check)
  #   2. Export reads all nodes and edges from the graph snapshot — nodes via segment iteration (skipping internal id column), edges via edge_batch_for_save with id→slug resolution
  #   3. Import supports two modes: 'overwrite' (default — replaces all existing data) and 'merge' (upserts via slug-based key matching) — reusing GraphDatabase::load_entities_overwrite and load_entities
  #   4. Import validates schema compatibility before loading — compares bundle schema.pg hash against the current compiled schema, rejects with actionable error if they differ
  #   5. JSONL round-trip uses existing entities_to_jsonl format for export, and a new jsonl_to_entities parser for import — ensures nodes and edges survive serialization without data loss
  #
  # EXAMPLES:
  #   1. Export a graph with Function, File, Type, Dependency nodes and Contains, Calls, Imports edges → .astbundle file is a valid ZIP containing entities.jsonl + metadata.json + schema.pg
  #   2. Import an .astbundle into an empty graph → all nodes and edges are loaded, ast_stats shows same counts as the export source
  #   3. Round-trip test: export → reset graph → import → export again → second bundle has same entity count and JSONL content as first
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to export the AST graph to a portable bundle file and import pre-built bundles
    So that I can share pre-indexed codebases across sessions, teams, and machines without re-indexing

  @graph
  @export
  Scenario: Export creates valid astbundle ZIP archive
    Given a graph indexed with Function, File, Type, and Dependency nodes plus edges
    When I export the graph to an astbundle file
    Then the output is a valid ZIP archive
    And the archive contains entities.jsonl with all nodes and edges
    And the archive contains metadata.json with version and entity counts
    And the archive contains schema.pg matching the current schema

  @graph
  @import
  Scenario: Import loads bundle into empty graph with overwrite mode
    Given an exported astbundle file from a graph with known node and edge counts
    And an empty graph database
    When I import the bundle with overwrite mode
    Then the graph contains the same number of nodes as the export source
    And the graph contains the same number of edges as the export source

  @graph
  @import
  Scenario: Import loads bundle with merge mode
    Given an exported astbundle file containing additional functions
    And a graph with some existing functions
    When I import the bundle with merge mode
    Then the graph contains both the existing and imported functions

  @graph
  @import
  Scenario: Import rejects bundle with incompatible schema
    Given an astbundle file created with a different schema version
    When I attempt to import the bundle
    Then the import fails with a schema mismatch error
    And no data in the graph is modified

  @graph
  @roundtrip
  Scenario: Export and import round-trip preserves all data
    Given a graph indexed with multiple node types and edge types
    When I export the graph to a bundle
    And I reset the graph
    And I import the bundle
    Then ast_stats shows the same node and edge counts as before export

  @graph
  @serialization
  Scenario: JSONL round-trip serializes and deserializes all entity types
    Given a list of GraphEntity nodes and edges with various property types
    When I serialize them with entities_to_jsonl
    And I deserialize the result with jsonl_to_entities
    Then the deserialized entities match the originals in type, slugs, and properties
