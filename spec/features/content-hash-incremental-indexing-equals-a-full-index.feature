@done
@rust
@code-analysis
@parser
@KGRAPH-076
Feature: Content-hash incremental indexing that equals a full index
  """
  Fact cache: a process-global map, per canonical root, from relative path to (contentHash, Arc<FileFacts>), held with the root's registry slot and emptied by GraphWrite::reset. Kept in memory rather than on disk because facts depend on the extractor build as well as the bytes, so only an in-process cache is exact without a build identity; a fresh process parses every file once (fact extraction of this repo is about 0.44 s of a full index).
  Write record: after a successful Overwrite load ast_index writes <graph>/fspec-index.sha256, the SHA-256 of the loaded entities' JSONL. GraphDatabase::overwrite, the one load call, deletes the record before loading, so a bundle import or any other write invalidates it. An incremental index compares the hash of the entities it built with the record and writes only when they differ; the entities cover every input (sources, resolve manifests, dependency manifests), so the comparison is exact without listing inputs.
  Removed: ast_pipeline/incremental.rs (collect_file_mtimes, read_stored_mtimes, partition_changed_files, stamp_file_mtimes), the File.lastModified property (replaced by contentHash: String), the second walk, the export_all_entities read of stored mtimes and the files_re_extracted field. The schema change makes an older code graph fail with the 'Schema has changed ... reset: true' error once (graph-database-reset.feature). The GraphSearch 'incremental' and 'reset' descriptions are rewritten to the new behaviour.
  Exactness test: an ordered edit script over a fixture tree; after each edit the incremental graph's sorted export JSONL equals that of a full index of a copy of the tree (contentHash and every other property compared). Deterministic per TEST-042: fixed edits, no randomness, no timing.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An incremental index produces exactly the graph a full index of the same tree produces: the sorted exports of the two graphs are identical, every property included (product owner, 2026-11-07; supersedes KGRAPH-065 rule 5)
  #   2. An index walks the tree once and reads each source file once; the SHA-256 of those bytes is the file's contentHash, stored on its File node, and the file's facts come from the same bytes. There is no modification-time logic
  #   3. The facts of a file are cached in the process by root, path and contentHash. An incremental index parses only the files whose path and contentHash are not cached and reuses the cached facts of the others; a full index parses every file. A process starts with an empty cache, and a reset empties the root's cache
  #   4. Every index runs the global resolve pass over the facts of the whole tree and adds the dependencies from the manifests, then builds the graph's entities
  #   5. ast_index records the SHA-256 of the entities it loaded beside the graph. An incremental index whose entities have the recorded hash writes nothing; every other write to the graph discards the record, so the next index writes. There is no 50% rule and no other fallback
  #   6. The index result names the root and reports files_parsed, files_reused, files_failed [{path, error}], files_skipped [{path, reason}], unresolved and whether the graph was written
  #
  # EXAMPLES:
  #   1. a.py imports helper from b.py, which has no helper; after an index, def helper is added to b.py and an incremental index runs: a's call to helper is a Calls edge and the graph equals a full index of the tree
  #   2. A fixed sequence of 12 edits to a Rust crate, a Python package and a TypeScript barrel (add, remove, rename and move definitions, change imports, add and delete files, change a manifest): after every edit the incrementally indexed graph equals a full index of a copy of the tree
  #   3. After a full index, an incremental index in the same process with nothing changed parses 0 files, reuses 3 and writes nothing
  #   4. One of 3 files is edited: the incremental index parses 1 file and reuses 2
  #   5. A file's text changes while its modification time is set back to the old one: the incremental index parses it and the graph holds the new definition
  #   6. A dependency is added to package.json and no source file changes: the incremental index writes the graph and it holds the new Dependency
  #   7. A bundle is imported into the graph after an index: the next incremental index writes the graph, which then equals a full index of the tree
  #   8. A full index (incremental false) parses every file, also when their facts are cached, and writes the graph
  #   9. After the root's cache is emptied by a reset, an incremental index parses every file
  #   10. A source file that is not UTF-8 is listed in files_failed with its path and error, a 2 MiB source file in files_skipped with reason too_large, and the result names the root
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch
    I want to re-index a project incrementally after editing it
    So that the code graph is exactly what a full index of the tree would build, and an unchanged file is never parsed twice in one process

  Scenario: A definition added to the target of an unchanged file's import is called after an incremental index
    Given a Python package where a.py imports helper from b.py and calls it, and b.py has no helper
    And the package has been indexed
    When def helper is added to b.py and an incremental ast_index runs
    Then a Calls edge leads from a.py's caller to b.py's helper
    And the graph equals a full index of a copy of the tree

  Scenario: After every edit of a fixed sequence the incremental graph equals a full index
    Given a project holding a Rust crate, a Python package and a TypeScript barrel has been indexed
    When each of 12 fixed edits is applied in turn, with an incremental ast_index after each
    Then after every edit the graph's sorted export equals that of a full index of a copy of the tree

  Scenario: An incremental index with nothing changed parses nothing and writes nothing
    Given a project of 3 source files has been fully indexed
    When an incremental ast_index runs in the same process with nothing changed
    Then it parses 0 files and reuses 3
    And it reports that the graph was not written

  Scenario: An incremental index parses only the edited file
    Given a project of 3 source files has been fully indexed
    When one file is edited and an incremental ast_index runs
    Then it parses 1 file and reuses 2
    And the graph holds the edited file's new definition

  Scenario: A content change that keeps the modification time is indexed
    Given a project of 3 source files has been fully indexed
    When a file's text changes, its modification time is set back to the old one, and an incremental ast_index runs
    Then it parses that file
    And the graph holds the new definition, and the File node's contentHash is the SHA-256 of the new text

  Scenario: A manifest change alone makes an incremental index write the graph
    Given a project with a package.json and source files has been indexed
    When a dependency is added to package.json and an incremental ast_index runs
    Then the graph is written
    And it holds the new Dependency

  Scenario: Another write to the graph makes the next incremental index write
    Given a project has been indexed
    And a bundle exported from another project's graph has been imported into its graph
    When an incremental ast_index runs with no file changed
    Then the graph is written
    And the graph equals a full index of a copy of the tree

  Scenario: A full index parses every file
    Given a project of 3 source files has been indexed
    When a full ast_index runs with nothing changed
    Then it parses 3 files and reuses 0
    And the graph is written

  Scenario: A reset empties the root's fact cache
    Given a project of 3 source files has been indexed
    When an incremental ast_index runs with reset
    Then it parses 3 files and reuses 0

  Scenario: Files that are not parsed are reported
    Given a project holding a source file that is not UTF-8, a 2 MiB source file and one valid source file
    When ast_index runs for the project
    Then files_failed lists the file that is not UTF-8 with its error
    And files_skipped lists the 2 MiB file with reason too_large
    And it parses 1 file and the result names the project's canonical root
