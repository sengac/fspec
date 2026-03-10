@CMPCT-003
Feature: Compaction completion incorrectly sets status to Idle while agent loop is still running

  """
  Rust side: stream_loop.rs emits CompactionStarted/Complete/Continuing. NAPI BackgroundOutput::emit() translates to status changes. Agent loop in session_manager.rs calls apply_pending_dag() after stream completes. JS side: AgentView.tsx handles SessionStateChange and CompactionComplete chunks via useCompaction hook.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CompactionComplete must NOT be emitted from stream_loop after execute_compaction() returns — the setup phase is not the completion
  #   2. CompactionComplete must only be emitted from agent_loop after apply_pending_dag() succeeds
  #   3. CompactionContinuing in NAPI BackgroundOutput::emit must set status to Running (not be a no-op) so the UI shows activity during DAG construction
  #   4. Session status must never be Idle while the agent is actively processing (streaming, building DAG, or calling tools)
  #   5. The JS CompactionComplete handler in AgentView must only call endCompaction() — it must NOT be reached while DAG construction is in progress
  #   6. The JS SessionStateChange handler must NOT call endCompaction() when state is Running during an active compaction — only CompactionComplete should end it
  #   7. The initial compaction phase text must NOT reference anchors — anchors no longer exist in the compaction system
  #   8. performManualCompaction must NOT call endCompaction via setTimeout — CompactionComplete from agent_loop is the definitive end signal
  #
  # EXAMPLES:
  #   1. Post-loop compaction: agent finishes (Done→Idle), compaction triggers (CompactionStarted→Compacting), execute_compaction runs (~5ms), CompactionContinuing→Running, retry stream runs (status stays Running), stream finishes (Done→Idle), apply_pending_dag, emit CompactionComplete
  #   2. Pre-prompt compaction: threshold exceeded before API call → CompactionStarted→Compacting, execute_compaction (~5ms), CompactionContinuing→Running, main stream processes compaction instruction, Done→Idle, apply_pending_dag, emit CompactionComplete
  #   3. No DAG pending: agent responds normally without compaction. Done→Idle. No CompactionComplete emitted from agent_loop (apply_pending_dag returns false)
  #
  # ========================================

  Background: User Story
    As a user
    I want to see accurate status indicators during compaction
    So that I know the agent is still working and don't try to type while it's building the DAG

  @post-loop
  Scenario: Post-loop compaction keeps status active through DAG construction
    Given an agent stream has just finished and emitted Done
    And the compaction hook detected that compaction is needed
    When stream_loop emits CompactionStarted
    Then the session status should be Compacting
    When execute_compaction completes the in-memory setup phase
    Then stream_loop must NOT emit CompactionComplete
    When stream_loop emits CompactionContinuing
    Then the NAPI handler should set session status to Running
    And the retry stream should begin processing the compaction instruction
    When the retry stream finishes and emits Done
    And the agent_loop calls apply_pending_dag which returns true
    Then the agent_loop should emit CompactionComplete
    And the session status should transition to Idle

  @pre-prompt
  Scenario: Pre-prompt compaction keeps status active through DAG construction
    Given a user prompt would exceed the compaction threshold
    When stream_loop emits CompactionStarted before the API call
    Then the session status should be Compacting
    When execute_compaction completes the in-memory setup phase
    Then stream_loop must NOT emit CompactionComplete
    When stream_loop emits CompactionContinuing
    Then the NAPI handler should set session status to Running
    And the main stream should process the compaction instruction
    When the main stream finishes and emits Done
    And the agent_loop calls apply_pending_dag which returns true
    Then the agent_loop should emit CompactionComplete
    And the session status should transition to Idle

  @no-compaction
  Scenario: Normal response without compaction does not emit CompactionComplete from agent_loop
    Given an agent stream completes normally without triggering compaction
    When the stream emits Done
    Then the session status should be Idle
    And the agent_loop calls apply_pending_dag which returns false
    And no CompactionComplete event should be emitted from the agent_loop

  @napi
  Scenario: CompactionContinuing sets status to Running instead of being a no-op
    Given the session status is Compacting after CompactionStarted was emitted
    When the NAPI BackgroundOutput receives a CompactionContinuing event
    Then the session status should be set to Running
    And a SessionStateChange with Running should be emitted to JavaScript

  @napi
  Scenario: CompactionComplete is not emitted prematurely from stream_loop
    Given execute_compaction has completed the in-memory setup phase
    When stream_loop processes the successful compaction result
    Then stream_loop must emit CompactionContinuing instead of CompactionComplete
    And no CompactionComplete event should be emitted at this point
