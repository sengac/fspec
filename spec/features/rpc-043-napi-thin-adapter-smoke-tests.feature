@done
@rpc
@infrastructure
@session-management
@codelet
@RPC-043
Feature: RPC-043 NAPI thin-adapter smoke-test contract

  """
  Architecture notes:
  - Split out from the main RPC-043 feature file (reduce-codelet-napi-to-thin-adapter-session-bindings-rs-update-cargo-toml.feature)
    to satisfy fspec's 1-feature = 1-test-file invariant. RPC-043's deliverables include BOTH a static shape-test
    binary (codelet/napi/tests/session_bindings_shape.rs) AND a runtime smoke-test binary
    (codelet/napi/tests/session_bindings_smoke.rs). When fspec's 1:1 validator was introduced this feature was
    refactored into two siblings to keep both test binaries traceable to their owning Gherkin scenarios.
  - The smoke binary exercises every public #[napi] wrapper from codelet-napi against a fresh SessionManager
    singleton + the canonical UNKNOWN_UUID, locking in the pre-RPC-043 observable error/no-op behaviour as a
    regression detector for future agent-loop lifts (RPC-046..RPC-068).
  - Behaviour-preservation invariant: NO #[napi] symbol is renamed, removed, or has its signature altered; every
    wrapper continues to compile against the existing `use codelet_napi::{...}` import block.
  """

  Background:
    Given the RPC-043 changes are applied to the codelet workspace
    And UNKNOWN_UUID is the well-formed but unregistered UUID "00000000-0000-0000-0000-000000000000"

  @rule:smoke_exercises_every_wrapper
  @smoke
  @binding
  Scenario: A new smoke test exercises every #[napi] wrapper at least once
    Given the RPC-043 changes are applied to the codelet workspace
    And no real sessions have been created on the global SessionManager singleton
    And UNKNOWN_UUID is the well-formed but unregistered UUID "00000000-0000-0000-0000-000000000000"
    When I run `cargo test -p codelet-napi --test session_bindings_smoke`
    Then the command exits with code 0
    And `session_get_status(UNKNOWN_UUID)` returns a napi::Error with message containing "Session not found"
    And `session_manager_list()` returns an empty Vec
    And `session_get_pending_input(UNKNOWN_UUID)` returns Err with message containing "Session not found"
    And `session_clear_active()` is a silent no-op
    And `session_get_active()` returns None when no session is active
    And `session_set_active(UNKNOWN_UUID)` returns an Err containing "Session not found"
    And every other `#[napi]` wrapper from the 66-entry table is invoked at least once
    And wrappers that parse session ids reject the literal string "nonexistent" with an "Invalid session ID" napi::Error (UUID parse failure, observed pre-RPC-043 behaviour)

  @rule:behaviour_preserved
  @behaviour
  Scenario: Each #[napi] wrapper preserves observable behaviour across the move
    Given the RPC-043 changes are applied to the codelet workspace
    And the Rust smoke test exercises every public `#[napi]` wrapper via the codelet-napi crate API
    When the smoke test runs against the post-RPC-043 native module
    Then every wrapper returns the same value, error, or no-op as the pre-RPC-043 baseline
    And no public `#[napi]` symbol is renamed, removed, or has its signature altered
    And every wrapper imported in the smoke test's `use codelet_napi::{...}` block continues to resolve at compile time
