@wip
@CMPCT-011
Feature: In-View DAG Construction Compaction Flow
  """
  execute_compaction() in interactive_helpers.rs lines 170-303 is rewritten in-place. The new flow: (1) set compaction_in_progress=true on BackgroundSession, (2) call clear_history(), (3) inject compaction system instruction as Message::User, (4) return Ok with a sentinel indicating 'agent takes over'. The old LLM prompt creation, turn conversion, compactor.compact(), and kept-turns reconstruction are all removed.
  stream_loop.rs post-turn logic: After each completed turn, call annotation_detector::detect(turn_tool_calls, previous_turn_state) to get Vec<StructuralAnnotation>. Annotations are serialized into the persisted message metadata via persist_assistant_message(). The stream_loop already tracks tool calls — annotations piggyback on existing data flow.
  The compaction system instruction is a static const string in interactive_helpers.rs (or a new compaction_instruction.rs). It must be concise (<500 tokens) since it consumes context during rebuild. It guides the agent to: (1) use SessionSearch strategically, (2) build D0/D1/D2 depth-level DAG with [SessionSearch: turns X-Y] references, (3) call inject_summary.
  execute_compaction() needs access to BackgroundSession's compaction_in_progress Arc<AtomicBool>. Currently it takes &mut Session (codelet_cli::session::Session). The function signature must be updated to also accept the compaction_in_progress flag, or the compaction trigger in stream_loop.rs must set the flag before calling execute_compaction().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. execute_compaction() must NOT make any separate LLM calls — zero marginal token cost
  #   2. execute_compaction() must set compaction_in_progress flag to true BEFORE clearing context
  #   3. execute_compaction() must call clear_history() to clear in-memory context while preserving on-disk persistence
  #   4. After clear, a compaction system instruction must be injected as the next user message to guide DAG construction
  #   5. The compaction system instruction must tell the agent to: search via SessionSearch, build a D0/D1/D2 hierarchical DAG, call inject_summary
  #   6. execute_compaction() must return control to the agent loop after injecting the system instruction — agent takes over from there
  #   7. Post-compaction context must contain ONLY: system reminders + injected DAG (via inject_summary handler from CMPCT-009)
  #   8. Per-turn annotation detection must inspect completed turns for: fspec tool calls → FspecMilestone, error→success transitions → ErrorResolution, Write/Edit calls → FileModification
  #   9. Per-turn annotations must be zero-cost inline detection — no LLM calls, no external process, just pattern matching on tool call metadata
  #   10. Annotations must be attached to turn metadata in the persisted message store so SessionSearch can surface them during DAG construction
  #   11. Wall-clock compaction time must be <5 seconds (the only latency is /clear + system instruction injection, no LLM wait)
  #   12. The annotation_detector module must live in codelet/core/src/compaction/annotation_detector.rs and use StructuralAnnotation from CMPCT-007
  #   13. session_compact() NAPI binding must trigger the same in-view flow as execute_compaction() — not the batch LLM pipeline
  #   14. No watchdog needed in this card. If the agent fails to call inject_summary, context keeps growing until the emergency threshold (CMPCT-012) fires again. CMPCT-012 is the explicit safety net for this case. The compaction_in_progress flag stays true, keeping SessionSearch trimmed, which is benign.
  #   15. The inject_summary handler (CMPCT-009) should clear the flag. This is per the CMPCT-009 description: 'handler will be extended to clear that flag after injection completes'. This ensures the flag is cleared atomically with the injection — no window where flag is true but DAG is already pinned. We'll modify the CMPCT-009 handler to accept the Arc<AtomicBool> and clear it.
  #
  # EXAMPLES:
  #   1. Session has 300 messages. Compaction triggers. execute_compaction() sets compaction_in_progress=true, calls clear_history() (context now has only system reminders), injects compaction system instruction as user message, returns control to agent loop. Wall-clock: <1 second. No LLM calls made.
  #   2. Agent receives compaction system instruction. Agent calls SessionSearch(show, max_turns: 10) for recent context, SessionSearch(search, query: 'decision|architecture') for decisions. Agent writes D0/D1/D2 DAG. Agent calls inject_summary(content). Context now contains system reminders + pinned DAG. Agent continues working on original task.
  #   3. Agent calls Fspec(update-work-unit-status, {_: ['AUTH-001', 'implementing']}). Per-turn detector sees fspec tool call, creates FspecMilestone{command: 'update-work-unit-status', args: ['AUTH-001', 'implementing']} annotation on the persisted turn.
  #   4. Agent's Bash tool call fails with exit code 1 ('cargo test' failure). Next turn, agent calls Edit to fix the file, then Bash succeeds with exit code 0. Per-turn detector creates ErrorResolution{failed_tool: 'Bash', resolved_file: 'src/main.rs'} annotation.
  #   5. Agent calls Write tool to create src/auth/handler.rs. Per-turn detector creates FileModification{path: 'src/auth/handler.rs', operation: FileOp::Created} annotation on the turn.
  #   6. During compaction, agent calls SessionSearch(show). Because compaction_in_progress is true (CMPCT-010), results are trimmed — Read tool outputs show '[file: path, N lines]' instead of full content, giving the agent a compact overview for DAG construction.
  #   7. Agent calls inject_summary(content) with DAG. inject_summary handler (CMPCT-009) partitions→clears→restores system reminders→injects DAG, clears compaction_in_progress flag. Subsequent SessionSearch calls return full untrimmed content again.
  #   8. Turn has only Read and Grep tool calls, no fspec commands, no errors. Annotation detector produces only FileModification annotations (if any Write/Edit present) — never fabricates annotations for non-matching patterns.
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should the stream_loop handle the case where the agent fails to call inject_summary after receiving the compaction instruction? (e.g., agent ignores instruction, or gets stuck in a loop). Should there be a turn limit or watchdog?
  #   A: No watchdog needed in this card. If the agent fails to call inject_summary, context keeps growing until the emergency threshold (CMPCT-012) fires again. CMPCT-012 is the explicit safety net for this case. The compaction_in_progress flag stays true, keeping SessionSearch trimmed, which is benign.
  #
  #   Q: Should the inject_summary handler (CMPCT-009) clear the compaction_in_progress flag, or should execute_compaction() clear it after inject_summary returns? The CMPCT-009 description says 'future integration note: handler will be extended to clear flag' — confirming handler clears it.
  #   A: The inject_summary handler (CMPCT-009) should clear the flag. This is per the CMPCT-009 description: 'handler will be extended to clear that flag after injection completes'. This ensures the flag is cleared atomically with the injection — no window where flag is true but DAG is already pinned. We'll modify the CMPCT-009 handler to accept the Arc<AtomicBool> and clear it.
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to have my context compacted via in-view DAG construction instead of batch LLM calls
    So that compaction costs zero marginal LLM tokens, completes in <5 seconds, and I retain full judgment over what to keep

  @compaction-flow
  Scenario: execute_compaction sets flag, clears context, and injects system instruction
    Given a session with 300 messages and compaction_in_progress flag set to false
    When compaction is triggered and execute_compaction is called
    Then the compaction_in_progress flag should be set to true before clearing
    And in-memory session state should be cleared and system reminders restored
    And a compaction system instruction should be injected as the next user message
    And execute_compaction should return control to the agent loop
    And no separate LLM calls should be made during compaction
    And wall-clock compaction time should be less than 5 seconds

  @compaction-flow
  Scenario: execute_compaction makes zero LLM calls
    Given the old execute_compaction that creates LLM prompt functions and calls compactor.compact
    When the rewritten execute_compaction is called
    Then no ProviderManager or LLM prompt functions should be created
    And no compactor.compact call should be made
    And no summarization budget should be calculated

  @compaction-instruction
  Scenario: Compaction system instruction guides agent through DAG construction
    Given a session where compaction has just cleared the context
    When the compaction system instruction is injected as a user message
    Then the instruction should tell the agent to search via SessionSearch
    And the instruction should specify D0, D1, and D2 depth levels for the DAG
    And the instruction should tell the agent to call inject_summary with the complete DAG
    And the instruction should be concise and under 500 tokens

  @compaction-flow
  @integration
  Scenario: Agent builds DAG via SessionSearch and calls inject_summary
    Given a session where compaction system instruction has been injected
    And compaction_in_progress flag is true
    When the agent calls SessionSearch to retrieve session history
    Then SessionSearch results should be trimmed via Layer 0 trimming
    And the agent can build a hierarchical D0/D1/D2 DAG from trimmed results
    And the agent can call inject_summary with the DAG content
    And after inject_summary the context contains only system reminders and the pinned DAG

  @compaction-flow
  @integration
  Scenario: inject_summary handler clears compaction_in_progress flag
    Given a session with compaction_in_progress flag set to true
    When the agent calls inject_summary with DAG content
    Then the inject_summary handler should clear the compaction_in_progress flag
    And subsequent SessionSearch calls should return full untrimmed content

  @napi-integration
  Scenario: session_compact NAPI binding triggers in-view flow
    Given a session accessible via the session_compact NAPI binding
    When session_compact is called
    Then the same in-view compaction flow as execute_compaction should be triggered
    And no batch LLM pipeline should be used
    And the compaction_in_progress flag should be set to true
