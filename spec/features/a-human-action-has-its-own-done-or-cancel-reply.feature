@done
@security
@tui
@rpc
@session
@rust
@AMGR-023
Feature: A human-action request has its own reply (done or cancel), not the confirm reply
  """
  WIRE CHANGE (AMGR-023). rpc-types session_state.rs: PauseKind gains HumanAction (appended after Triple) and a new HumanActionReply {Done, Cancel}. sessions conversions: the internal PauseKind::Continue maps to the wire HumanAction (no longer Confirm). FspecService: pause_human_action(session_id, request_id, reply) is appended at the end of supervision_credentials.rs and to rpc/tests/fixtures/wire_order_frozen.txt; pause_resume keeps its declaration at its frozen index (RPC-449) but is removed everywhere else: its service implementation answers nothing and returns an error naming pause_human_action, and SessionManagerHandle, the stub handle, the client MCP catalog, reply_interaction_checked and the TUI backend drop it. BackgroundSession::answer_pause takes PauseReply::HumanAction(reply): Done gives the tool Resumed, Cancel gives Interrupted (the result an interrupt gives); PauseReply::Confirm answers PauseKind::Confirm only. The TUI human-action prompt (views/agent/pause_prompt.rs, pause_keys.rs) shows [Enter] Done [Esc] Cancel and sends Action::PauseHumanAction. The bridge control gains human_action_response (done | cancel) naming the request id.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A human action has its own reply on the wire, done or cancel, naming the request id; it answers a human action only, and every other reply type answers exactly one other request kind (product owner, 2026-11-08)
  #   2. pause_confirm answers a two-choice confirm only: it rejects a human action naming the request id, the pause kind and the action, and answers nothing (product owner, 2026-11-08)
  #   3. The wire pause kind tells a human action apart (a new PauseKind variant appended, not the Confirm a human action is shown as today), so every client knows which reply it takes; wire additions are appended, never reordered (RPC-449)
  #   4. The TUI, the bridge and the client MCP answer a human action with the human-action reply naming the request id: the TUI prompt offers Done and Cancel
  #   5. Yes (product owner, 2026-11-09): the new done/cancel reply replaces pause_resume. pause_resume is removed; its tarpc slot is kept frozen per RPC-449, so a human action has exactly one reply type.
  #   6. Product owner, 2026-11-09: cancel gives the waiting tool the same result as an interrupt: WebSearch stops without the screenshot and reports it was cancelled; the /goal 'human review needed' escalation ends while the goal stays active.
  #
  # EXAMPLES:
  #   1. A WebSearch human action 'Interact with the page, then press Enter to capture screenshot.' is pending with request id R; pause_human_action(R, Done) answers it; the tool receives Resumed and captures the screenshot; R is announced Answered by the user
  #   2. The same human action; pause_human_action(R, Cancel) answers it; the tool receives Interrupted, the result an interrupt gives: WebSearch stops without the screenshot and returns 'Operation interrupted by user'
  #   3. The /goal escalation 'human review needed' is pending; pause_human_action(R, Cancel) ends it with Interrupted; the goal is still set
  #   4. A human action is pending; pause_confirm(R, accept=true) is rejected naming R, the human action and pause_confirm; the human action stays pending
  #   5. A two-choice confirm and a three-choice approval are pending; pause_human_action(Done) naming either id is rejected and answers nothing
  #   6. get_pause_state for a pending WebSearch human action returns PauseKind::HumanAction (the appended wire variant); a two-choice confirm is still Confirm
  #   7. The FspecService still declares pause_resume at its frozen index; calling it answers nothing and errors naming pause_human_action; pause_human_action is appended last and in wire_order_frozen.txt
  #   8. The TUI shows a human action as '⏸ WebSearch: Interact with the page...' with '[Enter] Done [Esc] Cancel'; Enter sends pause_human_action(R, Done), Esc sends pause_human_action(R, Cancel); y and n do nothing
  #   9. The bridge sends human_action_response 'done' (or 'cancel') naming R; the human action is answered; a pause_response 'allow_once' naming R is rejected
  #   10. The client MCP catalog lists pause_human_action (session_id, request_id, reply Done|Cancel) and no pause_resume; a call naming R answers the human action
  #
  # QUESTIONS (ANSWERED):
  #   Q: Does the new done/cancel reply replace pause_resume, which today answers only a human action (done)? Proposal: yes, pause_resume is removed (its tarpc slot kept frozen per RPC-449, as get_interaction_snapshot was), so a human action has exactly one reply type.
  #   A: Yes (product owner, 2026-11-09): the new done/cancel reply replaces pause_resume. pause_resume is removed; its tarpc slot is kept frozen per RPC-449, so a human action has exactly one reply type.
  #
  #   Q: What does cancel do? Proposal: the waiting tool gets the same result as an interrupt: WebSearch stops without the screenshot and reports it was cancelled; the /goal 'human review needed' escalation ends (the goal stays active, as after done).
  #   A: Product owner, 2026-11-09: cancel gives the waiting tool the same result as an interrupt: WebSearch stops without the screenshot and reports it was cancelled; the /goal 'human review needed' escalation ends while the goal stays active.
  #
  # ========================================
  Background: User Story
    As a user or parent answering a sub-agent's human-action request
    I want to answer a human action (WebSearch 'interact with the page', /goal 'human review needed') with its own done or cancel reply
    So that no reply meant for another request kind can ever answer it

  Scenario: Done answers a human action and the tool goes on
    Given a session waiting on a WebSearch human action "Interact with the page, then press Enter to capture screenshot."
    When pause_human_action names its request id with Done
    Then the waiting tool receives Resumed and the session is Running
    And the request is announced Answered by the user

  Scenario: Cancel stops WebSearch with the result an interrupt gives
    Given a session whose WebSearch waits for the user to interact with the page before it captures the screenshot
    When pause_human_action names the human action's request id with Cancel
    Then the wait ends as an interrupt does, so WebSearch returns "Operation interrupted by user" and captures no screenshot

  Scenario: Cancel ends the goal escalation and the goal stays active
    Given a session with the goal "ship the release" whose "human review needed" escalation waits for the user
    When pause_human_action names the escalation's request id with Cancel
    Then the escalation returns Interrupted
    And the session's goal is still "ship the release"

  Scenario: pause_confirm does not answer a human action
    Given a session waiting on a WebSearch human action
    When pause_confirm with accept true names its request id
    Then it is rejected with an error naming the request id, the human action and pause_confirm
    And the tool is still waiting and the human action is still pending under the same id

  Scenario: pause_human_action answers a human action only
    Given a two-choice confirm and a three-choice approval, each pending on its own session
    When pause_human_action with Done names each request id
    Then both are rejected with an error naming pause_human_action
    And both requests are still pending under the same ids with their tools waiting

  Scenario: The wire shows a human action as its own pause kind
    Given a session waiting on a WebSearch human action and another waiting on a two-choice confirm
    When get_pause_state reads each session
    Then the human action's kind is HumanAction and the confirm's kind is Confirm

  Scenario: pause_resume keeps its frozen index and answers nothing
    Given the FspecService methods in declaration order and the frozen wire-order fixture
    When pause_resume and pause_human_action are looked up
    Then pause_resume keeps its frozen index and pause_human_action is the last method
    And a pause_resume call naming a pending human action's request id is rejected naming pause_human_action, and the human action is still pending

  Scenario: The TUI prompt answers a human action with Done or Cancel
    Given the TUI shows a session's pending WebSearch human action
    Then the prompt shows "[Enter] Done" and "[Esc] Cancel" and no Y/N choice
    When the user presses y, then Enter, and then Esc on the session's next human action
    Then y sends nothing, and the backend receives pause_human_action with Done for the first request id and Cancel for the second

  Scenario: The bridge answers a human action with done or cancel
    Given a session waiting on a WebSearch human action
    When the bridge sends pause_response "allow_once" and then human_action_response "cancel" naming its request id
    Then the pause_response answers nothing and the tool receives Interrupted from the human_action_response

  Scenario: The client MCP answers a human action by request id
    Given the client MCP catalog
    Then it lists pause_human_action with session_id, request_id and reply, and no pause_resume
    When pause_human_action is called naming a pending human action's request id with Done
    Then the human action is answered
