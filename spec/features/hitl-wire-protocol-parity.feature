@done
@rust
@pause-integration
@session
@RPC-410
Feature: HITL wire protocol parity: multi-question requests and cancel-capable structured responses

  """
  Dossier §3.1: codelet/rpc-types/src/lib.rs replaces single-question wire shapes with TS-parity shapes — HitlQuestion{id,header,question,options:Vec<HitlOption>}, HitlRequest{questions:Vec<HitlQuestion>}, HitlAnswer{id,selected:Vec<String>,other:Option<String>}, HitlResponse{cancelled:bool,answers:Vec<HitlAnswer>}. allow_text_input is DROPPED; serde derives match sibling wire types.
  Dossier §3.2/§3.3: handle_impl.rs get_hitl_request becomes full pass-through (every question, options None→[], delete the RPC-053 TODO); send_hitl_response becomes direct mapping (cancelled→Cancelled{true}, else Answered with HashMap keyed by answer id; tracing::warn on unknown ids is allowed but mapping must not depend on the pending request). handle_impl.rs is already >300 lines — mapping helpers extracted to a new sessions/src/hitl_mapping.rs module.
  Dossier §3.4: fspec-tui transport trait (mod.rs), websocket.rs, embedded.rs and test mock backends update to the new shapes; hitl_dialog.rs + dispatch_pause_hitl.rs get MINIMAL mechanical fixes (render questions[0], submit builds HitlResponse{cancelled:false, answers:[one]}, Esc unchanged). Out of scope: any UX change (RPC-411) and napi redesign (legacy path keeps tools-crate internal types; verify it compiles). Old tests pinning the RPC-408 heuristic (rpc408_hitl_response_answer_mapping.rs) and old wire-shape assertions (rpc036_widen_types.rs, rpc037 parity, pause_hitl_rpc053) are rewritten mechanically. hitl-response-answer-mapping.feature scenarios are superseded by this card's feature.
  HANG-SAFETY: all blocking-wait tests follow paused_chunk_delivery_rpc409.rs module rules — unblock the waiter BEFORE any assertion, bound every join/recv with a timeout
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The wire HitlRequest carries the FULL questions array (1-3 questions) in order; no first-question slicing
  #   2. A wire HitlQuestion is {id, header, question, options: Vec<HitlOption>}; an internal question with options: None surfaces options: [] — freeform capability is derived (no allow_text_input flag on the wire)
  #   3. The wire HitlResponse is {cancelled: bool, answers: Vec<HitlAnswer{id, selected, other}>}; cancelled:true maps to internal Cancelled{cancelled:true} delivered to the blocked tool
  #   4. A non-cancelled wire response maps pass-through to internal Answered: the answers vec becomes a HashMap keyed by each answer's id, preserving selected/other EXACTLY with no label inference and no reading of the pending request to classify answers
  #   5. Freeform text identical to an option label stays in other (the RPC-408 label-inference heuristic is deleted; send path contains no option-label comparison)
  #   6. The existing HitlDialog and transports keep compiling with minimal mechanical changes: first-question rendering and single-answer submit as HitlResponse{cancelled:false, answers:[one]}; Esc behavior and all UX unchanged (RPC-411 replaces the dialog)
  #
  # EXAMPLES:
  #   1. An internal request with 3 questions (approach, priority, notes) surfaces a wire HitlRequest whose questions array has exactly those 3 ids in order with headers, question text, and option labels preserved
  #   2. An internal question with options: None surfaces on the wire with options: [] so the frontend derives a pure-freeform question
  #   3. The user presses Esc: the wire response {cancelled:true, answers:[]} reaches the blocked tool as Cancelled{cancelled:true} and the agent receives the cancellation JSON
  #   4. A wire response with answers [{id:approach, selected:[Option A]}, {id:notes, other:free text}] reaches the tool as Answered with a 2-entry map keyed by approach and notes preserving selected/other exactly
  #   5. The user types freeform text "Yes" that is identical to an option label: the tool still receives it as other:Some("Yes") with selected:[] — no misclassification
  #   6. The send path source in handle_impl.rs contains no option-label comparison (source-shape check that the RPC-408 heuristic is gone)
  #   7. The new wire shapes (HitlQuestion, multi-question HitlRequest, HitlAnswer, cancel-capable HitlResponse) round-trip through serde_json equal to the original value
  #
  # ========================================

  Background: User Story
    As a AI agent using the request_user_input tool
    I want to have my full multi-question HITL request delivered to the frontend and receive a structured cancel-capable response with exact selected/other answers
    So that I never lose questions 2-3 and never receive misclassified or spuriously-answered responses

  Scenario: A three-question internal request surfaces all questions over the wire in order
    Given a BackgroundSession with a pending internal HITL request containing questions "approach", "priority" and "notes"
    When the handle's get_hitl_request is called for that session
    Then the wire HitlRequest contains exactly 3 questions with ids "approach", "priority" and "notes" in order
    And each wire question preserves the internal header, question text and option labels

  Scenario: A question without options surfaces an empty options array on the wire
    Given a BackgroundSession with a pending internal HITL request whose question "notes" has options None
    When the handle's get_hitl_request is called for that session
    Then the wire question "notes" has an empty options array

  Scenario: A cancelled wire response reaches the blocked tool as Cancelled
    Given a BackgroundSession with a pending internal HITL request
    And a thread is blocked on the session's wait_for_hitl_response
    When the handle receives send_hitl_response with cancelled true and no answers
    Then the blocked thread receives a Cancelled response with cancelled true

  Scenario: A structured multi-answer wire response maps pass-through to Answered keyed by answer id
    Given a BackgroundSession with a pending internal HITL request containing questions "approach" and "notes"
    And a thread is blocked on the session's wait_for_hitl_response
    When the handle receives send_hitl_response with answers for "approach" selecting "Option A" and for "notes" with other "free text"
    Then the blocked thread receives an Answered response with a 2-entry map keyed by "approach" and "notes"
    And the answer for "approach" has selected equal to ["Option A"] and other equal to None
    And the answer for "notes" has selected equal to [] and other equal to Some("free text")

  Scenario: Freeform text identical to an option label stays in other
    Given a BackgroundSession with a pending internal HITL request whose question "confirm_choice" has options "Yes" and "No"
    And a thread is blocked on the session's wait_for_hitl_response
    When the handle receives send_hitl_response with an answer for "confirm_choice" with empty selected and other "Yes"
    Then the blocked thread receives an Answered response
    And the answer for "confirm_choice" has selected equal to [] and other equal to Some("Yes")

  Scenario: The send path contains no option-label comparison
    Given the source of handle_impl.rs and the hitl mapping module
    When the send_hitl_response path is inspected
    Then it contains no comparison of answer values against option labels
    And it does not read the pending request to classify answers

  Scenario: New wire shapes round-trip through serde_json
    Given a wire HitlRequest with 3 questions including one without options and a wire HitlResponse with cancelled false and mixed answers
    When each value is serialized to JSON and deserialized back
    Then each deserialized value equals the original
    And a cancelled wire HitlResponse with empty answers also round-trips equal
