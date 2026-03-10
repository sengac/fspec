@CMPCT-015
Feature: After inject_summary ends compaction, isLoading not set while agent loop continues running

  """
  Uses Rust-side approach: on_injected callback emits SessionStateChange(Running) before CompactionComplete
  JS-side: CompactionComplete handler must call refreshRustState() after endCompaction() so isLoading reflects current Rust status
  Rust-side: Done handler should keep status as Running when pending_dag_content has content (apply_pending_dag hasn't run yet)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After CompactionComplete from on_injected, JS must call refreshRustState() so isLoading reflects the current Rust session status (Running)
  #   2. The on_injected callback in Rust must emit SessionStateChange(Running) BEFORE CompactionComplete so JS picks up isLoading=true before isCompacting=false
  #   3. The Done handler must NOT set status to Idle when apply_pending_dag has not yet run — use a flag or check pending_dag_content
  #   4. InputTransition must smoothly transition from Compacting to Thinking display without flickering to idle in between
  #
  # EXAMPLES:
  #   1. Compaction finishes (inject_summary called) while agent is still streaming → UI transitions from Compacting to Thinking, user can press Esc to stop
  #   2. inject_summary fires as the last tool call → CompactionComplete arrives, then Done arrives very shortly after → UI briefly shows Thinking then transitions to idle (correct since agent is done)
  #   3. Agent fails to call inject_summary (error or interruption) → Done fires → compaction_in_progress still true → status stays Running (from CompactionContinuing) → agent_loop cleanup clears flag and sets Idle
  #
  # ========================================

  Background: User Story
    As a user
    I want to see the Thinking indicator with Esc-to-stop after compaction completes but the agent loop is still running
    So that I can stop the agent if needed and know it's still working

  @napi
  Scenario: on_injected emits SessionStateChange Running before CompactionComplete
    Given the agent loop is processing a compaction instruction
    And the Rust session status is Running from CompactionContinuing
    When the inject_summary handler fires the on_injected callback
    Then a SessionStateChange with state Running must be emitted before CompactionComplete
    And the Rust session status must remain Running after CompactionComplete is sent

  @napi
  Scenario: Done handler keeps status Running when pending DAG has not been applied
    Given the inject_summary handler has stored DAG content in pending_dag_content
    And compaction_in_progress has been cleared by inject_summary
    When the stream finishes and the Done event fires
    Then the Done handler must check pending_dag_content before setting Idle
    And the status must remain Running if pending_dag_content has content

  @napi
  Scenario: Agent fails to call inject_summary and cleanup prevents permanent compaction flag
    Given compaction_in_progress is true because compaction was started
    And the agent fails without calling inject_summary so no DAG is stored
    When the agent_loop cleanup runs after the stream completes
    Then compaction_in_progress must be unconditionally cleared to false
    And the session status must be set to Idle since no DAG is pending

