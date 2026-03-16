@AMGR-014
Feature: Spawned subordinate sessions not searchable — no persistence manifest created

  """
  handle_spawn creates a SessionManifest via with_provider() before create_session_with_id(). Manifest ID overridden to match subordinate UUID. Provider extracted from model string split on '/'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Subordinate sessions must have a persistence manifest created before the in-memory session, so messages are stored on disk
  #   2. The manifest ID must match the subordinate session UUID so persistence and in-memory session are linked
  #   3. The provider field in the manifest should be extracted from the model string (e.g. 'anthropic/claude-opus-4-6' → 'anthropic')
  #   4. If manifest creation fails, the session should still be created (degraded mode — works but not searchable)
  #
  # EXAMPLES:
  #   1. Parent session spawns subordinate, sends it a task. Parent then uses SessionSearch to find the subordinate's conversation — subordinate shows up in 'recent' and its messages appear in 'search'
  #   2. Subordinate processes a message from parent. The user and assistant messages are persisted to disk and can be loaded by load_session()
  #   3. Persistence manifest creation fails (e.g. disk full). Spawn still succeeds but logs a warning. Session works for the current run but history won't survive restart
  #
  # ========================================

  Background: User Story
    As a AI agent operator
    I want to search the conversation history of spawned subordinate sessions
    So that I can review what my subordinate agents discussed and find relevant information across all sessions

  Scenario: Persistence manifest created before session
    Given a parent session with model "anthropic/claude-opus-4-6"
    When the parent spawns a subordinate via AgentManager
    Then a persistence manifest is saved with the subordinate's UUID
    Then the manifest provider field is "anthropic"
    Then the manifest is created before create_session_with_id is called


  Scenario: Subordinate messages are searchable via SessionSearch
    Given a spawned subordinate session with a persistence manifest
    When the subordinate processes a message and produces a response
    Then the subordinate session appears in SessionSearch recent results for the project
    Then the subordinate's messages are found via SessionSearch search action


  Scenario: Spawn succeeds even when persistence manifest creation fails
    Given a parent session with model "anthropic/claude-opus-4-6"
    When the parent spawns a subordinate via AgentManager
    Then the subordinate session is still created successfully
    Given the persistence layer will fail to save the manifest
    Then a warning is logged about the persistence failure

