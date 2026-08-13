@done
@RPC-073
@rpc
@session-management
@bug
Feature: RPC-073 Slash Clear No Panic
  """
  Bug 1: typing /clear in a Work Agent on the Rust fspec binary panicked
  the tokio-rt-worker at rust/sessions/src/background_session.rs:1156:36
  with 'Cannot block the current thread from within a runtime'.

  Root cause: BackgroundSession::clear_history calls self.inner.blocking_lock()
  and was invoked from inside the async tarpc dispatch context via
  handle_impl.rs::clear_history without a tokio::task::block_in_place wrapper.
  The TS path was safe because NAPI dispatched it on a libuv worker (not Tokio).

  Fix: wrap session.clear_history() in tokio::task::block_in_place inside
  rust/sessions/src/handle_impl.rs::clear_history (the same pattern already
  used for create_session, create_isolated_session, test_provider_connection).
  Extend the RPC-070 source-shape scan to also flag .blocking_lock( / .blocking_read(
  / .blocking_write( in any sync trait method body so future siblings are caught.

  Reference: spec/attachments/RPC-073/research-bug1-clear-panic-ts-vs-rust.md
  """

  Background: User Story
    As a fspec user driving the Rust binary
    I want /clear to wipe my session history without crashing
    So that I can reset context inside a Work Agent session

  Scenario: Calling clear_history over embedded tarpc on a multi-thread runtime returns Ok and does not panic
    Given a SharedFspecService built with a real SessionManager is bound to an EmbeddedTransport on a tokio multi-thread runtime
    When the client calls client.clear_history(context::current(), session_id).await
    Then the RPC returns Ok(()) within 5 seconds
    Given a session has been created via client.create_session(context::current(), None) and has at least one message
    Then no worker thread emits the panic 'Cannot block the current thread from within a runtime'

  Scenario: Calling SessionManagerHandle::clear_history directly from an async task on a multi-thread runtime does not panic
    Given a SessionManager wrapped as Arc<dyn SessionManagerHandle> is created on a multi-thread tokio runtime
    When the test spawns an async task that calls handle.clear_history(&session_id) from inside the multi-thread runtime worker
    Then the task returns Ok(()) within 5 seconds
    Given the manager has a session created via handle.create_session(None)
    Then no worker thread emits a 'Cannot block the current thread' panic

  Scenario: Source-shape regression: every sync trait method in handle_impl.rs whose body contains .blocking_lock or .blocking_read or .blocking_write is wrapped in tokio::task::block_in_place
    Given the file rust/sessions/src/handle_impl.rs
    When the test reads the source bytes and strips line and block comments
    Then for every match of .blocking_lock( or .blocking_read( or .blocking_write( inside an fn body, the same fn body also contains tokio::task::block_in_place
    Then the test fails if any future change removes the wrapper from clear_history

  Scenario: After /clear the session output_buffer is empty and a SessionState::Cleared chunk is broadcast
    Given a session created via embedded tarpc has accumulated at least one StreamChunk in its output_buffer
    When the client calls client.clear_history(context::current(), session_id).await
    Then the call returns Ok(())
    Given a subscriber is listening on backend.chunks_rx for that session_id
    Then the subscriber receives a StreamChunk::SessionStateChange(SessionState::Cleared) within 1 second
    Then the session's output_buffer length is zero
