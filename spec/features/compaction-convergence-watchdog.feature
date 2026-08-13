@CMPCT-020
Feature: Compaction Convergence Watchdog
  """
  New functions in rust/cli/src/interactive_helpers.rs: COMPACTION_ESCALATION_MESSAGE constant, force_inject_fallback_dag(), extract_partial_dag_nodes()
  Watchdog retry logic in rust/napi/src/session_manager.rs agent_loop — after run_with_provider check compaction_in_progress and retry with escalation
  Reuses wrap_dag_content from inject_summary_handler.rs and reset_session_to_reminders + recalculate_token_tracker from interactive_helpers.rs
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After each agent_loop stream invocation during compaction, check if inject_summary was called
  #   2. Attempt 1 (normal): Agent has one full stream to build DAG and call inject_summary
  #   3. Attempt 2 (escalation): If attempt 1 failed, inject escalation message and run another stream
  #   4. Attempt 3 (force-inject): If attempt 2 also failed, engine constructs fallback DAG deterministically
  #   5. Force-inject scans recent messages for partial <dag-node> blocks
  #   6. Watchdog counter resets to 0 when inject_summary succeeds
  #   7. Force-inject uses reset_session_to_reminders + inject DAG directly into session.messages
  #
  # ========================================
  Background: User Story
    As a compaction engine
    I want to guarantee compaction convergence with a watchdog that escalates when the agent fails to call inject_summary
    So that compaction never gets stuck indefinitely, even when the agent loops or forgets to finalize

  Scenario: Normal compaction succeeds without watchdog intervention
    Given a session in compaction mode after execute_compaction
    When the agent calls inject_summary during the first stream attempt
    Then the compaction_in_progress flag should be cleared
    And no escalation message should be injected
    And the watchdog counter should remain at 0

  Scenario: Escalation triggers after first failed attempt
    Given a session in compaction mode where the agent never calls inject_summary
    When the first stream attempt completes without inject_summary
    Then the watchdog should detect that compaction_in_progress is still true
    And an escalation message should be injected into session messages
    And a second stream attempt should be initiated automatically

  Scenario: Force-inject with partial dag-nodes after two failed attempts
    Given a session in compaction mode where the agent writes partial dag-node blocks but never calls inject_summary
    And the recent messages contain dag-node blocks for turns 0-30 and 31-50
    When both stream attempts complete without inject_summary
    Then the engine should extract partial dag-node blocks from recent messages
    And assemble them into a complete DAG
    And force-inject the assembled DAG into the session
    And clear the compaction_in_progress flag

  Scenario: Force-inject with minimal fallback DAG when no partial nodes exist
    Given a session in compaction mode where the agent produces no dag-node blocks at all
    When both stream attempts complete without inject_summary
    Then the engine should create a minimal fallback DAG with a D1 node
    And the fallback D1 node should cover turns 0 through the last known turn
    And the fallback label should indicate auto-recovery from compaction timeout
    And force-inject the fallback DAG into the session

  Scenario: Escalation message content is directive
    Given the COMPACTION_ESCALATION_MESSAGE constant
    When its content is examined
    Then it should instruct the agent to stop making SessionSearch calls
    And it should instruct the agent to write a summary and call inject_summary immediately
    And it should convey urgency about the compaction timeout

  Scenario: force_inject_fallback_dag resets session state correctly
    Given a session with conversation messages and compaction_in_progress flag set to true
    When force_inject_fallback_dag is called with a fallback DAG
    Then it should call reset_session_to_reminders to preserve system reminders
    And the DAG should be wrapped in compaction-dag system-reminder tags
    And the wrapped DAG should be pushed as a user message
    And recalculate_token_tracker should update the token counts
    And the compaction_in_progress flag should be cleared

  Scenario: extract_partial_dag_nodes finds dag-node blocks in messages
    Given session messages containing partial dag-node blocks from assistant responses
    When extract_partial_dag_nodes is called
    Then it should find and return all dag-node block strings from assistant messages

  Scenario: extract_partial_dag_nodes returns empty when no blocks exist
    Given session messages with no dag-node blocks
    When extract_partial_dag_nodes is called
    Then it should return an empty collection
