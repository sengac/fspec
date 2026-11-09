@done
@RPC-410
@REFAC-052
@rust
@pause-integration
@session
Feature: HITL wire protocol parity static checks: send-path source shape and serde round-trip
  """
  Split from hitl-wire-protocol-parity.feature by REFAC-052 so each feature's one test file stays under 300 lines. These two scenarios need no session: one inspects the send_hitl_response source (handle_impl composed tree plus sessions/src/hitl_mapping.rs), the other round-trips the wire shapes from rust/rpc-types through serde_json. Their test file is rust/sessions/tests/rpc410_hitl_wire_protocol_static_checks.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Freeform text identical to an option label stays in other (the RPC-408 label-inference heuristic is deleted; send path contains no option-label comparison)
  #   2. The new wire shapes round-trip through serde_json unchanged
  #
  # EXAMPLES:
  #   1. The send path source in handle_impl.rs contains no option-label comparison (source-shape check that the RPC-408 heuristic is gone)
  #   2. The new wire shapes (HitlQuestion, multi-question HitlRequest, HitlAnswer, cancel-capable HitlResponse) round-trip through serde_json equal to the original value
  #
  # ========================================
  Background: User Story
    As a AI agent using the request_user_input tool
    I want to have my full multi-question HITL request delivered to the frontend and receive a structured cancel-capable response with exact selected/other answers
    So that I never lose questions 2-3 and never receive misclassified or spuriously-answered responses

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
