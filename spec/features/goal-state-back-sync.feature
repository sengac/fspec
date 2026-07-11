@done
@session
@completion
@codelet
@cli
@tui
@CONT-008
Feature: Goal state back-sync to chrome: stale bar indicator and satisfied-goal resurrection
  """
  VERIFIED sync topology: chrome→inner one-way at dispatch only (agent-loop/src/agent_loop.rs:495-530); inner→chrome write-back does NOT exist. Engine acceptance clears inner goal (goal.rs:81-84 → session/mod.rs:235-242) + registry (stream_loop.rs:1519) but BackgroundSession.goal_state (background_session.rs:397-406) keeps the satisfied goal → RESURRECTION: next dispatch sees chrome_goal != inner (None) and re-applies it via inner_session.set_goal (agent_loop.rs:508-518, set at :515). Fix = write-back on engine transitions (BackgroundSession::set_goal_state) + pushed state chunk consumed by the TUI to clear chrome_state goal; consider a generation stamp on goal state to make sync direction unambiguous. Also: TUI goal_parser.rs:88,:98 hard-codes 'nudges used: 0, rejections: 0' in bare /goal display — must read real state. Shares the push chunk with CONT-007; hang write-back off the shared teardown helper from CONT-006 so early-exit and settle paths both propagate.
  MECHANISM (reconciled against post-CONT-006/007/009 code): (1) apply_finish_with_summary (done_early_exit.rs) picks the snapshot reason BEFORE the branch — GoalSatisfied when a goal is being cleared, DoneAccepted otherwise; the existing emit at the tail is the single emission for both exit sites. (2) Both BackgroundOutput twins (agent-loop/src/background_output.rs ContinueState arm; napi/src/agent_loop.rs mirror) call the NEW shared helper BackgroundSession::clear_goal_state_if_unchanged_since_sync() on GoalSatisfied, then map goalCleared + doneRejections into ContinueStateInfo. (3) BackgroundSession gains goal_generation + goal_synced_generation AtomicU64s: set_goal_state bumps generation; sync_completion_contract_for_user_turn applies chrome→inner ONLY on generation change (stores synced gen after); the write-back helper clears chrome ONLY when generation == synced generation, so a mid-turn /goal replacement survives. Registry arming continues to read the INNER goal after the guarded apply. (4) TUI: dispatch_stream_chunks folds doneRejections into ContinueLiveState and clears the goal cache on goalCleared; goal_parser::apply_goal_subcommand gains live counters for the Show arm; footer logic unchanged (live goal_active already drives 🎯). JSON serializers agent-loop/src/stream_chunk_json.rs + napi/src/types.rs gain the two fields. rpc-types fields use #[serde(default)] for backward compat
  """

  Background: User Story
    As a codelet TUI/NAPI surface user with an active /goal
    I want to have the engine's goal transitions (satisfied via accepted done()) propagate back to the chrome state and the status bar
    So that the bar never shows a stale 🎯 for a satisfied goal, the next message cannot resurrect the completed goal server-side, and bare /goal reports real counters

  Scenario: Goal-satisfied teardown emits the dedicated goal-cleared snapshot
    Given a session with an active goal and 4 nudges used
    When the shared teardown runs for an accepted done() summary
    Then the satisfied goal is announced before the counter snapshot
    And the snapshot carries the goal-satisfied reason with goalActive false and nudgesUsed 0
    And running the teardown without a goal emits the done-accepted reason instead

  Scenario: Goal-satisfied snapshot writes the chrome goal state back through the background output
    Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    When the background output maps a goal-satisfied counter snapshot
    Then the chrome goal state reads as no goal
    And the pushed chunk carries goalCleared true
    And mapping a non-goal done-accepted snapshot leaves a synced chrome goal untouched

  Scenario: A goal replaced mid-turn survives the goal-satisfied write-back
    Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    And the user has since replaced the chrome goal mid-turn
    When the background output maps a goal-satisfied counter snapshot
    Then the chrome goal state still holds the replacement goal
    And the next dispatch sync applies the replacement goal to the inner session

  Scenario: A satisfied goal is not resurrected on the next dispatched user message
    Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    And the engine accepted done() for the goal and the background output performed the write-back
    When the dispatch-site sync helper runs for the next real user message
    Then the inner session has no goal
    And the done() registry reports the session as disarmed with no goal spec
    And no CompletionContract reminder is re-injected into the inner session messages

  Scenario: The dispatch sync never re-applies a chrome goal the engine already consumed
    Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    And the engine cleared the inner goal without the chrome write-back landing
    When the dispatch-site sync helper runs for the next real user message
    Then the inner session still has no goal
    And setting a new chrome goal afterwards is applied by the following dispatch sync

  Scenario: The TUI drops the goal indicator when the engine clears the goal
    Given a TUI session whose footer shows the goal indicator from a live counter snapshot
    When a counter snapshot chunk with goalCleared true is dispatched for the session
    Then the cached goal state for the session is cleared
    And the painted footer no longer shows the goal indicator
    And bare /goal reports that no goal is set

  Scenario: Bare /goal reports the live nudge and rejection counters
    Given a TUI session with an active goal whose live snapshot reports 3 nudges used and 2 rejections
    When the user enters "/goal"
    Then the state output shows nudges used 3 and rejections 2
    And the state output no longer hard-codes zero counters

  Scenario: Escalation keeps the goal indicator on the bar
    Given a TUI session whose footer shows the goal indicator from a live counter snapshot
    When the engine raises a goal escalation through a registered pause handler
    Then the painted footer still shows the goal indicator
    And the escalation path emits no goal-cleared signal in the engine sources
    And the pause handler is invoked and no continue-state event reaches the output sink
    And the inner session goal remains set after the escalation

  Scenario: Wire chunk and twin serializers carry the goal-cleared flag and rejection count
    Given a ContinueStateUpdate chunk with goalCleared true and doneRejections 2
    When the chunk is serialized to JSON and deserialized back
    Then the JSON payload uses the camelCase field names goalCleared and doneRejections
    And a payload without the new fields still deserializes with false and zero defaults
    And both background twins and both twin JSON serializers carry the new fields and the shared write-back call
