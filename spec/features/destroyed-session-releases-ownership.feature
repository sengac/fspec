@BUG-582
@done
@BUG-288
@BUG-333
@session-management
@codelet
Feature: Destroying a session releases its ownership lease
  """
  A daemon owns a session's manifest through a writer lease: SessionStore caches an Arc<WriterLease> per session
  (rust/core/src/persistence/manifest_commit.rs) holding an OS file lock on sessions/<id>.lock. destroy_session
  (rust/sessions/src/session_manager/manager_impl_6.rs) removes the session from memory and now also calls
  codelet_core::persistence::release_session_ownership, which drops the lease, so another process (a second daemon,
  or persistence_delete_session sent to it) can take ownership of the destroyed session.
  BUG-333: releasing ownership drops ONLY the writer lease. The cached manifest and the project's last-session pointer
  stay in SessionStore, because list_sessions/list_sessions_for_project and resume_last read that cache: a destroyed
  (closed) session must stay listed and resumable. Deleting a session (persistence_delete_session) is what evicts it.
  """

  Background: User Story
    As a user running more than one fspec daemon
    I want a destroyed session's ownership to be released
    So that another daemon can delete or reopen it without a lock error

  Scenario: A destroyed session's ownership lock can be taken by another owner
    Given a live session whose manifest the daemon has committed
    And its ownership lock is held by the daemon
    When the session is destroyed
    Then the ownership lock file can be locked by another owner

  Scenario: A destroyed session stays listed and resumable after its ownership is released
    Given a live session whose manifest the daemon has committed
    When the session is destroyed
    Then the ownership lock file can be locked by another owner
    And the session is still listed for its project
    And resuming the project's last session returns the destroyed session
