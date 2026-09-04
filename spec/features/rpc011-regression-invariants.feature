@done
@workflow-automation
@regression
@rust
@rpc
@RPC-011
Feature: RPC-011 regression invariants (prior RPC-005..010 signatures and architecture preserved)
  """
  RPC-011 is ADDITIVE only: bind_and_serve signature unchanged, WebSocketFspecBackend::connect
  signature unchanged, build_service unchanged, no second App / no second envelope format / no
  rat-dialog framework. Architecture invariants from RPC-005 still hold (single trait/impl,
  loopback bind, no codelet-napi dep on fspec). All earlier test suites still pass.
  """

  Background: User Story
    As a power-user driving the rust binary
    I want RPC-011 changes to be purely additive
    So that earlier work (RPC-005..010) remains intact and green

  Scenario: bind_and_serve signature is unchanged
    Given the public signature of codelet_rpc_server::bind_and_serve
    When compared against its RPC-005 form
    Then it still returns (SocketAddr, ServerStats, JoinHandle<()>)
    And it still takes (bind_addr: &str, service: Arc<SharedFspecService>) — no new parameter

  Scenario: WebSocketFspecBackend::connect signature is unchanged
    Given the public signature of WebSocketFspecBackend::connect
    When compared against its RPC-008 form
    Then it still takes a single url::Url and returns Result<Self>
    And the new connect_with_supervisor sits BESIDE it as an additive constructor (does NOT replace it)

  Scenario: Architecture invariants from RPC-005 still hold
    Given the rust/rpc-embedded/tests/architecture_invariants.rs source-shape regression
    When the test is run on the RPC-011 tree
    Then it asserts: types defined exactly once
    And rpc-server still binds 127.0.0.1
    And no tokio::runtime::Builder / Runtime::new() construction in rust/fspec/src/ or rust/rpc-server/src/
    And no second envelope format exists (Envelope is the sole wire-format type in rust/rpc-server/src/envelope.rs)
    And the rpc crate has no codelet-core dep
    And the test passes

  Scenario: Earlier RPC-005..010 test suites still pass
    Given the full Cargo workspace at the end of RPC-011 implementation
    When running cargo test -p codelet-rpc -p codelet-rpc-server -p codelet-rpc-embedded -p codelet-fspec-tui -p codelet-fspec --release
    Then all prior tests pass
    And no test was disabled, skipped, or marked #[ignore] to make RPC-011 green
