@done
@integration-test
@p1
@critical
@workspace
@infrastructure
@rust
@tarpc
@rpc
@RPC-006
Feature: Reserved envelope variants narrowed after WorkUnitsUpdate
  """
  Architecture

  RPC-005 reserved five envelope variants behind the server's reject-and-warn
  defensive default: Event, LogEvent, WorkUnitsUpdate, CmdReq, CmdRes.

  RPC-006 implements `WorkUnitsUpdate(Vec<WorkUnitInfo>)` as a legitimate
  first-class variant. The reserved-variants list narrows to four:
  {Event, LogEvent, CmdReq, CmdRes}. This feature codifies the
  regression: the server MUST continue to reject the still-reserved
  variants AND MUST NOT count `WorkUnitsUpdate` as rejected.

  RPC-007 will lift `Event` and `LogEvent` next.

  References: spec/attachments/RPC-006/plan.md (Step 5);
  RPC-005 architecture rule [6].
  """

  Background: User Story
    As a Rust developer maintaining the new RPC stack
    I want the server to keep rejecting Event/LogEvent/CmdReq/CmdRes envelopes after WorkUnitsUpdate becomes legitimate
    So that future variants land deliberately rather than accidentally and the rejected-variants log faithfully reflects the implementation set

  Scenario: Reserved envelope variants are still rejected after WorkUnitsUpdate is implemented
    Given the rpc-server is running after RPC-006
    When a WebSocket client sends a frame whose Envelope variant is one of Event, LogEvent, CmdReq, or CmdRes
    Then the server records the unsupported variant by name in its rejection log, does not invoke any FspecService method as a result of that frame, and the rejected-variants list reported by ServerStats does not contain WorkUnitsUpdate
