@validation
@done
@BUG-403
@rpc
@rust
Feature: HITL wire payloads require their options and answers
  """
  rust/rpc-types/src/session_state.rs marked HitlQuestion.options and
  HitlResponse.answers #[serde(default)], so a payload that omitted either
  silently became a pure freeform question or a submit with no answers. Both
  fields are now required on the wire (an empty list is still valid when sent
  explicitly). The RPC-410 static check
  (sessions/tests/rpc410_hitl_wire_protocol_static_checks.rs) fell back to the
  rest of the composed source when its end marker was missing; it now fails
  naming the marker.
  Tests: rust/rpc-types/tests/bug403_hitl_wire_required_fields.rs.
  """

  Background: User Story
    As a frontend sending HITL questions and answers over the wire
    I want a payload missing its options or answers to be rejected
    So that a malformed message is never treated as a freeform question or an empty submit

  Scenario: A HITL question without options is rejected
    Given a HITL question payload with no options field
    When it is deserialized
    Then deserialization fails

  Scenario: A HITL response without answers is rejected
    Given a submitted HITL response payload with no answers field
    When it is deserialized
    Then deserialization fails and a response with an explicit empty answers list still parses

  Scenario: The send-path static check requires its end marker
    Given the RPC-410 send-path static check
    When it locates the end of the send_hitl_response body
    Then a missing end marker fails the check instead of falling back to the rest of the source
