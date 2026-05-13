@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@rust
@tarpc
@rpc
@RPC-005
Feature: Cross-transport parity for the FspecService tarpc surface
  """
  Architecture

  Both the embedded and WebSocket transports MUST produce semantically identical results for the same call against the same shared service implementation. There is exactly one implementation of FspecService, hosted in a shared module; both transports delegate to it. Tests assert (a) value-level equality between transports and (b) that both calls actually reach the same business-logic function via an invocation counter.

  References: spec/attachments/RPC-002/07-recommended-architecture.md section 7; rule [3] in RPC-005 (service implementation written ONCE).
  """

  Background: User Story
    As a Rust developer maintaining the new RPC stack
    I want automated tests that prove the embedded and WebSocket transports observe the same shared service impl
    So that no RPC method silently drifts into being embedded-only or WebSocket-only over time

  Scenario: Both transports produce semantically identical results for the same call
    Given I have an embedded FspecServiceClient and a WebSocket FspecServiceClient connected to the same shared service impl seeded with a fixture of two WorkUnitInfo records
    When I call list_work_units through both clients
    Then both calls return Ok and the two returned Vec<WorkUnitInfo> values are equal under PartialEq

  Scenario: Both transport calls reach the same shared service implementation
    Given the shared FspecService implementation increments a list_work_units invocation counter on every call, and I have an embedded FspecServiceClient and a WebSocket FspecServiceClient connected to that single shared impl
    When I call list_work_units once on the embedded client and once on the WebSocket client
    Then the shared invocation counter has been incremented exactly twice
