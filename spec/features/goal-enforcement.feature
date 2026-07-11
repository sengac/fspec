@done
@codelet
@cli
@completion
@streaming
@session
@CONT-003
Feature: Goal Enforcement: conditional done() acceptance against a user-set goal
  """
  VERIFIED system_reminders.rs: SystemReminderType enum at line 25 (doc said 23), SYSTEM_REMINDER_TAG consts at 67-68, partition_for_compaction at 208 (keeps only LATEST per type). DISCREPANCY: doc §6 claims 'retain-based removal in add_system_reminder dedup logic supports replacement' — FALSE. add_system_reminder (system_reminders.rs:328-346) is append-only with supersession markers (CLI-013 prompt-cache design); there is NO removal API. Adding remove_system_reminders_of_type(messages, type) is in scope for /goal clear + accepted done(). Session::add_system_reminder is at session/mod.rs:186 (doc said 169-172).
  ESCALATION SURFACING (decision): reuse codelet_tools::tool_pause — per-session PauseHandler registry (tool_pause.rs:67-91). pause_for_user(session_id, PauseRequest{kind: Continue, tool_name: "goal", message, details}) blocks via the handler that sets SessionStatus::Paused (agent_loop.rs:511-529, napi/agent_loop.rs:608-625 register it; TUI surfaces Paused state). In plain CLI repl NO handler is registered → pause_for_user returns PauseResponse::Resumed immediately, so the stream loop first emits the prominent blocked message ('🎯 goal: … — human review needed') then calls pause_for_user; both surfaces get correct behavior from one code path. Goal stays active after escalation.
  DYNAMIC done() DESCRIPTION (decision): extend the CONT-002 registry in tools/src/done.rs — replace CONTINUE_ARMED: RwLock<HashSet<Uuid>> with RwLock<HashMap<Uuid, ContractState>> where ContractState{armed: bool, goal_text: Option<String>, verify: Option<String>}. Add set_session_goal(session_id, Option<GoalSpec>) + get_session_goal(session_id) synced at the same dispatch sites as set_continue_armed (agent_runner.rs:39-43, agent_loop.rs:495-505). DoneTool::definition() (done.rs:123) already runs per-prompt (agents are rebuilt each user turn via create_rig_agent) — it reads the registry and appends 'The current goal is: <text>. You must not call done() unless this goal is met; provide evidence and goal_assessment.' Tier 1/2 checks live in DoneTool::call() (done.rs:155) reading the same registry; verify execution uses std::process::Command + wait_timeout-style bounded polling (unified_exec uses tokio kill_on_drop; done() call is async so tokio::time::timeout + tokio::process::Command is available and preferred).
  SESSION STATE + ENGINE: Session (cli/src/session/mod.rs:31-73) gains goal: Option<SessionGoal>{text, verify: Option<String>, set_at} + done_rejections: u32. auto_continue.rs decide_continuation gains Goal-mode branch (or a sibling decide_goal_continuation): inputs add goal_active, done_rejections, consecutive_zero_activity_nudges; new ContinueDecision::Escalate(String) variant consumed at the single settle point stream_loop.rs:1439-1557 (CONT-002 decision block). Effective budget resolution max(explicit,15) computed where budget is read, not stored. /goal command surface mirrors CONT-002: cli repl_loop.rs handler before catch-all (:136-161 precedent); TUI goal parsing added to slash_parser.rs + a goal_parser.rs mirror (continue_parser.rs precedent, 147 lines) + dispatch via dispatch_slash_continue.rs pattern; footer indicator via continue_status_indicator precedent (chrome_paint.rs:105-113, footer.rs:54,76). /continue-off refusal requires apply_continue_command to learn goal_active: bool input (CLI + TUI mirror both).
  VERIFIED remaining doc claims: DoneArgs (done.rs:86-96) already has optional evidence: Vec<String> + goal_assessment: String fields (CONT-002 forward-compat) — Tier 1 only adds validation, no schema change. Per-turn reminder injection precedent confirmed at agent-loop/src/agent_loop.rs:298-306 (user_prompt_submit additional_context → add_system_reminder). stream_loop.rs settle point confirmed at 1439-1557 with take_done_acceptance at 1464; done acceptance must widen from Option<String> summary to a rejected/accepted outcome (rejections are tool-level errors, so the stream loop only ever sees accepted summaries — rejection counting lives in the done.rs registry, read at the settle point for the >=4 threshold). RPC chrome precedent for goal state: set_continue_state/get_continue_state in background_session.rs (AtomicBool+AtomicU32) — goal text needs a Mutex<Option<String>> analogue. Feature split per CONT-002 precedent: goal-enforcement.feature (engine: tiers, escalation, budget, persistence) + goal-command-surface.feature (/goal grammar, /continue-off refusal, indicator).
  """

  Background: User Story
    As a codelet agent-loop user (TUI or CLI repl)
    I want to set a goal with /goal so that done() is only accepted when the goal is met (with optional deterministic verification)
    So that the agent cannot silently claim completion — it must provide evidence, pass verification, or escalate to me for review

  Scenario: Goal presence wins the derived mode over the continue toggle
    Given a session with auto-continue off
    When a goal "make all tests pass" is set on the session
    Then the effective mode is Goal
    And when the goal is cleared with auto-continue on the effective mode is AutoContinue
    And when the goal is cleared with auto-continue off the effective mode is Off

  Scenario: done() without evidence or goal assessment is rejected at Tier 1
    Given a session with an active goal "make all tests pass"
    When the model calls done with only a summary
    Then the done call is rejected as a failed tool result
    And the rejection message contains the goal text "make all tests pass"
    And the rejection message instructs the model to provide evidence and a goal_assessment

  Scenario: done() with a trivial goal assessment is rejected at Tier 1
    Given a session with an active goal "make all tests pass"
    When the model calls done with evidence and a goal_assessment shorter than 20 characters
    Then the done call is rejected as a failed tool result

  Scenario: done() with evidence and assessment is accepted when no verify command is configured
    Given a session with an active goal "make all tests pass" and no verify command
    When the model calls done with a summary, non-empty evidence, and a substantive goal_assessment
    Then the done call is accepted
    And the acceptance is recorded for the session

  Scenario: Failing verify command rejects done() with exit code and output tail
    Given a session with an active goal and verify command "false"
    When the model calls done with valid Tier 1 arguments
    Then the done call is rejected as a failed tool result
    And the rejection message reports the verification exit code

  Scenario: Verify command exit code is surfaced in the rejection
    Given a session with an active goal and verify command "sh -c 'echo boom; exit 3'"
    When the model calls done with valid Tier 1 arguments
    Then the rejection message contains exit code 3
    And the rejection message contains the verification output tail "boom"

  Scenario: Passing verify command accepts done() and auto-clears the goal
    Given a session with an active goal and verify command "true"
    When the model calls done with valid Tier 1 arguments
    Then the done call is accepted
    And the user sees the goal satisfied announcement with the summary
    And the goal is auto-cleared falling back to the continue toggle
    And the done rejection count is reset

  Scenario: Verify command exceeding the timeout rejects done() with a timeout message
    Given a session with an active goal and a verify command that sleeps past a bounded timeout
    When the model calls done with valid Tier 1 arguments
    Then the done call is rejected as a failed tool result
    And the rejection message reports a verification timeout

  Scenario: Fourth done() rejection escalates while the goal stays active
    Given a session with an active goal and 3 recorded done rejections
    When the model's done call is rejected a 4th time and the stream settles
    Then the engine decides to escalate for human review
    And the escalation message says the model repeatedly claims completion but verification fails
    And the goal remains active

  Scenario: Budget exhaustion in Goal mode escalates instead of the AutoContinue warning finish
    Given a session in Goal mode with all zero-progress nudges consumed
    When the model stops cleanly without calling done and the stream settles
    Then the engine decides to escalate for human review
    And the AutoContinue budget-exhaustion warning is not used

  Scenario: Two consecutive zero-activity nudges escalate immediately
    Given a session in Goal mode with remaining nudge budget
    When two consecutive nudged segments produce no tool calls and no done call
    Then the engine decides to escalate immediately without burning the remaining budget

  Scenario: Escalation pauses the session when a pause handler is registered
    Given a session with a registered pause handler
    When a goal escalation is raised for the session
    Then the pause handler receives the goal escalation request
    And the turn finishes after the pause resolves

  Scenario: Escalation in plain CLI repl finishes the turn with a prominent blocked message
    Given a session with no registered pause handler
    When a goal escalation is raised for the session
    Then pause resolution returns immediately
    And the turn finishes with the prominent blocked message

  Scenario: Larger explicit continue budget overrides the Goal default of 15
    Given a session where the user set an explicit continue budget of 40
    When a goal is set on the session
    Then the effective Goal-mode budget is 40

  Scenario: Goal default budget of 15 overrides a smaller explicit continue budget
    Given a session where the user set an explicit continue budget of 5
    When a goal is set on the session
    Then the effective Goal-mode budget is 15

  Scenario: Goal text survives compaction via the CompletionContract system reminder
    Given a session with an active goal injected as a CompletionContract system reminder
    When the conversation is partitioned for compaction
    Then the CompletionContract reminder is preserved as the latest of its type
    And the preserved reminder contains the goal text

  Scenario: Clearing the goal removes the CompletionContract system reminder
    Given a session with an active goal injected as a CompletionContract system reminder
    When the CompletionContract reminders are removed from the conversation
    Then no CompletionContract reminder remains in the conversation
    And other system reminder types are untouched

  Scenario: done() tool description includes the goal text while a goal is active
    Given a session with an active goal "make all tests pass"
    When the done tool definition is built for the session
    Then the tool description contains "The current goal is: make all tests pass"
    And after the goal is cleared the tool description no longer mentions a goal
