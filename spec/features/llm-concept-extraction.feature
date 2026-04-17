@KGRAPH-005
Feature: LLM-Based Concept Extraction Pipeline
  """
  Extraction prompt sends batches to an LLM. Pipeline is pure Rust: build prompt, parse JSON response, validate entities, return Vec<GraphEntity>. No DB writes in this card — outputs feed into KGRAPH-006 merge/upsert.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The extraction prompt must instruct the LLM to extract three entity types: Concepts (slug, name, category, summary, confidence), Decisions (slug, title, rationale, domain, confidence), and Relations (from, to, type, strength)
  #   2. Conversation turns must be batched (5-10 turns per LLM call) with each turn truncated to 2000 characters
  #   3. Only user and assistant message turns are sent to the LLM — tool results are skipped (handled by structural extractors in KGRAPH-004)
  #   4. The LLM response must be parsed as JSON with validation: reject empty slugs/names, reject self-referencing relations (from==to), reject strength outside 0.0-1.0, skip malformed entries without failing the batch
  #   5. The extraction pipeline outputs Vec<GraphEntity> (the same type used by structural extractors) so it can feed into the same merge/upsert path (KGRAPH-006)
  #   6. Concept categories must be from the schema enum: architecture, convention, decision, dependency, domain_term, error_class, feature, library, pattern, person, platform, process, technology, tool
  #   7. Decision domains must be from the schema enum: architecture, convention, dependency, deployment, design, implementation, process, testing
  #   8. Relation types must be from the schema enum: causes, composes, conflicts_with, depends_on, extends, implements, similar_to, supersedes, uses
  #
  # EXAMPLES:
  #   1. A batch of 5 conversation turns about 'JWT authentication' produces Concept nodes for 'jwt-authentication' (technology) and 'session-management' (pattern), a Decision node for 'use-jwt-over-sessions', and a RelatesTo edge between them
  #   2. LLM returns malformed JSON with a concept missing its slug field — that concept is skipped but other valid entities in the same response are still returned
  #   3. LLM returns a relation where from == to (self-reference) — that relation is rejected during validation
  #   4. A batch of only tool result turns (no user/assistant content) is skipped without invoking the LLM
  #   5. Conversation turns longer than 2000 chars are truncated before being included in the extraction prompt
  #   6. LLM returns a concept with category 'foobar' not in the schema enum — that concept is rejected during validation
  #
  # ========================================
  Background: User Story
    As an agent developer
    I want to have conversation content automatically analyzed by an LLM to extract concepts, decisions, and relations
    So that the knowledge graph is enriched with semantic understanding beyond structural tool-call data

  Scenario: Valid conversation batch produces concept, decision, and relation entities
    Given a batch of 5 conversation turns discussing JWT authentication
    When the LLM extraction pipeline processes the batch
    Then Concept nodes are produced with valid slugs, categories, and confidence levels
    And Decision nodes are produced with valid domains and rationale
    And RelatesTo edges connect related concepts with valid types and strength values
    And all entities are returned as Vec<GraphEntity> compatible with the merge/upsert pipeline

  Scenario: Malformed concept entry is skipped without failing the batch
    Given an LLM response containing a concept with a missing slug field and two valid concepts
    When the response is parsed and validated
    Then the malformed concept is skipped
    And the two valid concepts are returned as GraphEntity nodes

  Scenario: Self-referencing relation is rejected
    Given an LLM response containing a relation where from and to slugs are identical
    When the response is parsed and validated
    Then the self-referencing relation is rejected
    And other valid entities in the response are still returned

  Scenario: Tool-result-only batch is skipped without LLM invocation
    Given a batch of turns that are all tool results with no user or assistant messages
    When the batch is submitted to the extraction pipeline
    Then the pipeline returns an empty list without invoking the LLM

  Scenario: Long conversation turns are truncated before extraction
    Given a batch containing a user turn with 5000 characters of content
    When the extraction prompt is built from the batch
    Then the turn content in the prompt is truncated to 2000 characters

  Scenario: Invalid enum values in entities are rejected
    Given an LLM response with a concept having category 'foobar' and a decision having domain 'invalid-domain'
    When the response is parsed and validated
    Then both entities with invalid enum values are rejected
    And entities with valid enum values from the same response are still returned
