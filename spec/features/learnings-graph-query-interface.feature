@KGRAPH-023
Feature: Learnings Graph Query Interface

  """
  Learnings dispatch in codelet/napi/src/graph/learnings_dispatch.rs. Query source in codelet/napi/schemas/learnings-queries.gq (loaded via include_str!). New action variants: LearningsSearch, LearningsDecisions, LearningsStats, LearningsRelated added to GraphSearchAction enum. Handler routes Learnings-prefixed actions to learnings_dispatch functions via registry::get_graph(LEARNINGS_GRAPH).
  Follows the exact same architectural pattern as KGRAPH-019 (ast_dispatch.rs) — separate dispatch file, separate query file, registry-based DB access, JSON string returns.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. New Learnings action types must be added to the GraphSearchAction enum in types.rs following the same serde tag pattern as AST actions
  #   2. Learnings queries must use the Learnings graph database via registry::get_graph(LEARNINGS_GRAPH), NOT the agent-memory or AST graph
  #   3. Learnings dispatch functions must be in a separate file (learnings_dispatch.rs), following the same pattern as ast_dispatch.rs
  #   4. Must support at minimum: search learnings by text/category, get learnings stats, query decisions by domain/status, and find related learnings by topic
  #   5. Nanograph PG queries for Learnings data must be in a separate file (learnings-queries.gq) bundled via include_str!
  #   6. The existing graph_search_handler.rs dispatch_action match must route Learnings-prefixed actions to learnings_dispatch — transparent to the caller
  #   7. Each dispatch function must take a &GraphDatabase parameter and return a String (JSON formatted), following the same signature pattern as ast_dispatch functions
  #
  # EXAMPLES:
  #   1. Search for a Learning by text 'nanograph queries require explicit edge type names' and get back the Learning node with category, confidence, domain, and session origin
  #   2. Query decisions filtered by domain 'architecture' and status 'active' to see all active architectural decisions with rationale and alternatives
  #   3. Get Learnings graph statistics showing counts of Learning, Exploration, Convention, Decision, CodePattern nodes and relationship edge counts
  #   4. Find learnings related to 'error handling' topic and get back Learning nodes with strength and relation type, sorted by relevance
  #
  # ========================================

  Background: User Story
    Given an AI agent wants to query accumulated learnings from the Learnings graph

  Scenario: Search for a learning by text using LearningsSearch action
    Given the Learnings graph contains Learning nodes with various categories and domains
    When I search learnings with query "nanograph queries require explicit edge type names"
    Then I should receive matching Learning nodes with slug, category, confidence, domain, and session origin

  Scenario: Query decisions filtered by domain and status using LearningsDecisions action
    Given the Learnings graph contains Decision nodes with domain, status, rationale, and alternatives
    When I query decisions with domain "architecture" and status "active"
    Then I should receive only active architectural decisions with their rationale and alternatives

  Scenario: Get Learnings graph statistics using LearningsStats action
    Given the Learnings graph contains nodes of type Learning, Exploration, Convention, Decision, and CodePattern
    When I request Learnings graph statistics
    Then I should receive node counts per type and total edge counts

  Scenario: Find related learnings by topic using LearningsRelated action
    Given the Learnings graph contains Learning nodes connected by RelatesTo edges with strength values
    When I search for learnings related to topic "error handling"
    Then I should receive related Learning nodes sorted by relevance with strength and relation type
