@done
@KGRAPH-026
Feature: AST extraction pipeline creates duplicate File entities for import targets — @unique constraint violation on File.path
  """
  Fix is in walk_and_extract() (ast_pipeline/mod.rs). After collecting all entities from all files, deduplicate Node entities by (node_type, slug). For File nodes, prefer the node with more properties (full > stub). Use a HashMap<(node_type, slug), index> to track seen nodes.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Import resolution in ast_ts_extractor.rs creates stub File nodes for import targets that may duplicate File nodes created by the file walker
  #   2. The entity list must be deduplicated by (node_type, path) before JSONL serialization, preferring full nodes over stubs
  #   3. Deduplication must happen in walk_and_extract() after all files are processed, not in individual extractors
  #   4. Full File nodes (with language, lineCount, isTest) must always win over stub File nodes (with only slug, path) during deduplication
  #   5. Edges must NOT be deduplicated — only Node entities with the same (node_type, slug) pair
  #   6. Deduplication must preserve all edges — even if two File nodes are merged, both Imports edges from different source files must be kept
  #
  # EXAMPLES:
  #   1. bridge/telegram-endpoint.ts imports ./telegram-slash-commands → walker also walks bridge/telegram-slash-commands.ts → two File nodes with path='bridge/telegram-slash-commands.ts' → @unique constraint violation
  #   2. File A imports File B which is also in the project → after dedup only one File B node exists with full properties → graph loads successfully
  #   3. Import target resolves to a file outside the project → stub File node is kept because no full node exists → no dedup collision
  #   4. Multiple files all import the same target → only one stub File node for the target in the output, not N duplicates
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to index the codebase without duplicate entity failures
    So that the AST graph is fully populated and queryable

  Scenario: Deduplicate File nodes when import target is also walked directly
    Given a project with file "src/index.ts" that imports from "./utils"
    And a project file "src/utils.ts" that is also walked by the file walker
    When the extraction pipeline processes the project directory
    Then only one File node should exist for "src/utils.ts"
    And that File node should have the full properties including language, lineCount, and isTest
    And the Imports edge from "src/index.ts" to "src/utils.ts" should be preserved

  Scenario: Preserve stub File nodes for external import targets
    Given a project with file "src/app.ts" that imports from "express"
    When the extraction pipeline processes the project directory
    Then a stub File node should exist for the external import target
    And no unique constraint violation should occur

  Scenario: Multiple files importing same target produce single File node
    Given a project with files "src/a.ts", "src/b.ts", and "src/c.ts" all importing from "./shared"
    And a project file "src/shared.ts" that is also walked by the file walker
    When the extraction pipeline processes the project directory
    Then only one File node should exist for "src/shared.ts"
    And all three Imports edges should be preserved
    And the graph should load successfully without constraint violations

  Scenario: Full codebase indexing completes without errors
    Given a project directory with TypeScript files containing cross-imports
    When the AST index operation runs via walk_and_extract
    And the entities are loaded into the graph database
    Then the load operation should succeed with no unique constraint violations
    And the graph should contain File, Function, and Imports data
