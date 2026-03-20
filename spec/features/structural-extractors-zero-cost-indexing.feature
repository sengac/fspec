@KGRAPH-004
Feature: Structural Extractors (Zero-Cost Indexing)

  """
  Pure extractor functions in codelet/napi/src/graph/extractors.rs. Batch EntityQueue in the same file. Integration hook in session_manager.rs after tool call responses. No LLM dependency — pattern matching only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Extractors are deterministic (regex/pattern matching) — no LLM API calls, zero token cost
  #   2. Write/Edit tool calls produce CodeEntity nodes with file path, language (from extension), and Modifies edges (Turn → CodeEntity)
  #   3. Fspec tool calls produce WorkUnit nodes from create-story/bug/task commands and WorksOn edges from update-work-unit-status
  #   4. Entities are queued in a batch buffer and flushed periodically (threshold count or session idle), not on every single turn
  #   5. Extractor functions are pure — take tool call data as input, return a list of graph entities to upsert (no side effects)
  #   6. If graph DB is not available (not initialized, feature disabled), extractors are silently skipped — no errors
  #
  # EXAMPLES:
  #   1. Agent edits src/auth/login.rs → CodeEntity node created with slug 'src-auth-login-rs', language 'rust', entityType 'file'
  #   2. Fspec create-story creates a WorkUnit node in the graph; later update-work-unit-status adds WorksOn edge from current session
  #   3. 50 file edits accumulate in queue → flushed as a batch insert of 50 CodeEntity nodes in one graph write
  #
  # ========================================

  Background: User Story
    As an agent developer
    I want to have my agent sessions automatically populate the knowledge graph from tool calls without any LLM cost
    So that the graph accumulates useful structured data (file modifications, work unit activity) as a side effect of normal work

  Scenario: File edit creates CodeEntity node
    Given the graph database is initialized
    When an Edit tool call modifies 'src/auth/login.rs'
    Then a CodeEntity node is produced with the file path, language 'rust', and entityType 'file'
    And a Modifies edge is produced linking the current turn to the CodeEntity


  Scenario: Fspec create-story produces WorkUnit node
    Given the graph database is initialized
    When an Fspec tool call with command 'create-story' creates work unit 'AUTH-001'
    Then a WorkUnit node is produced with slug 'AUTH-001', title, and workType 'story'


  Scenario: Batch queue flushes at threshold
    Given the entity queue has 49 pending entities
    When one more entity is added to the queue
    Then all 50 entities are flushed to the graph database
    And the queue is empty after flush


  Scenario: Extractors silently skip when graph is unavailable
    Given the graph database is not initialized
    When a Write tool call is processed
    Then no entities are queued and no error is raised

