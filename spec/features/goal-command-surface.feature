@done
@tui
@cli
@codelet
@completion
@slash-commands
@CONT-003
Feature: Goal Command Surface
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

  Scenario: Setting a goal arms Goal mode even when auto-continue is off
    Given a session with auto-continue off
    When the user enters "/goal make all tests pass"
    Then the goal is set to "make all tests pass"
    And the printed state confirms the goal is active
    And the done rejection count and nudge count are reset

  Scenario: Bare /goal shows the contract state
    Given a session with an active goal and a verify command configured
    When the user enters "/goal"
    Then the state output shows the goal text
    And the state output shows the verify command
    And the state output shows the effective budget, nudges used, and rejections

  Scenario: Bare /goal without an active goal reports no goal set
    Given a session with no active goal
    When the user enters "/goal"
    Then the output reports that no goal is set

  Scenario: /goal verify attaches a verify command to the active goal
    Given a session with an active goal and no verify command
    When the user enters "/goal verify cargo test"
    Then the goal's verify command is "cargo test"
    And the printed state confirms the verify command

  Scenario: /goal verify without an active goal is an error
    Given a session with no active goal
    When the user enters "/goal verify cargo test"
    Then the command errors telling the user to set a goal first
    And no goal state is changed

  Scenario: /goal clear drops the goal and prints the fallback state
    Given a session with an active goal and auto-continue on
    When the user enters "/goal clear"
    Then the goal is cleared
    And the printed state shows the fallback to auto-continue

  Scenario: /goal clear without an active goal reports no goal set
    Given a session with no active goal
    When the user enters "/goal clear"
    Then the output reports that no goal is set
    And no state is changed

  Scenario: /goal with replacement text replaces the goal and resets counters
    Given a session with an active goal and recorded rejections and nudges
    When the user enters "/goal ship the release"
    Then the goal text becomes "ship the release"
    And the done rejection count and nudge count are reset

  Scenario: /continue off is refused while a goal is active
    Given a session with an active goal and auto-continue armed
    When the user enters "/continue off"
    Then the command is refused with the message to clear the goal first
    And the continue toggle and budget are unchanged

  Scenario: /continue off works normally when no goal is active
    Given a session with no active goal and auto-continue on
    When the user enters "/continue off"
    Then auto-continue turns off

  Scenario: Status indicator shows the goal marker while a goal is active
    Given a session with an active goal and nudge accounting
    When the status indicator is computed
    Then it shows the goal indicator with nudges used over the effective budget
    And it replaces the auto-continue indicator
    And after the goal is cleared with auto-continue on the auto-continue indicator returns
