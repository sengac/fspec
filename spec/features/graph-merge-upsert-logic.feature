@KGRAPH-006
Feature: Graph Merge & Upsert Logic
  """
  Pure Rust module at codelet/napi/src/graph/merge.rs. Converts Vec<GraphEntity> to JSONL, loads via nanograph merge mode with @key. For increment/min/max merge semantics, implements read-before-write pattern. Watermark in ~/.fspec/graph/index-state.json written atomically.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Vec<GraphEntity> from extractors/LLM pipeline must be converted to JSONL format for nanograph load
  #   2. All loads use merge mode: if a node with same @key slug exists, update properties; if not, insert
  #   3. mentionCount and coOccurrenceCount use increment semantics (read-before-write), not overwrite
  #   4. firstSeen keeps earliest timestamp, lastSeen keeps latest timestamp on merge
  #   5. confidence is promoted (low→medium→high) never demoted on merge
  #   6. Watermark state stored in index-state.json tracks per-session lastIndexedTurn for incremental re-indexing
  #   7. index-state.json written atomically via temp file + rename after each successful batch
  #   8. RelatesTo edge strength recalculated as min(1.0, log2(coOccurrenceCount + 1) / 10.0) on each co-occurrence
  #
  # EXAMPLES:
  #   1. 2 Concept GraphEntity nodes loaded into nanograph → 2 Concept rows visible in DB with correct slug/name/category
  #   2. Same Concept slug loaded twice with different mentionCount → mentionCount is summed (not overwritten), firstSeen preserved, lastSeen updated
  #   3. Concept loaded with confidence=medium, then same slug loaded with confidence=high → confidence promoted to high
  #   4. Concept loaded with confidence=high, then same slug loaded with confidence=low → confidence stays high (no demotion)
  #   5. After successful upsert, index-state.json updated with session watermark and lastRunAt timestamp
  #   6. RelatesTo edge with coOccurrenceCount=1 loaded, then same pair loaded again → coOccurrenceCount becomes 2 and strength recalculated
  #
  # ========================================
  Background: User Story
    As an agent developer
    I want to have extracted graph entities persisted to the nanograph database via merge/upsert with idempotent semantics
    So that re-indexing sessions produces consistent results without duplicates

  Scenario: GraphEntity nodes are converted to JSONL and loaded into nanograph
    Given a Vec of 2 Concept GraphEntity nodes with valid slugs, names, and categories
    When the entities are converted to JSONL and loaded via merge mode
    Then 2 Concept rows are visible in the database with correct slug, name, and category values

  Scenario: Duplicate concept slug merges with increment semantics
    Given a Concept node with slug "jwt-auth" and mentionCount 3 already exists in the database
    When the same slug is loaded again with mentionCount 2 and a later lastSeen timestamp
    Then the mentionCount is 5 (summed, not overwritten)
    And the firstSeen timestamp is preserved from the original load
    And the lastSeen timestamp is updated to the later value

  Scenario: Confidence is promoted on merge
    Given a Concept node with slug "test-concept" and confidence "medium" exists in the database
    When the same slug is loaded with confidence "high"
    Then the confidence is promoted to "high"

  Scenario: Confidence is not demoted on merge
    Given a Concept node with slug "stable-concept" and confidence "high" exists in the database
    When the same slug is loaded with confidence "low"
    Then the confidence remains "high"

  Scenario: Watermark state updated after successful upsert
    Given an empty index-state.json
    When a batch of entities from session "abc-123" up to turn 42 is successfully loaded
    Then the index-state.json contains a watermark entry for session "abc-123" with lastIndexedTurn 42
    And the lastRunAt timestamp is updated to the current time

  Scenario: RelatesTo edge co-occurrence count and strength are updated on merge
    Given a RelatesTo edge between "jwt-auth" and "session-mgmt" with coOccurrenceCount 1
    When the same concept pair is loaded again as a RelatesTo edge
    Then the coOccurrenceCount becomes 2
    And the strength is recalculated as min(1.0, log2(3) / 10.0)
