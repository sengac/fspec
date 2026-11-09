@session
@done
@BUG-369
@tool-execution
@rust
Feature: Session handles report malformed ids and exec-stdin state failures
  """
  rust/sessions/src/handle_impl.rs mapped an unparsable session id to
  Uuid::nil() (uuid_from); background_session/exec_stdin.rs skipped the write
  and reported "no request" when the exec_stdin_request lock was poisoned;
  and the SessionManagerHandle::write_exec_stdin default
  (core handle_trait_3.rs) returned Ok(()) without writing.
  Now: handle methods pass the raw id to SessionManager::get_session (which
  rejects it as an invalid session ID), parse_session_uuid returns the error,
  and logged_session_uuid logs it for methods with no error channel. The
  exec-stdin state goes through sessions::lock_state (errors on a poisoned
  lock), and the trait default returns an unsupported error.
  Tests: rust/sessions/tests/bug369_session_exec_stdin_and_id_errors.rs.
  """

  Background: User Story
    As a client of the session manager handle
    I want malformed ids and broken exec-stdin state to be reported
    So that a bad request never silently addresses a nil session or loses typed input

  Scenario: A malformed session id is rejected
    Given a session manager handle
    When a supervisor is added with a malformed subordinate id
    Then the call fails naming the invalid session id

  Scenario: A poisoned exec-stdin state lock is reported
    Given exec-stdin state whose lock was poisoned
    When the state is read and written
    Then both operations fail instead of reporting no request

  Scenario: The default write_exec_stdin reports that it is unsupported
    Given a session manager handle that does not implement exec stdin writes
    When text is written to an exec session through it
    Then the write fails instead of claiming the text was written
