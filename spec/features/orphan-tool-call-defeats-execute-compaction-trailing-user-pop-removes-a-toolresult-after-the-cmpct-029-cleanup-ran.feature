@done
@cli
@compaction
@context-window
@error-handling
@resilience
@CMPCT-050
Feature: Orphan tool_call defeats execute_compaction: trailing-User pop removes a ToolResult AFTER the CMPCT-029 cleanup ran
  """
  Fix shape (from root-cause-analysis.md): (1) recovery_compaction.rs begin_compaction_recovery — restrict the trailing-User pop to messages with NO UserContent::ToolResult items; (2) run inject_synthetic_tool_results_for_orphans as the final step of begin_compaction_recovery (after the pop + partial-text flush) so every entry path gets the guarantee; Path C keeps its explicit reconcile since only the PromptCancelled error chain carries rig chat_history; (3) manual /compact paths (sessions handle_impl.rs compact_session + NAPI session_compact) run a pre-flight sanitize before execute_compaction; (4) regression test first (ACDD): feature file + test-helpers-based unit test asserting tail ToolResult is preserved and validate_no_orphan_tool_calls passes, plus a control test that a trailing text prompt is still popped. Attachments root-cause-analysis.md (E1-E8 in code-evidence.md) carry all line-number evidence. Estimated 3 points.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The trailing-User pop in begin_compaction_recovery (pop_user_prompt=true) must only pop when the trailing User message contains NO UserContent::ToolResult items — a trailing User(ToolResult) is mid-turn conversation state and must be preserved
  #   2. The orphan safety net (reconcile with rig chat_history where the PromptCancelled payload is available, drain tool_calls_buffer, inject synthetic cancelled tool_results for remaining orphans) must run on EVERY compaction-recovery entry path (B prompt-too-long, C PromptCancelled, D Gemini continuation, CMPCT-032 clean-exit, CMPCT-044 overflow) before execute_compaction — not only Path C
  #   3. The safety net must run AFTER any pop or other mutation in begin_compaction_recovery so that the post-mutation state is the one that is guaranteed clean — cleanup must not precede the pop (the current Path C ordering is exactly what this bug fixes)
  #   4. The defensive orphan guard in execute_compaction (validate_no_orphan_tool_calls → refuse) is retained unchanged as the final assertion; it must still refuse when an orphan survives all upstream recovery, but after this fix it should only fire on a genuinely unrecoverable state
  #   5. No behavior change for the original case: a trailing plain User text prompt (no ToolResult items) at the tail of session.messages must still be popped when pop_user_prompt=true, so compaction does not embed an unconsumed duplicate prompt
  #
  # EXAMPLES:
  #   1. Reproducing shape (Path C): session.messages ends with [Assistant(ToolCall call_X), User(ToolResult call_X)]; PromptCancelled fires on the usage update right after the tool result; begin_compaction_recovery(pop_user_prompt=true) runs → AFTER the fix the User(ToolResult) is preserved, validate_no_orphan_tool_calls returns Ok, and execute_compaction proceeds instead of refusing
  #   2. User's experience (fixed): a sub-agent near its context limit shows 'Compaction in progress' and then resumes seamlessly with a DAG summary — the 'Compaction failed: execute_compaction refuses to proceed: 1 orphan tool_call(s)...' notification never appears, and the next turn starts normally instead of re-failing on the same persisted orphan
  #   3. Manual /compact with a persisted orphan: a session file already containing an orphan tool_call from a prior failed turn — user runs /compact → the session recovers (orphan closed with the cancelled marker or the result restored from history) and the DAG builds, instead of returning 'Compaction failed: execute_compaction refuses to proceed'
  #   4. Integration (every entry path): the same orphan-free guarantee holds whether compaction is triggered by a 'prompt too long' API error, a PromptCancelled hook cancel, a Gemini continuation cancel, the CMPCT-032 clean-exit flag, or a CMPCT-044 context-overflow error — each path ends in execute_compaction passing its orphan guard
  #   5. Control case (no behavior change): session.messages ends with a plain User text prompt (e.g. the user's own question that rig pushed but the API never consumed); begin_compaction_recovery(pop_user_prompt=true) still pops it exactly as before, so compaction does not embed a duplicate unconsumed prompt
  #
  # ========================================
  Background: User Story
    As a sub-agent or interactive session near its context limit
    I want to have every compaction-recovery entry path preserve the tool-call/tool-result pairing invariant in session.messages
    So that execute_compaction's orphan guard passes, compaction succeeds, and the session is not stuck in a retry-forever loop after a PromptCancelled fired right after a tool result

  Scenario: begin_compaction_recovery preserves a trailing User(ToolResult) at the tail of session.messages
    Given a session whose messages end with an Assistant(ToolCall) for call X and the matching User(ToolResult) for call X
    When begin_compaction_recovery runs with pop_user_prompt=true
    Then the trailing User(ToolResult) message is preserved in session.messages
    And validate_no_orphan_tool_calls reports zero orphan call_ids for the session

  @integration
  Scenario: PromptCancelled firing right after a tool result lets compaction resume without a failure notification
    Given a sub-agent stream in which the CompactionHook cancels on the usage update immediately after a tool result
    And session.messages ends with the matching Assistant(ToolCall) and User(ToolResult) pair
    When the compaction-recovery path runs and execute_compaction builds the DAG instruction
    Then no "Compaction failed" notification is emitted
    And execute_compaction succeeds instead of refusing on the orphan guard
    And the post-compaction session is tool-pair-clean for the next turn

  Scenario: A trailing plain User text prompt is still popped (control — no behavior change)
    Given a session whose messages end with a plain User text prompt that the API has not consumed
    When begin_compaction_recovery runs with pop_user_prompt=true
    Then the trailing User text prompt is removed from session.messages
    And no User(ToolResult) message is altered or removed

  @integration
  Scenario: Orphan safety net closes orphans after all mutations on every entry path
    Given a session whose persisted messages contain an orphan Assistant(ToolCall) from a prior turn
    When any compaction-recovery entry path (prompt-too-long, PromptCancelled, Gemini continuation, clean-exit, or context-overflow) runs the safety net before execute_compaction
    Then the orphan call receives a synthetic User(ToolResult) marked "cancelled_by_context_limit"
    And execute_compaction's orphan guard passes for that path

  Scenario: Manual /compact recovers a persisted orphan instead of failing
    Given a session file whose messages contain an orphan tool_call from a previously failed compaction
    When the user runs /compact
    Then compaction succeeds and DAG construction begins
    And no "Compaction failed: execute_compaction refuses to proceed" error is returned to the user

  Scenario: execute_compaction still refuses when an orphan survives with no recovery performed
    Given a session whose messages contain an orphan Assistant(ToolCall) with no matching User(ToolResult)
    And no reconciliation, drain, or synthetic injection has been performed
    When execute_compaction is invoked directly
    Then it returns an error listing the orphan call_id
    And the defensive guard behavior of CMPCT-029 is unchanged
