@BUG-582
@BRIDGE-016
@tools
@inter-agent-coordination
Feature: Extend Rust bridge message types for command support
  """
  Pure type extension in rust/tools/src/bridge_relay.rs (InboundMessage) and rust/tools/src/bridge.rs (OutboundMessage). No behavioral changes. Uses serde Optional fields with skip_serializing_if for backward-compatible JSON serialization.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. InboundMessage in bridge_relay.rs gains three new Optional fields: request_id, command, args_json — all with serde default and skip_serializing_if Option::is_none
  #   2. OutboundMessage in bridge.rs gains one new Optional field: request_id — with serde default and skip_serializing_if Option::is_none
  #   3. All existing OutboundMessage construction sites must be updated with request_id: None to compile
  #   4. Existing InboundMessage without command fields still deserializes correctly (backward compatibility)
  #   5. OutboundMessage serialization with request_id: None must NOT include request_id in the JSON output (skip_serializing_if)
  #
  # EXAMPLES:
  #   1. JSON {type:command, session_id:s1, message:'', request_id:r1, command:board, args_json:'{}'} deserializes to InboundMessage with all command fields populated
  #   2. JSON {type:input, session_id:test-123, message:Hello} deserializes with all new command fields as None
  #   3. OutboundMessage with type commandResponse and request_id Some(req-001) serializes to JSON containing both type and request_id fields
  #   4. OutboundMessage with type chunk and request_id None serializes to JSON without any request_id field present
  #
  # ========================================
  Background: User Story
    As a bridge relay task
    I want to deserialize command InboundMessages and serialize commandResponse OutboundMessages
    So that the bridge protocol can carry fspec command traffic

  Scenario: Deserialize fspec command envelope with all fields populated
    Given a JSON envelope with service "fspec", type "command", request_id, and data command and args
    When the JSON is deserialized into an Envelope and routed with route_inbound
    Then route_inbound returns FspecCommand with request_id, command, and args_json populated from the JSON values

  Scenario: Backward compatible deserialization of input envelope without command fields
    Given a JSON envelope with service "session", type "input" and session_id but no request_id
    When the JSON is deserialized into an Envelope and routed with route_inbound
    Then the envelope request_id should be None and route_inbound returns SessionInput

  Scenario: Serialize OutboundMessage with request_id for commandResponse
    Given an OutboundMessage with type commandResponse and request_id set to a value
    When the OutboundMessage is serialized to JSON
    Then the JSON output should contain both the type and request_id fields

  Scenario: Serialize OutboundMessage without request_id for regular chunks
    Given an OutboundMessage with type chunk and request_id set to None
    When the OutboundMessage is serialized to JSON
    Then the JSON output should NOT contain a request_id field
