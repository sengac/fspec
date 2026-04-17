@KGRAPH-022
Feature: Cross-Session Learning & Periodic Synthesis
  """
  Implementation uses learnings_context.rs in codelet/napi/src/graph/ for the standalone context building function, integrated via session_start hook in session_manager.rs
  Context injection uses existing learnings_dispatch::dispatch_learnings_search and dispatch_learnings_decisions functions — no new nanograph queries needed, only formatting and integration logic
  Post-session extraction hooks into the existing session_end hook (HOOK-013) to trigger learnings extraction from the session's compaction DAG, reusing extract_learnings_from_text from KGRAPH-021
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. At session start, when a work unit is assigned, the system must query the Learnings graph for relevant context (decisions, constraints, conventions, failed approaches) matching the work unit's domain and inject it as a system reminder
  #   2. Explorations with outcome 'failure' or 'abandoned' must be specifically surfaced as warnings when the session touches the same domain, including the failureConstraint text
  #   3. Context injection must be a standalone function in its own file (learnings_context.rs) that takes a query string and returns formatted learnings context or None if no relevant learnings exist
  #   4. When AgentManager spawns subordinate sessions, the system must inject relevant learnings from the supervisor's work unit domain into the subordinate's system prompt context
  #   5. Context injection must be non-blocking — if the Learnings graph is not initialized or the query fails, the session must start normally without learnings context
  #   6. Injected learnings context must be formatted as a system-reminder with type 'learningsContext' containing decisions, constraints, and failed approaches in a structured format
  #   7. Periodic synthesis must run as a post-session hook (after session_end) that extracts learnings from the completed session and loads them into the Learnings graph
  #   8. The context volume must be capped at 2000 tokens max to avoid consuming too much context window — use the most relevant learnings first, truncate if needed
  #
  # EXAMPLES:
  #   1. Session starts with work unit KGRAPH-022 assigned, Learnings graph has Decision 'use-dual-graph' in domain 'architecture' — context injection includes this decision in the system reminder
  #   2. Session starts in knowledge-graph domain, Learnings graph has Exploration 'monolithic-indexing' with outcome 'failure' — failure warning surfaces 'caused 7.6GB disk consumption' constraint
  #   3. Session starts but Learnings graph is not initialized (first run) — session starts normally with no learnings context, no errors
  #   4. AgentManager spawns subordinate for security review — subordinate receives relevant learnings about security patterns from supervisor's domain context
  #   5. Session ends and post-session extraction runs — compaction DAG content is passed to learnings_extraction and entities are loaded into the Learnings graph
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to receive relevant learnings from past sessions when starting work on a domain
    So that I avoid repeating failed approaches and leverage accumulated knowledge

  @happy-path
  Scenario: Inject relevant decisions into session context at session start
    Given a Learnings graph with a Decision node "use-dual-graph" in domain "architecture" with status "active"
    And the session is assigned to work unit "KGRAPH-022" in the "knowledge-graph" epic
    When the context injection function is called with query "knowledge-graph"
    Then the returned context should contain the decision "use-dual-graph"
    And the context should be formatted as a system-reminder with type "learningsContext"
    And the context should include the decision rationale

  @happy-path
  Scenario: Surface failed explorations as warnings in session context
    Given a Learnings graph with an Exploration node "monolithic-indexing" with outcome "failure"
    And the Exploration has failureConstraint "caused 7.6GB disk consumption"
    When the context injection function is called with query "knowledge-graph"
    Then the returned context should contain a warnings section
    And the warnings section should include "monolithic-indexing" as a failed approach
    And the warnings section should include the constraint "caused 7.6GB disk consumption"

  @edge-case
  Scenario: Graceful fallback when Learnings graph is not initialized
    Given the Learnings graph is not initialized
    When the context injection function is called with query "any-domain"
    Then the function should return None
    And no error should be raised

  @integration
  Scenario: Subordinate session receives learnings from supervisor domain
    Given a Learnings graph with a Learning node "bcrypt-hashing" in category "convention"
    And the supervisor session is working on domain "authentication"
    When a subordinate session is spawned for the "authentication" domain
    Then the subordinate context should include the learning "bcrypt-hashing"

  @integration
  Scenario: Post-session learnings extraction loads entities into graph
    Given a completed session with compaction DAG content containing learnings
    And a mock LLM response with extracted Learning and Exploration entities
    When post-session extraction is triggered with the DAG content
    Then the extracted entities should be loaded into the Learnings graph
    And the Learnings graph should contain the new Learning nodes
    And the Learnings graph should contain the new Exploration nodes

  @edge-case
  Scenario: Context volume capped at token limit
    Given a Learnings graph with 50 Learning nodes matching query "large-domain"
    When the context injection function is called with query "large-domain"
    Then the returned context should not exceed 2000 tokens
    And the most relevant learnings should be included first
