@done
@KGRAPH-065
@parser
@code-analysis
Feature: Incremental Re-indexing
  """
  Change detection is by content hash (KGRAPH-076, product owner 2026-11-07): an index reads each source file once, the SHA-256 of its bytes is the File node's contentHash, and the file's facts are cached per canonical root by path and contentHash.
  An incremental index parses only the files that are not in the cache, re-runs the global resolve pass over the whole tree and builds exactly the entities a full index builds; it writes nothing when they hash to the record of the last load. There is no modification-time logic and no 50% fallback.
  No file watcher exists: only an ast_index call indexes (KGRAPH-078).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Change detection is by content hash, not modification time: an incremental index parses only the files whose path and contentHash are not in the root's fact cache, re-runs the global resolve pass and builds exactly the graph a full index builds; there is no 50% fallback (KGRAPH-076, product owner 2026-11-07)
  #   2. The full overwrite-load strategy is preserved — every index that writes replaces the whole graph with one Overwrite load, so stale entities are always cleaned up
  #   3. Dependencies (Cargo.toml, package.json, etc.) are always re-extracted, since they are fast and small
  #
  # EXAMPLES:
  #   1. After a full index, an incremental index with nothing changed parses 0 files and writes nothing
  #   2. After one file is modified, an incremental index parses only that file and reuses the cached facts of the others
  #   3. After deleting a file, incremental re-index removes the deleted file's entities (File, Function, Type nodes and edges)
  #   4. After adding a new file, incremental re-index extracts it as a new file and includes it alongside reused entities
  #   5. An incremental index of a project with no stored graph parses and writes every file
  #   6. When incremental is false or omitted, every file is parsed and the graph is written
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want my AST graph to be incrementally re-indexed when only a few files have changed
    So that I avoid the expensive full re-index on large codebases and get fast graph updates

  Scenario: Incremental re-index detects no changes and skips extraction
    Given a project that has been fully indexed
    And no source files have been modified since the last index
    When I run an incremental ast_index on the project
    Then the result reports 0 files parsed
    And all existing entities in the graph are preserved unchanged

  Scenario: Incremental re-index extracts only modified files
    Given a project that has been fully indexed
    And one source file has been modified since the last index
    When I run an incremental ast_index on the project
    Then only the modified file is parsed
    And the facts of the unchanged files are reused
    And the graph contains the updated entities for the modified file

  Scenario: Incremental re-index removes deleted file entities
    Given a project that has been fully indexed
    And one source file has been deleted from the filesystem
    When I run an incremental ast_index on the project
    Then the deleted file's File node is no longer in the graph
    And the deleted file's Function and Type nodes are no longer in the graph

  Scenario: Incremental re-index adds new file entities
    Given a project that has been fully indexed
    And a new source file has been added to the project
    When I run an incremental ast_index on the project
    Then the new file's entities are extracted and added to the graph
    And entities from previously indexed files are preserved

  Scenario: An incremental index of a project with no stored graph parses and writes every file
    Given a project directory with source files
    And the AST graph is empty with no prior index
    When I run an incremental ast_index on the project
    Then every source file is parsed
    And all entities are loaded into the graph
