@done
@security
@session
@rust
@bug-fix
@BUG-631
Feature: pause_resume resolves a three-choice approval as approve
  """
  answer_pause (sessions background_session/interactions.rs) took any PauseResponse for any pending pause and checked only question vs approval, so pause_resume (Resumed) answered a Triple approval, and the blocklist, RLCD gate and RLCD security prompt treat Resumed as allow. Fix: answer_pause takes a PauseReply {Resume, Confirm(accept), Triple(ApprovalChoice)} naming the reply action, checks it against the pending PauseState.kind inside the InteractionSlot respond check (under the slot lock, before anything is sent), and only then converts it to the PauseResponse the tool receives. Accepted: Resume for Continue; Confirm for Confirm and Continue (the wire shows Continue as Confirm); Triple for Triple. The bridge pause_response maps allow_once/allow_session/deny to PauseReply::Triple. The core stub handle applies the same table to the wire kinds (route kind HumanAction for resume).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each reply action answers exactly one pause kind: pause_resume answers a human action (PauseKind::Continue: WebSearch 'interact with the page', /goal 'human review needed'); pause_confirm answers a two-choice confirm as the wire shows it (PauseKind::Confirm, and a human action, which pause_state_to_rpc shows as Confirm and the TUI answers with y/n); pause_triple and the bridge pause_response (allow_once / allow_session / deny) answer a three-choice approval (PauseKind::Triple) only
  #   2. A reply action sent to a pause of another kind is rejected with an error naming the request id, the pause kind and the reply; it answers nothing, the tool keeps waiting and the request stays pending under the same id
  #   3. The kind check is in the one checked reply (BackgroundSession::answer_pause), so every caller (RPC, TUI websocket and embedded, client MCP, bridge) gets it; the in-process stub handle checks the same kinds
  #
  # EXAMPLES:
  #   1. A blocklist three-choice approval is pending; pause_resume names its id; it is rejected, the Bash command is not run and the approval stays pending
  #   2. A blocklist three-choice approval is pending; pause_confirm(accept=true) names its id; it is rejected and the approval stays pending; pause_triple(approve) then answers it with AllowOnce
  #   3. A WebSearch human action is pending; pause_triple(approve) names its id; it is rejected; pause_resume answers it with Resumed
  #   4. A WebSearch human action is pending; pause_confirm(accept=true) answers it (the TUI's y on the prompt the wire shows as Confirm)
  #   5. A tool Confirm pause is pending; pause_resume and pause_triple are rejected; pause_confirm(false) answers it with Denied
  #   6. A bridge pause_response allow_once naming a human action's id is rejected and the human action stays pending
  #
  # ASSUMPTIONS:
  #   1. pause_confirm answering a human action is kept because the wire PauseKind has no Continue (rpc-types session_state.rs: Continue intentionally omitted), so the TUI shows a human action as a y/n Confirm prompt and answers it with pause_confirm; RPC-406 locks the TUI's prompt keys out of PauseResumed. Derived from the code, not a product-owner decision.
  #
  # ========================================
  Background: User Story
    As a user or holder answering a tool pause
    I want to have each reply action answer only the pause kind it was made for
    So that a resume, a two-choice answer or a three-choice answer sent to the wrong prompt never approves a tool by accident

  Scenario: A human-action reply sent to a three-choice approval is rejected
    Given a session waiting on a blocklist three-choice approval for "echo bug631 git reset --hard"
    When pause_human_action names that approval's request id
    Then it is rejected with an error naming the request id, the three-choice approval and pause_human_action
    And the tool is still waiting and the approval is still pending under the same id

  Scenario: Every reply action is accepted only for the pause kind it answers
    Given a human action, a two-choice confirm and a three-choice approval, each pending on its own session
    When pause_human_action done, pause_confirm accept and deny, and pause_triple approve, approve for session and deny are each sent to each of them
    Then pause_human_action is accepted only by the human action, pause_confirm only by the two-choice confirm, and pause_triple only by the three-choice approval
    And each accepted reply gives the tool the response it names, and each rejected reply leaves the request pending under the same id with its tool still waiting

  Scenario: The right reply still answers a request after a wrong one was rejected
    Given a session waiting on a blocklist three-choice approval
    When pause_confirm with accept true names its request id and then pause_triple with approve names the same id
    Then the pause_confirm is rejected and the tool receives allow once from the pause_triple

  Scenario: A bridge pause_response answers only a three-choice approval
    Given a session waiting on a WebSearch human action
    When the bridge sends pause_response "allow_once" naming its request id
    Then the human action is still pending under the same id and its tool is still waiting

  Scenario: The stub handle checks the same pause kinds
    Given the in-process stub handle with a pending three-choice approval and a pending human action
    When pause_human_action names the approval's id and pause_triple names the human action's id
    Then both are rejected and both requests are still pending
