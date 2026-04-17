@KGRAPH-007
Feature: GraphSearch Query Implementations
  """
  Pure Rust module at codelet/napi/src/graph/queries.rs. Each GraphSearch action maps to formatting and filtering functions that build results as JSON. Handler in graph_search_handler.rs dispatches to these functions. Depends on KGRAPH-003 handler map for action dispatch.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. search action performs text-based concept lookup by name/summary with fuzzy matching
  #   2. neighbors action returns concepts within N hops via RelatesTo edges (default depth=2, max depth=3)
  #   3. related action returns direct RelatesTo edges for a concept, filterable by min_strength
  #   4. decisions action lists Decision nodes filterable by domain and status
  #   5. stats action returns counts per node type and edge type
  #   6. All query results are returned as JSON strings for LLM consumption
  #
  # EXAMPLES:
  #   1. search for 'JWT' returns concept nodes ranked by mentionCount
  #   2. neighbors of 'jwt-authentication' at depth 2 returns related concepts
  #   3. related concepts with min_strength 0.5 returns only edges above threshold
  #   4. decisions filtered by domain 'architecture' returns only architecture decisions
  #   5. stats action returns counts per node and edge type
  #
  # ========================================
  Background: User Story
    As an agent developer
    I want to query the knowledge graph using GraphSearch actions (search, neighbors, related, decisions, stats)
    So that LLMs and tools can retrieve relevant knowledge from the graph during conversations

  Scenario: Search action finds concepts by text query
    Given a knowledge graph containing concept nodes for "JWT Authentication" and "Session Management"
    When the search action is invoked with query "JWT"
    Then concept nodes matching the query are returned as JSON
    And results include slug, name, category, and mentionCount fields

  Scenario: Neighbors action returns concepts within hop distance
    Given a knowledge graph with "jwt-authentication" related to "session-management" related to "redis-cache"
    When the neighbors action is invoked for "jwt-authentication" with depth 2
    Then "session-management" is returned at depth 1
    And "redis-cache" is returned at depth 2

  Scenario: Related action filters by minimum strength
    Given a knowledge graph with RelatesTo edges at various strengths
    When the related action is invoked for "jwt-authentication" with min_strength 0.5
    Then only edges with strength greater than or equal to 0.5 are returned
    And edges below the threshold are excluded

  Scenario: Decisions action filters by domain
    Given a knowledge graph with decisions across multiple domains
    When the decisions action is invoked with domain filter "architecture"
    Then only decisions with domain "architecture" are returned
    And results are sorted by decidedAt descending

  Scenario: Stats action returns type counts
    Given a knowledge graph with nodes and edges of various types
    When the stats action is invoked
    Then the result includes counts for each node type
    And the result includes counts for each edge type
    And the result is formatted as a JSON object

  Scenario: History action returns turn provenance for a concept
    Given a knowledge graph with Turn nodes linked to concepts via Mentions edges
    When the history action is invoked for concept 'redis'
    Then Turn nodes that mention the concept are returned with session and turn index

  Scenario: Index action flushes pending entities and returns status
    Given pending entities exist in the entity pipeline queue
    When the index action is invoked
    Then pending entities are flushed to the graph database
    Then the result indicates the indexing status as JSON
