@KGRAPH-021
Feature: Learnings Extraction Pipeline — Session Boundary Analysis

  """
  Extraction pipeline in codelet/napi/src/graph/learnings_extraction.rs with shared JSON parsing in llm_response_parser.rs.
  Uses the Learnings graph via registry. LLM prompt template uses Residue methodology structure.
  Entities loaded via GraphDatabase::load_entities batch API.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Extraction must use the Learnings graph database (via registry::LEARNINGS_GRAPH), not the agent-memory graph
  #   2. Extraction triggers at session boundaries only: compaction DAG summary, work unit completion, explicit index command — never per-turn
  #   3. LLM prompt must follow the Residue methodology structure: Learnings, Explorations, Reformulations, Constraints
  #   4. Must produce 5-20 entities per extraction (not hundreds) — volume-constrained to prevent graph bloat
  #   5. Extraction pipeline must live in its own file (learnings_extraction.rs), separate from other graph modules
  #   6. Entities must map to the Learnings graph schema nodes: Learning, Exploration, Constraint, Pattern, CodeReference
  #
  # EXAMPLES:
  #   1. After a session compaction produces a DAG summary, the extraction pipeline processes the summary text and produces Learning nodes like 'nanograph queries require explicit edge type names' with category 'technology', confidence 0.9
  #   2. When a work unit moves to done, the pipeline extracts from the session's conversation history and produces Exploration nodes (approaches tried) and Constraint nodes (hard facts discovered)
  #   3. When the LLM is unavailable or returns an error, extraction fails gracefully with a warning log, and no entities are written — the graph remains consistent
  #
  # ========================================

  Background: User Story
    As an AI agent
    I want to extract accumulated learnings from session boundaries using LLM analysis and the Residue methodology
    So that the Learnings Graph accumulates high-value knowledge without per-turn overhead or excessive disk usage

  Scenario: Extract learnings from a compaction DAG summary
    Given a compaction DAG summary text describing work done in a session
    And the Learnings graph database is initialized
    When the learnings extraction pipeline processes the summary text
    Then Learning nodes should be created in the Learnings graph database
    And each Learning node should have a title, category, confidence, and content
    And the extraction should produce between 1 and 20 entities

  Scenario: Extract explorations and constraints from session history
    Given a session conversation history describing multiple approaches tried
    And the Learnings graph database is initialized
    When the learnings extraction pipeline processes the session text
    Then Exploration nodes should be created for approaches tried with outcome and status
    And Constraint nodes should be created for hard facts discovered

  Scenario: Graceful failure when LLM is unavailable
    Given the Learnings graph database is initialized
    And the LLM provider returns an error
    When the learnings extraction pipeline attempts to process text
    Then no entities should be written to the Learnings graph
    And the pipeline should return an error result without panicking
