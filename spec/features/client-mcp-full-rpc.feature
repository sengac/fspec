@cli
@rpc
@integration
@tool-execution
Feature: Complete RPC access through the MCP frontend
  Every daemon operation has a typed discoverable tool.

  Scenario: Discover the complete typed RPC surface
    Given the current daemon service declaration
    When the client lists RPC tools
    Then every declared method has a matching argument schema
    And unknown arguments are rejected before dispatch

  Scenario: Forward valid calls without an adapter confirmation form
    Given a caller without MCP form elicitation support
    When it calls an RPC with valid typed arguments
    Then the adapter forwards the operation once
    And daemon approval controls and errors remain in force
    And invalid arguments are rejected before dispatch
    And a timeout after dispatch reports an uncertain outcome without retrying

  Scenario: Read large results without repeating mutations
    Given an operation whose result exceeds one response chunk
    When the client reads the result continuation
    Then it receives bounded JSON text from the same result snapshot
    And the operation is not executed again

  Scenario: Reject stale approval responses atomically
    Given a pending daemon interaction
    When that interaction changes before its checked response is dispatched
    Then the response is rejected without answering the newer interaction
