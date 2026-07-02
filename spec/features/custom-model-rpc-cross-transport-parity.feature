@done
@RPC-347
@rpc
@model-selection
@parity
Feature: Custom-model RPC write surface cross-transport parity
  """
  RPC-347 slice: prove add/update/delete_custom_model behave identically over
  the EmbeddedFspecBackend and WebSocketFspecBackend, both built against the
  SAME StubSessionManagerHandle, and that they are silent no-ops when no
  SessionManagerHandle is attached. Mirrors the RPC-037 parity pattern. Test:
  codelet/fspec-tui/tests/rpc347_cross_transport_parity.rs
  """

  Background: User Story
    As a model-selector UI (and any RPC client)
    I want add/update/delete custom-model calls to behave identically on every transport
    So that the choice of embedded vs websocket transport never changes the result

  Scenario: add_custom_model produces identical results across transports
    Given an openai profile "work-vllm" exists with no custom models
    When a client calls add_custom_model with the same definition over the embedded transport
    And another client calls add_custom_model with the same definition over the websocket transport
    Then both calls return Ok
    And both transports persist the identical customModels entry

  Scenario: RPC methods are a silent no-op without an attached SessionManagerHandle
    Given a FspecServiceImpl with no SessionManagerHandle attached
    When a client calls add_custom_model, update_custom_model, and delete_custom_model
    Then each call returns Ok
    And the websocket transport behaves identically over its no-handle path
    And no configuration is written
