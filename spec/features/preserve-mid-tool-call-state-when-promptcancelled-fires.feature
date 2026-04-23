@done
@context-window
@error-handling
@resilience
@cli
@CMPCT-029
Feature: Preserve mid-tool-call state when PromptCancelled fires
  """
  Rig patch at site 509 only: before yielding PromptCancelled after on_tool_result, push Message::Assistant(tool_calls) and Message::User(tool_results) into chat_history so the error payload carries the complete pair. Sites 486 and 542 are untouched.
  fspec-side recovery in stream_loop.rs compaction-cancel branch (Path C): before invoking begin_compaction_recovery, (1) extract_prompt_cancelled to get rig_chat_history, (2) reconcile_session_messages appends any missing tool pairs, (3) inject_synthetic_tool_results_for_orphans closes any remaining dangling tool_calls with status=cancelled_by_context_limit.
  validate_no_orphan_tool_calls(messages) runs defensively at the start of execute_compaction in interactive_helpers.rs. It returns Err(Vec<String>) listing orphan call_ids, and execute_compaction surfaces that as anyhow::Error so a misbehaving recovery path cannot quietly corrupt the compaction input.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Site 509 cancellation must flush the matching tool_call/tool_result pair from rig's local vecs into chat_history before yielding PromptCancelled
  #   2. Site 486 cancellation is recovered entirely on the fspec side using tool_calls_buffer; no rig patch needed
  #   3. Site 542 cancellation (mid tool_call_delta) is unrecoverable; any partial tool_call_delta is dropped
  #   4. Orphan tool_call detection must run at execute_compaction entry; orphans are resolved by injecting a synthetic tool_result with status='cancelled_by_context_limit'
  #   5. When PromptCancelled carries a rig chat_history payload with tool state fspec hasn't seen, fspec must reconcile its session.messages by appending the missing Assistant tool_call and matching User tool_result
  #
  # EXAMPLES:
  #   1. PromptCancelled at site 509 with one tool_call and one tool_result in rig's local vecs — after recovery session.messages contains the complete Assistant(ToolCall) + User(ToolResult) pair
  #   2. PromptCancelled at site 486 with a tool_call already buffered in fspec's tool_calls_buffer — after recovery session.messages contains the Assistant(ToolCall) plus a synthetic User(ToolResult) marked cancelled_by_context_limit
  #   3. A session with a clean conversation (every Assistant tool_call has a matching User tool_result) passes the orphan detector with no orphans and no synthetic injections
  #   4. Reconciliation handles a rig chat_history containing a tool_call/result pair that fspec did not yet know about — the pair is appended to session.messages before compaction runs
  #   5. execute_compaction refuses to run when orphan tool_calls remain and reports which call_ids are orphaned so the caller can log a diagnostic and inject synthetic tool_results
  #
  # ========================================
  Background: User Story
    As a developer
    I want to recover session.messages integrity when PromptCancelled fires mid-tool-call
    So that compaction continues without dangling tool_calls that break the next API request

  Scenario: PromptCancelled at site 509 preserves the complete tool_call and tool_result pair in session.messages
    Given the streaming hook fires cancel immediately after on_tool_result in rig
    And rig's local tool_calls vec contains one tool_call and tool_results vec contains its matching result
    And the rig patch has flushed that pair into chat_history before yielding PromptCancelled
    When fspec's compaction-cancel branch consumes the PromptCancelled error
    Then reconciliation appends the missing Assistant(ToolCall) message to session.messages
    And reconciliation appends the matching User(ToolResult) message to session.messages
    And the orphan detector reports zero orphan call_ids for the reconciled session

  Scenario: PromptCancelled at site 486 closes the dangling tool_call with a synthetic cancelled tool_result
    Given fspec's tool_calls_buffer already contains one Assistant(ToolCall) for an in-flight tool
    And the streaming hook fires cancel during on_tool_call before the tool has executed
    And the session.messages tail already holds the Assistant(ToolCall) from the tool_calls_buffer flush
    When fspec's compaction-cancel branch runs inject_synthetic_tool_results_for_orphans
    Then the dangling tool_call receives a matching User(ToolResult) carrying status "cancelled_by_context_limit"
    And the synthetic tool_result uses the original call_id so the pair is structurally complete
    And the orphan detector now reports zero orphan call_ids

  Scenario: Clean conversation passes the orphan detector with no changes
    Given a session whose Assistant tool_calls each have a matching User tool_result
    When validate_no_orphan_tool_calls inspects session.messages
    Then the detector returns Ok with zero orphan call_ids
    And inject_synthetic_tool_results_for_orphans reports zero synthetic injections
    And execute_compaction is allowed to proceed

  Scenario: Reconciliation does not duplicate tool pairs that fspec already holds
    Given rig's chat_history payload contains an Assistant(ToolCall) and matching User(ToolResult)
    And fspec's session.messages already contains the same Assistant(ToolCall) and User(ToolResult) pair
    When reconcile_session_messages runs with the rig_chat_history
    Then no new messages are appended to session.messages
    And the deduplication key is the tool correlation id (call_id if present, otherwise id)
    And the orphan detector still reports zero orphan call_ids

  Scenario: execute_compaction refuses to run when orphan tool_calls remain and reports the offending call_ids
    Given a session whose tail contains an Assistant(ToolCall) with no matching User(ToolResult)
    And no reconciliation or synthetic injection has been performed
    When execute_compaction is invoked
    Then it returns an error describing the orphan call_ids
    And no compaction instruction is pushed onto session.messages
    And the token tracker is NOT reset
