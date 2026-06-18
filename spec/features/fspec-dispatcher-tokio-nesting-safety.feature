@done
@tool-execution
@infrastructure
@cli
@RPC-327
Feature: list-work-units dispatcher hangs agent loop when invoked from inside tokio runtime

  """
  The synchronous Rust dispatcher in codelet/fspec-core/src/dispatch.rs is the single entry point the agent loop's fspec_handler closure delegates to when the NAPI chunk callback is NOT registered (i.e. the standalone fspec Rust binary). It MUST be callable from inside an active tokio runtime context without dead-locking. The Phase 1 implementation built a fresh tokio::runtime::Builder::new_current_thread() and called block_on() inside it, which nests tokio runtimes — that path is forbidden here. The fix replaces the nested-runtime path with a sync poll-once helper (poll_sync_future) that drives the command future to completion using a no-op Waker. Every ported command (currently just list-work-units, RPC-253) and every Phase 1 stub completes on the first poll because they only touch std::fs / serde_json, never tokio async I/O. A regression that introduces a genuine .await surfaces as a structured FspecCoreError::InvalidArgs from the helper instead of a silent hang.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. dispatch_command MUST be safe to call from within an active tokio runtime context (the agent loop's #[tokio::main])
  #   2. dispatch_command MUST NOT call tokio::runtime::Builder::new_current_thread().build().block_on(...) — nesting tokio runtimes deadlocks/panics inside the agent loop
  #   3. Ported and stub command futures today perform NO genuine async work, so a sync poll-once helper with a noop Waker is sufficient to drive them to completion
  #   4. If a future regression introduces a genuine .await on async I/O, the sync poll helper MUST surface a structured error (not hang) so the regression is caught loudly
  #
  # EXAMPLES:
  #   1. Inside a #[tokio::test], spawn_blocking → dispatch_command('list-work-units', JSON args, seeded project_root) returns success=true with the workUnits array within 2 seconds (does not hang)
  #   2. From a synchronous #[test] (no surrounding tokio runtime), dispatch_command continues to behave correctly — backwards-compatibility with the existing dispatcher_test suite is preserved
  #   3. An unported command name routed through dispatch_command from inside a tokio runtime returns the canonical NotYetPorted error within 2 seconds instead of deadlocking
  #
  # ========================================

  Background: User Story
    As a agent loop running the standalone fspec Rust binary inside #[tokio::main]
    I want to invoke the Fspec tool dispatcher for a ported command
    So that the LLM receives a structured result instead of the tool call hanging indefinitely

  Scenario: Dispatching list-work-units from inside an active tokio runtime returns synchronously without hanging
    Given a tempdir project root seeded with spec/work-units.json containing AUTH-001, AUTH-002, and DASH-001
    When I invoke dispatch_command for the list-work-units command via tokio::task::spawn_blocking
    Then the DispatchResult has success=true within 2 seconds
    Given the test is running inside an active tokio runtime via #[tokio::test]
    Then the workUnits array contains AUTH-001, AUTH-002, and DASH-001 in insertion order


  Scenario: Dispatching list-work-units from a synchronous test context still works (backwards compatibility)
    Given a tempdir project root seeded with a representative spec/work-units.json
    When I invoke dispatch_command for the list-work-units command directly
    Then the DispatchResult has success=true and the filter/render output matches the existing list_work_units test suite expectations
    Given the test is a plain #[test] with no surrounding tokio runtime


  Scenario: Dispatching a ported command with invalid args from inside a tokio runtime returns a structured error without hanging
    Given the canonical command 'add-rule' is ported (is_ported returns true)
    When I invoke dispatch_command for the 'add-rule' command with empty args_json via tokio::task::spawn_blocking
    Then the DispatchResult has success=false within 2 seconds
    Given the test is running inside an active tokio runtime via #[tokio::test]
    Then the error message contains the substring 'Invalid args'
    And the error message does NOT contain the substring 'not yet ported'

