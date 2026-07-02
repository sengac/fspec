@done
@RPC-408
@session
@pause-integration
@rust
@bug
Feature: HITL response answer mapping in SessionManagerHandle::send_hitl_response
  """
  Wire HitlRequest is single-question by design; option-label matching uses the pending request's first question. Internal→wire request conversion lives in handle_impl.rs::get_hitl_request (rpc-types HitlRequest{id,question,header,options,allow_text_input}).
  Parity reference: napi session_send_hitl_response (codelet/napi/src/session_bindings.rs:1720-1750) maps HitlResponseInfo answers to HashMap<id, HitlAnswer> and only sends Cancelled when the payload explicitly says cancelled:true. The wire path has no cancelled flag, so it must always produce Answered.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. send_hitl_response must map the wire HitlResponse{id,value} to internal HitlResponse::Answered — never Cancelled — and deliver it via session.send_hitl_response
  #   2. If value equals one of the pending HITL question's option labels, the answer is HitlAnswer{selected:[value], other:None}
  #   3. If value matches no option label (free text via allow_text_input), the answer is HitlAnswer{selected:[], other:Some(value)}
  #   4. Answers are keyed by the pending request's first question id; if response.id differs, the pending id is preferred and a tracing::warn! is logged
  #   5. If no pending HITL request is stored on the session, fall back to keying the answer by response.id and treating value as free text
  #   6. Semantics mirror the napi path (session_send_hitl_response): identical user actions produce identical Answered payloads on both frontends
  #
  # EXAMPLES:
  #   1. Pending question options [Yes,No], user picks Yes: blocked wait_for_hitl_response receives Answered{answers:{qid: {selected:["Yes"], other:None}}}
  #   2. User types free text "maybe later" (not an option label): tool receives Answered with selected=[] and other=Some("maybe later")
  #   3. TUI submits with a stale/mismatched question id: the answer is still delivered as Answered keyed by the pending question id, with a warning logged
  #   4. No matter what the user answers, the tool never sees Cancelled from send_hitl_response (source no longer contains the hard-coded Cancelled{cancelled:false})
  #
  # ASSUMPTIONS:
  #   1. Genuine cancel (Esc) sends nothing from the TUI today; Cancelled remains reserved for a future explicit cancel affordance — this fix never introduces a Cancelled send
  #
  # ========================================
  Background: User Story
    As a standalone Rust TUI user answering a request_user_input (HITL) dialog
    I want to have my selected option or typed free text delivered to the blocked request_user_input tool as an Answered response
    So that the agent actually receives my answer instead of a spurious cancellation

  Scenario: Selecting an option label delivers Answered with that label selected
    Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    And a thread is blocked on the session's wait_for_hitl_response
    When the handle receives send_hitl_response with id "confirm_choice" and value "Yes"
    Then the blocked thread receives an Answered response
    And the answer for question "confirm_choice" has selected equal to ["Yes"] and other equal to None

  Scenario: Typing free text delivers Answered with the text as other
    Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    And a thread is blocked on the session's wait_for_hitl_response
    When the handle receives send_hitl_response with id "confirm_choice" and value "maybe later"
    Then the blocked thread receives an Answered response
    And the answer for question "confirm_choice" has selected equal to [] and other equal to Some("maybe later")

  Scenario: Mismatched response id still answers keyed by the pending question id
    Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    And a thread is blocked on the session's wait_for_hitl_response
    When the handle receives send_hitl_response with id "stale_id" and value "No"
    Then the blocked thread receives an Answered response
    And the answer is keyed by "confirm_choice" and has selected equal to ["No"] and other equal to None

  Scenario: No pending HITL request falls back to the response id and free text
    Given a BackgroundSession with no pending HITL request stored
    And a thread is blocked on the session's wait_for_hitl_response
    When the handle receives send_hitl_response with id "orphan_question" and value "some answer"
    Then the blocked thread receives an Answered response
    And the answer for question "orphan_question" has selected equal to [] and other equal to Some("some answer")

  Scenario: Option label versus free text discrimination is the wire-path parity contract
    Given a BackgroundSession with a pending HITL request whose question "confirm_choice" has options "Yes" and "No" and allows text input
    When the handle receives send_hitl_response with value "Yes", then "No", then "anything else"
    Then value "Yes" maps to an Answered answer with selected equal to ["Yes"] and other equal to None
    And value "No" maps to an Answered answer with selected equal to ["No"] and other equal to None
    And value "anything else" maps to an Answered answer with selected equal to [] and other equal to Some("anything else")

  Scenario: send_hitl_response never delivers Cancelled
    Given the source of handle_impl.rs::send_hitl_response
    When the source of send_hitl_response is inspected
    Then it no longer contains the hard-coded Cancelled response
    And every response delivered from this path is an Answered variant
