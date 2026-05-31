@done
@session
@session-management
@rpc
@RPC-070
Feature: Fix sync→async block_on panic in SessionManagerHandle impl (Work Agent crash)

  """
  Architecture: Option B from the fix-proposal.md attachment was chosen — wrap each affected sync->async bridge in tokio::task::block_in_place(|| Handle::current().block_on(...)) instead of refactoring the SessionManagerHandle trait to async fn (Option A) or building per-call runtimes (Option C, rejected).
  Architecture: The fspec binary's main is annotated with #[tokio::main] (default: multi-thread runtime), and codelet-napi's #[napi(tokio_main)] also uses multi-thread, so block_in_place is legal on every production call site. A debug_assert!(handle.runtime_flavor() == MultiThread, ...) inside the helper makes the precondition explicit.
  Architecture: New tarpc-over-duplex integration test lives at codelet/rpc/tests/create_session_no_panic_rpc070.rs — spawns the tarpc server on a real multi-thread runtime, connects via tarpc::serde_transport::tcp (or an in-memory duplex from tokio::io::duplex + tarpc::serde_transport::new). Asserts no panic + a non-empty SessionId.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All six sync->async bridges in handle_impl.rs (create_session, create_isolated_session, test_provider_connection, loop_add, loop_cancel, loop_list) MUST wrap their tokio::runtime::Handle::current().block_on(...) call in tokio::task::block_in_place(|| ...) so they can be invoked from inside a tarpc async handler without the nested-runtime panic
  #   2. The shared loop_block_on helper MUST use the block_in_place pattern and MUST include a debug_assert! that the current tokio runtime flavor is MultiThread
  #   3. test_provider_connection MUST stop constructing its own runtime via Handle::try_current() and instead use the same block_in_place + Handle::current().block_on(...) pattern as the other five bridges
  #   4. The doc-comments at handle_impl.rs:11-18 and :51-58 MUST be rewritten to describe the correct contract (multi-thread runtime required, no requirement that the calling thread NOT be a runtime worker)
  #   5. A new tarpc-over-in-memory-duplex integration test MUST exercise create_session via the live tarpc dispatcher and assert no panic + a valid SessionId is returned
  #   6. The existing e2e/rpc-068-work-agent-panic-repro.test.ts MUST be kept as a permanent regression guard and MUST pass after the fix (no 'Cannot start a runtime' / 'block_on' / 'panicked' in the rendered buffer after pressing Enter on a DONE work unit)
  #   7. All pre-existing cargo test --workspace tests (including codelet-sessions/tests/handle_impl.rs shape tests) MUST continue to pass
  #
  # EXAMPLES:
  #   1. User runs `fspec` Rust binary, navigates to DONE column, presses Enter on a work unit -- the Work Agent panel renders WITHOUT any 'Cannot start a runtime' or 'block_on' panic text
  #   2. A tarpc client calls FspecService::create_session(None) over an in-memory duplex transport while the server is driven by the multi-thread tokio runtime -- the call returns a non-empty SessionId and no thread in the server panics
  #   3. Source-shape grep over codelet/sessions/src/handle_impl.rs shows zero raw `Handle::current().block_on(` occurrences outside a tokio::task::block_in_place(|| ...) wrapper (the impl block contains six block_in_place wrappers total)
  #   4. The pre-existing codelet-sessions handle_impl tests (scenario_session_manager_satisfies_trait_object, scenario_unknown_session_id_returns_safe_defaults, scenario_impl_block_exists_with_every_override) all still pass after the fix
  #
  # ========================================

  Background: User Story
    As a fspec user
    I want to open the Work Agent by pressing Enter on a work unit in the Rust TUI
    So that I can interact with my session without the Rust binary panicking on a nested tokio runtime

  Scenario: create_session does not panic when invoked from a multi-thread tokio runtime worker
    Given a tokio multi-thread runtime is active
    And a fresh SessionManager wrapped as Arc<dyn SessionManagerHandle>
    When the test calls handle.create_session(None) from inside the multi-thread runtime
    Then no thread panics with "Cannot start a runtime from within a runtime"
    And the call returns a SessionId value

  Scenario: create_session over the live tarpc embedded transport returns without panicking
    Given a tokio multi-thread runtime is active
    And a temp workspace with an empty spec/work-units.json
    And a SharedFspecService built with a real SessionManager via with_session_manager
    And an EmbeddedTransport bound to the multi-thread runtime handle
    When the test calls client.create_session(context::current(), None).await on the tarpc client
    Then the RPC returns Ok(SessionId)
    And no worker thread emits the panic "Cannot start a runtime from within a runtime"

  Scenario: Every Handle::current().block_on call inside handle_impl.rs is wrapped in tokio::task::block_in_place
    Given the file codelet/sessions/src/handle_impl.rs
    When I read the source bytes
    Then every occurrence of "tokio::runtime::Handle::current().block_on(" is preceded (within the same statement) by a "tokio::task::block_in_place(" call
    And the file contains exactly one "fn loop_block_on" helper
    And the loop_block_on helper body contains "tokio::task::block_in_place"
    And the loop_block_on helper body contains a debug_assert! on RuntimeFlavor::MultiThread

  Scenario: test_provider_connection no longer constructs its own runtime
    Given the file codelet/sessions/src/handle_impl.rs
    When I read the source bytes
    Then the test_provider_connection method body does not contain "Handle::try_current()"
    And the test_provider_connection method body contains "tokio::task::block_in_place"

  Scenario: Pre-existing SessionManagerHandle shape tests still pass
    Given the RPC-070 fix is applied
    When cargo test -p codelet-sessions --test handle_impl runs
    Then scenario_session_manager_satisfies_trait_object passes
    And scenario_unknown_session_id_returns_safe_defaults passes
    And scenario_impl_block_exists_with_every_override passes
