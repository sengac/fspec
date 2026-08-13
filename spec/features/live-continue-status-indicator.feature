@done
@tui
@cli
@codelet
@completion
@CONT-007
Feature: Live status-bar updates for auto-continue nudge counter (bar above input, not chat)
  """
  VERIFIED data flow: footer painted every frame (views/agent.rs:286 → chrome_paint.rs:92-119) but reads only the TUI-local cache (chrome_state.rs:55-92), written solely by /continue and /goal dispatch (dispatch_slash_continue.rs:50-54, dispatch_slash_goal.rs:50-51); nudges_used is a literal 0 at chrome_paint.rs:110. RPC getters get_continue_state/get_goal_state are plumbed end-to-end (handle_impl.rs:1352,1377 → background_session.rs:1191,1206; all three transports) but NEVER called by the TUI. Live n lives only on the inner CLI Session (stream_loop.rs:1468/:1547/:1525); BackgroundSession exports no nudge counter — polling cannot fix this without exporting the counter.
  CHOSEN DIRECTION (per codebase precedent — CompactionProgress, SupervisorPendingInjection, FooterStateUpdate are all push): new state-only StreamChunk variant (e.g. ContinueStateUpdate{enabled, budget, nudges_used, goal_active, effective_budget}) mapped in background_output.rs:216-219 + napi mirror (napi agent_loop.rs:1627-1630); emitted at every transition: nudge consume (stream_loop.rs:1547), refund (:1467-1471 — today invisible EVERYWHERE), turn reset (agent-loop agent_loop.rs:503), exhaustion/finish (:1525,:1531). Consume in session_context.rs state-only arm (:144-155) → chrome_state → footer. REMOVE the per-nudge chat print (stream_loop.rs:1556-1559); keep synthetic nudge user messages in chat (conversation content, :1575-1587) and terminal events (warning/done/satisfied) in chat. FIX: nudging display uses continue_budget, not the Goal effective budget (:1493 vs :1558). CLI repl untouched (no bar; StreamOutput split at output.rs:186).
  IMPLEMENTATION SHAPE (CONT-007): new cli module interactive/continue_state.rs owning ContinueStateReason + continue_state_event(session, reason) + emit_continue_state(session, output, reason) (effectiveBudget = effective_goal_budget when goal active, else continue_budget). StreamEvent::ContinueState(ContinueStateEvent) in output.rs; CliOutput renders '⏩ auto-continue: nudging (n/EFFECTIVE)' only for reason NudgeConsumed. rpc-types: ContinueStateInfo struct + StreamChunk::ContinueStateUpdate{continueState} + constructor. Emission sites: run_agent_stream_internal top (TurnStart), refund settle block, Nudge arm (replaces emit_status), FinishWithWarning arm (Exhausted), apply_finish_with_summary tail (DoneAccepted — covers both exit sites). TUI: session_context state-only arm; dispatch_stream_chunks writes set_continue_state + set_continue_live; chrome_state gains ContinueLiveState{nudges_used,effective_budget,goal_active} + accessors + clear; dispatch_slash_continue/goal call clear_continue_live; chrome_paint threads live values (agent_view.rs comment compressed to stay <300 LoC). One test file (workflow rule): rust/fspec-tui/tests/cont007_live_continue_status_test.rs — behavioral via codelet-cli/rpc-types deps + App MockBackend fixture + paint_footer buffer assertions; twin/napi/json-mapper/stream_loop wiring pinned by cont009-style source-shape scans.
  """

  Background: User Story
    As a TUI user running an agent with auto-continue or a /goal armed
    I want to see the nudge counter in the status bar above the input update live at every counter transition
    So that I can watch budget consumption without the chat transcript being spammed with per-nudge notifications

  Scenario: ContinueStateUpdate chunk round-trips and stays out of the transcript
    Given a StreamChunk::ContinueStateUpdate with enabled true, budget 10, nudgesUsed 3, goalActive false, effectiveBudget 10
    When the chunk is serialized to JSON and deserialized back
    Then the JSON payload uses the camelCase field names nudgesUsed, goalActive and effectiveBudget
    And the deserialized chunk carries the same field values
    And recording the chunk into a SessionContext adds zero scrollback chunks

  Scenario: Engine snapshot reports the effective goal budget
    Given a session with auto-continue enabled and budget 10 and 2 nudges used
    When the continue state event is built without an active goal
    Then the event reports enabled true, budget 10, nudgesUsed 2, goalActive false and effectiveBudget 10
    When a goal is set on the session and the event is rebuilt
    Then the event reports goalActive true and effectiveBudget 15
    When the budget is raised to 20 and the event is rebuilt
    Then the event reports effectiveBudget 20 and goalActive true

  Scenario: Accepted done() teardown emits a reset counter state from the shared teardown
    Given a session with 4 nudges used and an active goal
    When apply_finish_with_summary runs for an accepted done() summary
    Then a ContinueState event is emitted after the announcement with nudgesUsed 0 and goalActive false
    When the teardown runs again for a session without an active goal
    Then a ContinueState event is emitted with nudgesUsed 0

  Scenario: Footer paints the real nudge counter from a live update
    Given an App with an open session whose cached continue state is enabled with budget 10
    When a ContinueStateUpdate chunk with nudgesUsed 2 and effectiveBudget 10 is dispatched for the session
    Then the session scrollback gains no transcript chunk
    And the painted footer shows "⏩ auto-continue (2/10)"
    When a ContinueStateUpdate chunk with goalActive true, nudgesUsed 1 and effectiveBudget 15 is dispatched
    Then the painted footer shows "🎯 goal (1/15)"

  Scenario: Slash dispatch drops the stale live counter
    Given an App session whose live counter cache reports 7 nudges used
    When the user applies "/continue 20" through the continue dispatch
    Then the live counter cache for the session is cleared
    And the painted footer shows "⏩ auto-continue (0/20)"
    When a ContinueStateUpdate chunk repopulates the live counter and the user applies "/goal ship it" through the goal dispatch
    Then the /goal dispatch also drops the live counter state

  Scenario: Emission sites, twin mappings, and chat-print removal are pinned in the sources
    Given the codelet workspace sources
    Then stream_loop.rs emits the continue state at the turn start, refund settle, nudge and exhaustion sites
    And stream_loop.rs no longer emits the "⏩ auto-continue: nudging" status message
    And done_early_exit.rs emits the continue state at the tail of apply_finish_with_summary
    And both background twins map StreamEvent::ContinueState to StreamChunk::ContinueStateUpdate
    And both stream_chunk_to_json_value copies serialize the variant as type "continueStateUpdate"
    And CliOutput renders the nudging line from the ContinueState event using the effective budget
