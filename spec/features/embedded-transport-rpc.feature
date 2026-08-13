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
Feature: Embedded transport for the FspecService tarpc surface
  """
  Architecture

  rust/rpc-embedded provides an in-memory tarpc transport backed by tarpc::transport::channel. It accepts a tokio::runtime::Handle from the host and never spawns its own runtime (per resolved RPC-002 Q9). The shared FspecService implementation lives in rust/rpc and is consumed unchanged by this transport — no business logic is inlined here.

  Spike RPC: list_work_units(ctx) -> Vec<WorkUnitInfo>. Reads from a tiny test-only in-memory fixture in this card.

  References: spec/attachments/RPC-002/rpc-002-feasibility.md sections 4, 5, 6, 9.
  """

  Background: User Story
    As a Rust developer building the new fspec frontend
    I want to reach the FspecService trait via an in-memory tarpc transport
    So that the TUI can run in-process without forking the type system or business logic

  Scenario: Embedded transport returns WorkUnitInfo from a single shared service impl
    Given the codelet workspace contains the rpc-types, rpc, rpc-embedded, and rpc-server crates and the shared FspecService implementation is seeded with a fixture of two WorkUnitInfo records
    When I construct an EmbeddedTransport with the current tokio runtime Handle, obtain an FspecServiceClient, and call list_work_units on the client
    Then the call returns Ok with a Vec<WorkUnitInfo> equal to the fixture
