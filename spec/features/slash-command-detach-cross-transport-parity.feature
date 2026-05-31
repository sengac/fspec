@done
@tui
@session-management
@rust
@multi-session
@rpc
@cross-transport
@RPC-050
Feature: /detach and work-unit binding cross-transport parity
  """
  Pins cross-transport parity for the two new pieces of trait surface
  exercised by RPC-050: backend.set_work_unit_context(SessionId,
  Option<WorkUnitContext>) and backend.get_work_unit_context(SessionId).
  Both methods landed on SessionManagerHandle / FspecService / both
  backends as part of RPC-037; this scenario asserts identical
  round-trip behaviour through EmbeddedFspecBackend AND WebSocketFspecBackend
  against the same StubSessionManagerHandle.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. set_work_unit_context(Some(ctx)) MUST round-trip identically across both transports
  #   2. set_work_unit_context(None) MUST round-trip identically across both transports
  #   3. get_work_unit_context(session) MUST return the most-recently-stored ctx on each transport
  #   4. The StubSessionManagerHandle MUST expose call counters so cross-transport parity tests can compare invocation counts
  #
  # ========================================
  Background: User Story
    As a fspec developer maintaining the dual-transport boundary
    I want set_work_unit_context / get_work_unit_context to behave identically through both transports
    So that the AgentView's BoardView attach + /detach paths produce the same backend state regardless of whether the frontend is embedded or WebSocket-attached

  Scenario: set_work_unit_context and get_work_unit_context round-trip identically across both transports
    Given a SharedFspecService wired to a StubSessionManagerHandle
    And an EmbeddedFspecBackend over that service
    And a WebSocketFspecBackend over that same service
    And the initial set_work_unit_context call counter on the stub is 0
    When backend.set_work_unit_context(SessionId("stub-1"), Some(ctx)) is awaited through each transport
    And backend.get_work_unit_context(SessionId("stub-1")) is awaited through each transport
    And backend.set_work_unit_context(SessionId("stub-1"), None) is awaited through each transport
    Then all six awaited calls return Ok
    And the StubSessionManagerHandle's set_work_unit_context call counter increments by exactly 4 (twice per transport)
    And the StubSessionManagerHandle's get_work_unit_context call counter increments by exactly 2 (once per transport)
    And each transport's get_work_unit_context call returns the previously-stored WorkUnitContext
