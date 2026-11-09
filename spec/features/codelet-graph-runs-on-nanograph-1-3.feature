@done
@KGRAPH-072
@parser
@code-analysis
@rust
Feature: codelet-graph runs on nanograph 1.3.0
  """
  nanograph 1.3.0 removes Database::snapshot (store::graph is pub(crate)); stats come from store::metadata::DatabaseMetadata::open(path).manifest().datasets[*].row_count (kind node/edge) and export from store::export::build_export_rows_at_path(path, false, false), which already yields {type,data}/{edge,from,to,data} rows keyed by @key. Legacy detection uses store::storage_generation::read_storage_generation(path) == None on a directory holding schema.ir.json, the same test DatabaseMetadata::open uses. Fixtures: rust/graph/tests/fixtures/kgraph072/{ast-code,learnings}.nano written by nanograph 1.0.0 (codelet-graph at 7fb2449d5).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. codelet-graph depends on nanograph 1.3.0; its own arrow-array and arrow-schema pins are removed because graph statistics come from nanograph's manifest metadata and export rows from nanograph's export API (no Database::snapshot, no hand-rolled Arrow decoding, no compatibility shims)
  #   2. A code graph on disk in nanograph 1.0.0's legacy v3 storage is derived data: ast_index deletes it and builds a fresh graph; a query on it fails with an explicit error that says the graph predates nanograph 1.3.0 and that ast_index rebuilds it
  #   3. A learnings graph on disk in legacy v3 storage is moved to a timestamped backup beside it (learnings.nano.v3-backup-<UTC timestamp>) and a fresh empty learnings graph is started; nothing is migrated and the backup path is logged (product owner, 2026-11-07)
  #   4. Every entity the indexer and the learnings extractor load names only properties of the compiled schema, so nanograph 1.3.0's loader (which rejects unknown properties) accepts a full index of a multi-language fixture
  #   5. The bundled queries that traverse the contains edge ($f contains $fn) parse and run under nanograph 1.3.0
  #
  # EXAMPLES:
  #   1. A project's code graph written by nanograph 1.0.0 is on disk; ast_index succeeds and the graph on disk is in 1.3.0 storage holding the project's functions
  #   2. ast_search on a project whose code graph is still in 1.0.0 storage fails naming nanograph 1.3.0 and ast_index
  #   3. The learnings graph on disk is in 1.0.0 storage; the first learnings_stats call returns zero counts from a fresh graph and the old directory sits at learnings.nano.v3-backup-<timestamp> with its files intact
  #   4. ast_stats on an indexed fixture reports the node and edge counts the index loaded
  #   5. ast_export then ast_import of an indexed fixture into another root gives the same entities
  #   6. file_functions ($f contains $fn) returns the functions of an indexed file
  #
  # ========================================
  Background: User Story
    As a developer maintaining fspec's code graph
    I want to run codelet-graph on nanograph 1.3.0 with the arrow, datafusion and lance stack it brings
    So that the graph gets 1.3.0's metadata-first open, stricter loader and upstream fixes, and graphs written by 1.0.0 are handled explicitly instead of failing to open

  Scenario: ast_index rebuilds a code graph written by nanograph 1.0.0
    Given a project whose code graph on disk was written by nanograph 1.0.0
    When ast_index runs for the project
    Then the index succeeds
    And the code graph on disk is in nanograph 1.3.0 storage
    And ast_search lists the project's functions

  Scenario: A query on a code graph written by nanograph 1.0.0 names the rebuild
    Given a project whose code graph on disk was written by nanograph 1.0.0
    When ast_search runs for the project
    Then it fails with an error naming nanograph 1.3.0 and ast_index

  Scenario: A learnings graph written by nanograph 1.0.0 is backed up and started fresh
    Given the learnings graph on disk was written by nanograph 1.0.0
    When learnings_stats runs
    Then it reports zero learnings from a fresh learnings graph
    And the old learnings graph sits beside it at learnings.nano.v3-backup-<timestamp> with its files intact

  Scenario: ast_stats reports the counts the index loaded
    Given a project with Python and Rust sources has been indexed
    When ast_stats runs
    Then the File, Function and Contains counts equal the entities the index loaded

  Scenario: An exported code graph imports into another root unchanged
    Given a project with Python and Rust sources has been indexed
    When its graph is exported with ast_export and imported into another root with ast_import
    Then the other root's graph exports the same entities

  Scenario: Queries over the contains edge run on nanograph 1.3.0
    Given a project with Python and Rust sources has been indexed
    When the file_functions query runs for an indexed file
    Then it returns the functions of that file

  Scenario: A query over an edge type with no edges returns no rows
    Given a project whose Python files import nothing has been indexed
    When the file_imports query runs for one of its files
    Then it returns no rows instead of failing

  Scenario: A learnings edge survives loading another edge type and a reopen
    Given a learnings graph with an exploration that discovered two learnings
    When a Supersedes edge between two learnings is loaded
    And the learnings graph is reopened
    Then exploration_discoveries still returns both discovered learnings

  Scenario: codelet-graph never uses a Merge load
    Given the codelet-graph sources
    When every nanograph load call in them is collected from the parsed syntax tree
    Then the only load call is load_with_mode with LoadMode::Overwrite
    And no Database::load call and no LoadMode::Merge or LoadMode::Append appears
