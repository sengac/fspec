@KGRAPH-021
Feature: Learnings Extraction Pipeline — Session Boundary Analysis
  """
  Extraction pipeline in rust/napi/src/graph/learnings_extraction.rs.
  Uses the Learnings graph via registry. LLM prompt template uses Residue methodology structure.
  Entities loaded via GraphDatabase::load_entities batch API.
  Tests use REAL nanograph databases (tempdir), REAL fixture JSON for LLM responses,
  REAL dispatch functions for round-trip verification. NO mocks.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   [6] extract_learnings_from_text() must be the extraction path called at compaction — not the keyword-matching extract_structural_learnings_from_dag()
  #   [7] Extraction uses the Residue methodology prompt structure: Learnings, Explorations, Constraints
  #   [8] Extraction produces entities matching the Learnings graph schema: Learning, Exploration, Convention, Decision, CodePattern nodes plus Discovered/Eliminates edges
  #   [9] Entities must be loaded into the real Learnings graph and be queryable via learnings_dispatch functions
  #   [10] Volume constraint: 1-20 entities per extraction. Truncates at 20 max
  #   [11] When LLM extraction is unavailable, fail gracefully — no entities written, no panic
  #   [12] extract_learnings_from_text accepts pre-computed LLM response text (not an LLM client)
  #   [13] The keyword-matching extract_structural_learnings_from_dag() must be removed
  #
  # EXAMPLES:
  #   [3] Realistic DAG summary → 2 learnings + 1 exploration + 1 constraint → 4 entities with correct fields
  #   [4] Entities loaded → learnings_search query → matching nodes returned
  #   [5] Decision entities loaded → learnings_decisions with domain filter → filtered results
  #   [6] No LLM response → Err result → database remains empty
  #   [7] 25 entities in response → truncated to 20
  #   [8] Invalid categories in response → invalid entities skipped
  #   [9] Malformed JSON response → Err with parse message
  #   [10] Entities loaded → learnings_stats → node counts > 0
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to have learnings extracted from my session summaries into the Learnings graph at compaction boundaries
    So that accumulated knowledge persists across sessions and can be queried via GraphSearch

  Scenario: Extract learnings, explorations, and constraints from a DAG summary
    Given a compaction DAG summary text describing nanograph query syntax work
    And a realistic LLM response JSON containing 2 learnings, 1 exploration, and 1 constraint
    When the extraction pipeline processes the summary with the LLM response
    Then the result should contain 4 entities total
    And there should be 2 Learning nodes with category not equal to "constraint"
    And there should be 1 Learning node with category "constraint"
    And there should be 1 Exploration node with outcome "failure"
    And each Learning node should have slug, title, content, category, confidence, firstSeen, lastSeen, and mentionCount
    And each Exploration node should have slug, title, strategy, outcome, and createdAt

  Scenario: Extracted entities are queryable via learnings_search after loading into database
    Given a Learnings graph database initialized in a temp directory
    And entities extracted from a realistic LLM response
    When the entities are loaded into the Learnings graph via load_entities
    And dispatch_learnings_search is called with a keyword matching one of the learnings
    Then the search results should contain the matching Learning node with correct slug and title
    And the search results should not contain non-matching entities

  Scenario: Decision entities are queryable via learnings_decisions with domain filter
    Given a Learnings graph database initialized in a temp directory
    And extracted entities include Decision nodes with different domains
    When the Decision entities are loaded into the Learnings graph
    And dispatch_learnings_decisions is called with domain "architecture"
    Then only Decision nodes with domain "architecture" should be returned

  Scenario: Learnings stats reflect loaded entities
    Given a Learnings graph database initialized in a temp directory
    And entities extracted from a realistic LLM response containing Learning and Exploration nodes
    When the entities are loaded into the Learnings graph
    And dispatch_learnings_stats is called
    Then the stats should show Learning count greater than 0
    And the stats should show Exploration count greater than 0

  Scenario: Graceful failure when LLM response is unavailable
    Given a DAG summary text to process
    And no LLM response is available
    When the extraction pipeline attempts to process the text
    Then the result should be an Err with a descriptive message
    And a subsequently initialized Learnings graph database should remain empty

  Scenario: Volume constraint truncates at 20 entities
    Given an LLM response JSON containing 25 valid Learning entities
    When the extraction pipeline processes the response
    Then exactly 20 entities should be returned
    And no error should be raised

  Scenario: Invalid entity categories are skipped
    Given an LLM response JSON containing 3 learnings where 1 has category "foo"
    When the extraction pipeline processes the response
    Then only 2 Learning nodes should be returned
    And the entity with invalid category "foo" should be skipped

  Scenario: Malformed JSON response returns parse error
    Given an LLM response containing invalid JSON
    When the extraction pipeline attempts to process the response
    Then the result should be an Err containing a parse error message
    And no entities should be produced
