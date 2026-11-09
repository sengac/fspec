@session-management
@session
@done
@BUG-402
@rust
Feature: Session handle methods report the real session lookup error
  """
  rust/sessions/src/handle_impl/*.rs matched SessionManager::get_session's
  Err(_) and returned a fixed "Session not found: <id>" (or "Session
  unavailable", or wrapped the reason in another "Session not found"
  prefix), so an invalid session ID read the same as a missing session.
  Result-returning handle methods now propagate get_session's own error, and
  destroy_session returns the manager's reason unmodified. The Option and
  default-valued getters cannot carry an error without a trait/RPC contract
  change; that is a separate card.
  Tests: rust/sessions/tests/bug402_session_handle_lookup_errors.rs.
  """

  Background: User Story
    As a client of the session manager handle
    I want a failed session lookup to tell me why it failed
    So that a malformed id is not mistaken for a session that went away

  Scenario: A malformed session id is reported as invalid
    Given a session manager handle
    When session operations are called with a malformed session id
    Then each fails with an invalid session ID error

  Scenario: An unknown session is still reported as not found
    Given a session manager handle
    When session operations are called with a well-formed id of no session
    Then each fails with a session not found error
