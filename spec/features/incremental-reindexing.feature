@done
@KGRAPH-065
Feature: Incremental Re-indexing

  """
  Post-process mtime stamping on File nodes via stamp_file_mtimes() rather than changing all 14 language extractor signatures.
  New ast_pipeline/incremental.rs module with: collect_file_mtimes(), read_stored_mtimes(), partition_changed_files(), filter_reusable_entities(), stamp_file_mtimes() — reuses export_all_entities() from the graph database.
  Add incremental bool parameter to dispatch_ast_index in ast_index.rs; read_stored_mtimes() exports all entities and extracts lastModified from File nodes (no separate gq query needed).
  Entity ownership is determined by slug prefix — entities with slugs starting with a file slug belong to that file (e.g., src-foo-rs::main belongs to file src-foo-rs). Edges where from_slug starts with a changed file slug are considered owned by that file.
  Dependency nodes (slug starting with dep::) and DependsOn edges are always excluded from reuse and re-extracted every time.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. File modification times (mtime) are stored on File nodes in the graph using the existing lastModified DateTime property
  #   2. Incremental mode compares filesystem mtimes against stored mtimes to identify changed, new, and deleted files
  #   3. Only changed and new files undergo AST extraction — unchanged file entities are reused from the existing graph via export
  #   4. Deleted files are excluded by not including their entities in the overwrite load
  #   5. If more than 50% of files changed or no prior index exists, incremental mode falls back to full extraction
  #   6. Cross-file edges from unchanged files may be stale if targets were renamed — deduplicate_entities() prunes dangling edges by validating both endpoints exist and match schema-expected types
  #   7. The full overwrite-load strategy is preserved — incremental combines reused + fresh entities, deduplicates, then overwrites, so stale entities are always cleaned up
  #   8. Dependencies (Cargo.toml, package.json, etc.) are always re-extracted in incremental mode since they are fast and small — Dependency nodes and DependsOn edges are excluded from reuse
  #
  # EXAMPLES:
  #   1. Full index stores mtime on all File nodes; subsequent incremental index detects no changes and returns immediately with 0 re-extracted files
  #   2. After modifying one file, incremental re-index extracts only that file and reuses all other entities from the graph
  #   3. After deleting a file, incremental re-index removes the deleted file's entities (File, Function, Type nodes and edges)
  #   4. After adding a new file, incremental re-index extracts it as a new file and includes it alongside reused entities
  #   5. When incremental is used on an empty graph (no prior index), falls back to full extraction and loads all entities
  #   6. When incremental is false or omitted, full extraction + overwrite works as before (backward compatibility)
  #
  # ========================================

  Background: User Story
    As an AI agent
    I want my AST graph to be incrementally re-indexed when only a few files have changed
    So that I avoid the expensive full re-index on large codebases and get fast graph updates

  Scenario: Full index stores mtime on File nodes
    Given a project directory with multiple source files
    When I run a full ast_index on the project
    Then every File node in the graph has a lastModified property set to the file's filesystem mtime

  Scenario: Incremental re-index detects no changes and skips extraction
    Given a project that has been fully indexed with mtime-stamped File nodes
    And no source files have been modified since the last index
    When I run an incremental ast_index on the project
    Then the result reports 0 files re-extracted
    And all existing entities in the graph are preserved unchanged

  Scenario: Incremental re-index extracts only modified files
    Given a project that has been fully indexed with mtime-stamped File nodes
    And one source file has been modified since the last index
    When I run an incremental ast_index on the project
    Then only the modified file is re-extracted
    And entities from unchanged files are reused from the existing graph
    And the graph contains the updated entities for the modified file

  Scenario: Incremental re-index removes deleted file entities
    Given a project that has been fully indexed with mtime-stamped File nodes
    And one source file has been deleted from the filesystem
    When I run an incremental ast_index on the project
    Then the deleted file's File node is no longer in the graph
    And the deleted file's Function and Type nodes are no longer in the graph

  Scenario: Incremental re-index adds new file entities
    Given a project that has been fully indexed with mtime-stamped File nodes
    And a new source file has been added to the project
    When I run an incremental ast_index on the project
    Then the new file's entities are extracted and added to the graph
    And entities from previously indexed files are preserved

  Scenario: Incremental falls back to full extraction on empty graph
    Given a project directory with source files
    And the AST graph is empty with no prior index
    When I run an incremental ast_index on the project
    Then a full extraction is performed for all source files
    And all entities are loaded into the graph with mtime stamps

  Scenario: Incremental falls back to full extraction when more than 50% of files changed
    Given a project that has been fully indexed with mtime-stamped File nodes
    And more than 50% of the source files have been modified since the last index
    When I run an incremental ast_index on the project
    Then a full extraction is performed instead of incremental
