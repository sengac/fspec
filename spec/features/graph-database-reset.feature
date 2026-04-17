@KGRAPH-071
Feature: GraphSearch ast_index has no way to force-rebuild database after schema changes — stale on-disk and in-memory graph causes repeated failures
  """
  The schema is embedded via include_str!() at compile time — the binary always has the correct schema, but the on-disk DB preserves the schema from when it was first created
  KGRAPH-010 added schema hash comparison on open but blocks breaking changes with an error — this card needs to provide the rebuild path that KGRAPH-010 is missing
  Uses lazy_static! { Mutex<HashMap<String, GraphDatabase>> } as singleton registry — reset_all_graphs() already exists (clears HashMap). Need a reset_graph(name) that removes a single graph. GraphDatabase wraps Arc<Database> so Drop will clean up when all references are gone.
  No schema hash comparison exists despite KGRAPH-010 claiming it was done — open_or_init simply checks if schema.ir.json exists and opens blindly. The schema_source param is ignored when opening existing DB.
  AstIndex is the only action that doesn't call get_graph_or_err() first — it handles its own graph acquisition inside dispatch_ast_index(). The reset logic goes here: delete dir, remove from registry, then proceed with normal indexing which will init fresh.
  DB paths: AST = <cwd>/.fspec/graph/ast-code.nano/, Learnings = ~/.fspec/graph/learnings.nano/ — resolved in registry::resolve_graph_config(). Serde enum variant: GraphSearchAction::AstIndex { path: Option<String> } — add reset: Option<bool> here.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ast_index with reset flag must delete the on-disk .nano directory (e.g., ast-code.nano/) before opening a fresh database
  #   2. When schema hash mismatch is detected on normal ast_index (no reset flag), the error message must tell the user to re-index with the reset flag instead of just failing
  #   3. The reset flag must also apply to the learnings graph if it exists, not just the AST graph
  #   4. After reset + re-index, subsequent ast_search/ast_neighbors/ast_dead_code queries must work without process restart
  #   5. ast_index with reset flag must remove the graph from the lazy_static Mutex<HashMap> registry so the next get_graph() re-initializes with fresh schema
  #   6. open_or_init must compare compiled schema hash against on-disk schema.pg hash — if mismatch, return actionable error rather than silently opening with stale schema
  #
  # EXAMPLES:
  #   1. Schema changes to ast-code.pg (add 'extension' to Type.typeKind), rebuild binary, ast_index fails with 'invalid enum value extension' because old DB on disk has stale schema — reset flag deletes DB, re-indexes successfully
  #   2. Schema changes to ast-code.pg (add 'pubspec' to Dependency.source), user manually deletes .nano dir, but in-memory graph singleton still has old schema — reset flag clears both disk AND memory, re-index works
  #   3. ast_index without reset flag hits schema mismatch — error message says 'Schema has changed. Run ast_index with reset: true to rebuild the database' instead of cryptic enum error
  #   4. After reset + re-index of fspec codebase, ast_search for 'function' immediately returns results (no process restart needed)
  #
  # ========================================
  Background: User Story
    As a developer
    I want to force-rebuild the graph database after schema changes
    So that I don't have to manually delete database files and restart the process every time the schema changes

  @reset
  Scenario: Reset flag deletes on-disk database and re-indexes with fresh schema
    Given an existing AST graph database at "<cwd>/.fspec/graph/ast-code.nano/"
    And the compiled schema has a new enum value not present in the on-disk schema
    When I run ast_index with reset set to true
    Then the on-disk ".nano" directory is deleted before re-indexing
    And a fresh database is initialized with the compiled schema
    And the index completes successfully with entity counts

  @reset
  Scenario: Reset flag clears in-memory graph singleton so fresh schema takes effect
    Given an AST graph is cached in the in-memory registry
    And the on-disk database has been manually deleted
    When I run ast_index with reset set to true
    Then the graph is removed from the in-memory Mutex<HashMap> registry
    And the next get_graph call re-initializes with the compiled schema
    And subsequent ast_search queries return results without process restart

  @schema-mismatch
  Scenario: Schema mismatch without reset flag returns actionable error
    Given an existing AST graph database created with an older schema
    And the compiled schema has changed since the database was created
    When I run ast_index without the reset flag
    Then the error message includes "Schema has changed"
    And the error message tells the user to re-index with reset set to true

  @reset
  @queries
  Scenario: Queries work immediately after reset and re-index
    Given an AST graph database has been reset and re-indexed
    When I run ast_search with query "function"
    Then results are returned from the freshly indexed graph
    When I run ast_neighbors with a valid node_id from the new index
    Then neighbors are returned successfully
    When I run ast_dead_code
    Then dead code analysis completes without schema errors
