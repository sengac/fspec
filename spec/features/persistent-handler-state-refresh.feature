@done
@BUG-101
Feature: Persistent chunk handler refreshes React state on session state changes

  """
  Fix is in persistentChunkHandler in AgentView.tsx — add refreshRustState call for all
  SessionStateChange events. The endCompaction guard still applies (only CompactionComplete
  ends compaction indicator).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. persistentChunkHandler must call refreshRustState for SessionStateChange chunks so React picks up isLoading/isPaused transitions that arrive after the streaming handler is cleaned up
  #   2. The endCompaction guard must remain — only CompactionComplete should end the compaction indicator, not Running or Idle state changes
  #
  # EXAMPLES:
  #   1. inject_summary called during streaming → Done arrives with pending DAG → streaming handler resolves → cleanup runs → apply_pending_dag emits SessionStateChange(Idle) → persistentChunkHandler receives it → must call refreshRustState → isLoading transitions to false
  #   2. SessionStateChange(Running) arrives via persistentChunkHandler during /compact flow → refreshRustState called → isLoading=true shown correctly
  #
  # ========================================

  Background: User Story
    Given a user interacting with the AI agent in the TUI

  @BUG-101
  Scenario: SessionStateChange(Idle) via persistent handler transitions isLoading to false
    Given the streaming handler has been cleaned up after a Done chunk
    And the Rust session status transitions to Idle after apply_pending_dag
    When the persistentChunkHandler receives a SessionStateChange with state Idle
    Then refreshRustState should be called for the current session
    And isLoading should transition to false

  @BUG-101
  Scenario: SessionStateChange(Running) via persistent handler keeps isLoading true
    Given the streaming handler has been cleaned up
    And the Rust session status is Running during a compact flow
    When the persistentChunkHandler receives a SessionStateChange with state Running
    Then refreshRustState should be called for the current session
    And isLoading should remain true

  @BUG-101
  Scenario: endCompaction guard preserved for non-CompactionComplete state changes
    Given a compaction is in progress with isCompacting true
    When the persistentChunkHandler receives a SessionStateChange with state Idle
    Then endCompaction should NOT be called
    And the compaction indicator should remain visible until CompactionComplete arrives
