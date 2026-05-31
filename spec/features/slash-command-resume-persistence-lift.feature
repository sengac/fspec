@done
@RPC-049
@session-management
@rust
@persistence
@rpc
Feature: /resume persistence lift — get_session_message_envelopes
  """
  RPC-049 split-out feature file. Lifts
  `persistence_get_session_message_envelopes` from
  `codelet/napi/src/persistence/napi_bindings.rs` into
  `codelet_core::persistence::manifest::get_session_message_envelopes`
  as a public free function. The NAPI binding becomes a one-line
  delegate so the TS surface is byte-identical.

  Phase 1 of RPC-030 lifted the bulk of persistence out of NAPI;
  RPC-049 fills the last residual gap that `/resume` needs server-side.
  """

  Background: User Story
    As a fspec engineer wiring the /resume durable restore RPC
    I want get_session_message_envelopes to live in codelet_core::persistence
    So that codelet-sessions and codelet-rpc can build envelopes without crossing the NAPI boundary

  Scenario: get_session_message_envelopes for a missing session returns Err
    Given a non-existent session UUID
    When codelet_core::persistence::get_session_message_envelopes(uuid) is called
    Then the function returns Err with a message describing the missing session

  Scenario: get_session_message_envelopes returns parseable JSON envelopes
    Given an existing manifest with two appended messages
    When codelet_core::persistence::get_session_message_envelopes(uuid) is called
    Then the result is Ok(Vec<String>) with two JSON envelopes
    And each envelope parses via serde_json::from_str into a serde_json::Value with a 'message' field

  Scenario: codelet-napi binding is reduced to a thin delegate
    Given the codelet/napi/src/persistence/napi_bindings.rs file after RPC-049
    Then the codelet-napi binding persistence_get_session_message_envelopes still exists as a thin delegate
    And the binding's body calls codelet_core::persistence::get_session_message_envelopes(uuid)
