@done
@CMPCT-012
Feature: Emergency Threshold Compaction Safety Net

  """
  CMPCT-011 created annotation_detector module (codelet/core/src/compaction/annotation_detector.rs) with detect_annotations(), ToolCallInfo, and TurnContext — all pub, tested, ready to call. When rewriting stream_loop's compaction trigger, also wire per-turn annotation detection: after each completed turn, call detect_annotations() with the turn's tool calls, serialize resulting Vec<StructuralAnnotation> into the persisted message metadata (StoredMessage.metadata HashMap). This is required by CMPCT-011 rules [7],[9],[10] but the wiring naturally belongs here since stream_loop.rs is being modified anyway.
  CMPCT-011 provides execute_compaction_legacy() as the named swap target. The stream_loop currently calls execute_compaction_legacy() in two places (pre-prompt compaction ~line 464, post-loop compaction ~line 1530). Replace both with execute_compaction(session, compaction_in_progress_flag) and thread the Arc<AtomicBool> through the run_agent_stream call chain. Also update repl_loop.rs /compact command to use the new flow.
  The SessionSearch retrieval path should surface persisted annotations during DAG construction. After CMPCT-012 wires detect_annotations() into the stream loop and annotations are stored in message metadata, SessionSearch handle_show/handle_search should include annotations in the output so the agent can navigate by structural signals (milestones, error resolutions, file modifications) rather than scanning raw text.
  execute_compaction signature needs extending: add last_user_message: Option<&str> parameter. When present, the compaction instruction embeds the original prompt so the agent knows what to resume after DAG construction. Pre-prompt case passes Some(prompt), /compact case passes None (agent initiated, no pending work).
  Post-loop compaction retry (stream_loop.rs:1530-1594): The old flow pushed original prompt + started retry stream. New flow: execute_compaction injects compaction instruction (with original prompt embedded) → start retry stream with empty/synthetic prompt → agent processes compaction instruction → builds DAG → calls inject_summary → finishes turn → stream_loop returns → next user input arrives with compact context.
  run_agent_stream_internal signature at line 385 gains compaction_in_progress: Arc<AtomicBool>. Callers: run_agent_stream (NAPI, passes session.compaction_in_progress), run_agent_stream_with_interruption (CLI, creates local Arc), run_agent_stream_with_images (NAPI, passes session.compaction_in_progress). Macro run_with_provider! passes it through.
  Per-turn annotation wiring location: stream_loop.rs handle_tool_result block already tracks tool calls. After a complete assistant turn (when FinalResponse is received or stream ends), collect ToolCallInfo from the turn's tool calls, call detect_annotations(), and attach to persisted message metadata via the existing message persistence flow.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All three execute_compaction_legacy() call sites must be replaced with execute_compaction(session, compaction_in_progress)
  #   2. Arc<AtomicBool> compaction_in_progress must be threaded through run_agent_stream call chain to reach execute_compaction
  #   3. After emergency compaction, the last user message must be re-appended so the agent knows what to resume
  #   4. Post-compaction context floor: system reminders + injected DAG + last user message — never drops below this
  #   5. CompactionHook threshold detection logic (compaction_hook.rs) is unchanged — only the response to the threshold trigger changes
  #   6. No separate LLM calls during emergency compaction — zero marginal token cost
  #   7. Per-turn annotation detection: after each completed turn in stream_loop, call detect_annotations() and serialize Vec<StructuralAnnotation> into persisted message metadata
  #   8. /compact slash command in repl_loop.rs must use the new in-view flow, not the legacy batch LLM pipeline
  #   9. The post-loop compaction retry logic (re-send prompt to LLM after compaction) must work with the new flow — the agent loop resumes normally with compaction instruction as the effective prompt
  #   10. execute_compaction() returns Ok(()) instead of (CompactionMetrics, Option<AnchorPoint>) — callers must adapt return handling
  #
  # EXAMPLES:
  #   1. Pre-prompt compaction (stream_loop.rs:464): estimated_total > threshold → calls execute_compaction(session, compaction_in_progress) instead of execute_compaction_legacy → sets flag, clears context, injects system instruction → agent loop resumes normally with DAG construction as next task
  #   2. Post-loop hook-triggered compaction (stream_loop.rs:1530): compaction_needed=true after stream completes → calls execute_compaction → then instead of re-sending original prompt to LLM, the compaction instruction IS the new prompt → agent builds DAG → calls inject_summary → resumes original work via last user message
  #   3. /compact command (repl_loop.rs:88): user types /compact → calls execute_compaction(session, compaction_in_progress) → agent builds DAG proactively → same flow as emergency but initiated by agent choice
  #   4. Agent calls Write(src/auth/handler.rs) in a turn. Stream loop completes the turn, calls detect_annotations with ToolCallInfo{tool_name: Write, input: {file_path: src/auth/handler.rs}, success: true}. Gets back vec![FileModification{path: src/auth/handler.rs, operation: Created}]. Serializes into persisted message metadata.
  #   5. run_agent_stream(agent, prompt, session, is_interrupted, interrupt_notify, output) gains a compaction_in_progress: Arc<AtomicBool> parameter. All callers in session_manager.rs pass session.compaction_in_progress.clone(). CLI callers create a new Arc<AtomicBool>::new(false).
  #   6. Post-loop compaction retry: old flow re-sent prompt via prompt_streaming_with_history_and_hook. New flow: after execute_compaction succeeds, the compaction system instruction is already in session.messages as the last user message — the retry just needs to call prompt_streaming_with_history_and_hook with an empty/synthetic continuation, OR skip the retry entirely since execute_compaction already set up the agent to resume.
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to have emergency compaction use the in-view DAG construction flow instead of batch LLM pipeline
    So that context compaction has zero marginal LLM cost and I maintain control over what to compress

  @emergency-compaction
  Scenario: Pre-prompt compaction uses in-view DAG flow instead of legacy batch LLM
    Given a session with estimated token total exceeding the compaction threshold
    And the session has turns available to compact
    When pre-prompt compaction is triggered
    Then execute_compaction is called with the session and compaction_in_progress flag
    And the compaction_in_progress flag is set to true
    And session messages are cleared and system reminders are restored
    And the compaction system instruction is injected as a user message
    And the original user prompt is embedded in the instruction so the agent knows what to resume
    And no separate LLM calls are made during the compaction setup

  @emergency-compaction
  Scenario: Post-loop hook-triggered compaction uses in-view DAG flow
    Given a session where the compaction hook has set compaction_needed to true
    And the stream has completed without interruption
    When post-loop compaction is triggered
    Then execute_compaction is called with the session and compaction_in_progress flag
    And the compaction system instruction includes the original user prompt
    And a retry stream is started so the agent can process the compaction instruction
    And the agent builds a DAG via SessionSearch and calls inject_summary
    And after inject_summary the context contains system reminders plus the DAG summary

  @proactive-compaction
  Scenario: Slash compact command uses in-view DAG flow instead of legacy batch LLM
    Given a session with messages available to compact
    When the /compact command is executed
    Then execute_compaction is called with the session and compaction_in_progress flag
    And no last_user_message is embedded because compaction was agent-initiated
    And the compaction system instruction is injected as a user message
    And the agent can build a DAG via SessionSearch on the next turn

  @compaction-in-progress-flag
  Scenario: compaction_in_progress flag is threaded through run_agent_stream call chain
    Given the run_agent_stream_internal function accepts a compaction_in_progress parameter
    When run_agent_stream is called from NAPI
    Then session.compaction_in_progress is passed through to run_agent_stream_internal
    And the flag is available for execute_compaction within the stream loop

  @compaction-in-progress-flag
  Scenario: CLI callers create a local compaction_in_progress flag
    Given the CLI run_agent_stream_with_interruption function
    When called from the CLI interactive loop
    Then a new Arc AtomicBool initialized to false is created and passed through
    And pre-prompt and post-loop compaction can use the flag

  @context-floor
  Scenario: Post-compaction context never drops below minimum floor
    Given a session that has undergone emergency compaction
    And the agent has built a DAG and called inject_summary
    When the inject_summary handler completes
    Then the context contains system reminders
    And the context contains the injected DAG summary
    And the context contains the last user message
    And the compaction_in_progress flag is cleared to false

  @return-type
  Scenario: Callers adapt to execute_compaction returning Ok instead of metrics
    Given stream_loop.rs previously matched on CompactionMetrics from execute_compaction_legacy
    When the call sites are updated to use execute_compaction
    Then the callers handle Ok(()) on success without expecting metrics or anchor data
    And compaction events are emitted using pre-compaction token counts captured before the call
    And token tracker is reset after compaction

  @annotation-detection
  Scenario: Per-turn annotation detection wired into stream loop
    Given the stream loop has completed processing a turn with tool calls
    And the turn includes a Write tool call to create a file
    When the turn completion handler runs
    Then detect_annotations is called with ToolCallInfo from the turn
    And the resulting annotations are serialized into the persisted message metadata
    And the annotations include FileModification with the file path and Created operation

  @annotation-detection
  Scenario: Per-turn annotation detection captures fspec milestones
    Given the stream loop has completed processing a turn with tool calls
    And the turn includes a successful Fspec tool call with command update-work-unit-status
    When the turn completion handler runs
    Then detect_annotations returns a FspecMilestone annotation
    And the annotation is serialized into the persisted message metadata

  @annotation-detection
  Scenario: Per-turn annotation detection captures error resolution transitions
    Given the previous turn had a failed Bash tool call
    And the current turn has an Edit tool call followed by a successful Bash call
    When the turn completion handler runs
    Then detect_annotations returns an ErrorResolution annotation
    And the annotation references the failed tool and resolved file

  @compaction-hook-unchanged
  Scenario: CompactionHook threshold detection logic remains unchanged
    Given the CompactionHook in compaction_hook.rs
    When context usage exceeds the 85-90 percent threshold
    Then the hook sets compaction_needed to true on the TokenState
    And no changes have been made to the threshold detection logic itself
    And only the downstream response to the compaction_needed flag has changed
