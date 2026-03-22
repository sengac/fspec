@KGRAPH-024
Feature: Deprecate Old Graph & Migrate Useful Data

  """
  Delete 18 purely-old files (~4200 lines): schemas/agent-memory.pg, schemas/graph-queries.gq, entity_pipeline.rs, extractors.rs, merge.rs, watermark.rs, indexing.rs, session_scanner.rs, compaction.rs, dispatch.rs, queries.rs, deepsearch_integration.rs, llm_extraction.rs, llm_validation.rs, llm_caller.rs, tests.rs, graph_lifecycle_test.rs, tool tests.rs. Surgically update 7 shared files: database.rs, registry.rs, dispatch_helpers.rs, llm_response_parser.rs, tool types.rs/mod.rs/handler.rs. Update graph_search_handler.rs to remove old dispatch routing. Update DeepSearch to use Learnings graph context.
  The graph module after migration should only export: database (GraphDatabase), registry (get_graph, AST_CODE_GRAPH, LEARNINGS_GRAPH), ast_pipeline (extractors), ast_dispatch (AstSearch/AstNeighbors/AstStats), learnings_extraction (extract_learnings_from_text), learnings_dispatch (LearningsSearch/LearningsDecisions/LearningsStats/LearningsRelated), dispatch_helpers (shared utilities), llm_response_parser (shared JSON parsing).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All exclusively-old files must be deleted: agent-memory.pg, graph-queries.gq, entity_pipeline.rs, extractors.rs, merge.rs, watermark.rs, indexing.rs, session_scanner.rs, compaction.rs, dispatch.rs, queries.rs, deepsearch_integration.rs, llm_extraction.rs, llm_validation.rs, llm_caller.rs, tests.rs (~4200 lines)
  #   2. Shared infrastructure files (database.rs, registry.rs, dispatch_helpers.rs, tool types.rs, tool mod.rs, tool handler.rs, llm_response_parser.rs) must be surgically updated to remove agent-memory references while preserving AST and Learnings graph support
  #   3. GraphSearchAction enum must remove old 8 variants (Search, Neighbors, Path, Related, Decisions, History, Stats, Index) and keep only AstSearch/AstNeighbors/AstStats and LearningsSearch/LearningsDecisions/LearningsStats/LearningsRelated
  #   4. The entity_pipeline.rs global PENDING_ENTITIES queue and per-tool-call entity interception must be completely removed — no more real-time interception of Write/Edit/Fspec tool calls
  #   5. DeepSearch graph context injection (deepsearch_integration.rs) must be updated to use the Learnings graph instead of the old agent-memory graph for concept/decision context
  #   6. The graph_search_handler.rs dispatch must only route to ast_dispatch and learnings_dispatch — no more routing to old dispatch.rs functions
  #   7. The graph module mod.rs must be updated to remove old public API functions (ensure_graph_db, graph_db_stats for agent-memory) and old module declarations, keeping only database, registry, ast_pipeline, ast_dispatch, learnings_extraction, learnings_dispatch, dispatch_helpers, and llm_response_parser
  #   8. Old test files (graph_lifecycle_test.rs and any other tests that only exercise agent-memory functionality) must be deleted; tool handler tests must be updated to only test AST/Learnings actions
  #   9. All existing AST and Learnings tests must continue passing after migration — zero regressions
  #
  # EXAMPLES:
  #   1. After migration, GraphSearchAction enum only has AST* and Learnings* variants (7 total) — cargo build succeeds, no old action types compile
  #   2. After migration, registry.rs has no AGENT_MEMORY_GRAPH constant — only AST_CODE_GRAPH and LEARNINGS_GRAPH constants exist and are used
  #   3. After migration, DeepSearch build_graph_context() uses learnings_dispatch functions to get context from the Learnings graph instead of the old agent-memory Concept nodes
  #   4. After migration, graph module mod.rs has no references to entity_pipeline, extractors, merge, watermark, indexing, session_scanner, compaction, dispatch (old), queries, deepsearch_integration, llm_extraction, llm_validation, or llm_caller modules
  #   5. All 15 existing tests pass (4 learnings_query + 3 learnings_extraction + 3 ast_query + 5 ast_data_model) with zero regressions after removing old infrastructure
  #
  # ========================================

  Background: User Story
    As an AI agent
    I want to have the old monolithic graph infrastructure deprecated and removed
    So that the system uses only the lean dual-graph architecture (AST + Learnings) without the disk-heavy Turn/Session provenance model that consumed 7.6GB

  Scenario: GraphSearchAction enum only contains AST and Learnings variants after migration
    Given the old monolithic graph infrastructure has been removed
    When the GraphSearchAction enum is compiled
    Then it should only contain AST-prefixed and Learnings-prefixed variants
    And the old agent-memory variants Search, Neighbors, Path, Related, Decisions, History, Stats, and Index should not exist
    And the crate should build successfully with no compilation errors

  Scenario: Graph registry only contains AST and Learnings graph instances
    Given the old monolithic graph infrastructure has been removed
    When the graph registry is initialized
    Then it should only contain AST_CODE_GRAPH and LEARNINGS_GRAPH constants
    And the AGENT_MEMORY_GRAPH constant should not exist
    And get_graph should work for both AST and Learnings graphs

  Scenario: DeepSearch uses Learnings graph context instead of agent-memory
    Given the old monolithic graph infrastructure has been removed
    And the Learnings graph contains accumulated learnings and decisions
    When DeepSearch builds graph context for a sub-agent system prompt
    Then it should query the Learnings graph for relevant decisions and learnings
    And it should not reference the old agent-memory Concept nodes

  Scenario: Graph module exports only dual-graph infrastructure
    Given the old monolithic graph infrastructure has been removed
    When the graph module is compiled
    Then it should export database, registry, ast_pipeline, ast_dispatch, learnings_extraction, learnings_dispatch, learnings_context, dispatch_helpers, graph_entities, and llm_response_parser modules
    And it should not export entity_pipeline, extractors, merge, watermark, indexing, session_scanner, compaction, or old dispatch and queries modules

  Scenario: All existing AST and Learnings tests pass after migration
    Given the old monolithic graph infrastructure has been removed
    When the full test suite is executed
    Then all 4 learnings query interface tests should pass
    And all 3 learnings extraction tests should pass
    And all 3 AST query interface tests should pass
    And all 5 AST graph data model tests should pass
    And all 3 AST dependency population tests should pass
