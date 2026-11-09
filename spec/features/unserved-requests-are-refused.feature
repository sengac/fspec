@done
@BUG-453
@rpc
@multi-workspace
@rust
Feature: Requests a service or backend cannot serve are refused
  """
  The tarpc service implementation answered session RPCs with a default
  value when no SessionManagerHandle was attached (schedule_add returned
  Ok(ScheduledJob::default()), destroy-style calls Ok(())); the TUI's
  FspecBackend trait defaults faked success (create_isolated_session
  returned an empty session id, destroy_session Ok(())); and the client
  daemon directory turned an unreadable saved-daemons file into no daemons.
  Now the 37 Result-returning no-handle arms refuse with
  codelet_rpc::NO_SESSION_MANAGER (getters without an error channel are
  BUG-445); the 73 Result-returning backend defaults refuse with
  BackendError::Unimplemented(method) and the test mock implements the ten
  it relied on explicitly; DaemonDirectory::list returns Result and the
  switcher shows a failure as an error dialog.
  Tests: rust/fspec-tui/tests/bug453_unserved_requests_are_refused.rs.
  """

  Background: User Story
    As a user of the TUI or an RPC client
    I want to have requests a service or backend cannot serve refused with the reason
    So that a missing session manager or an unimplemented backend method never looks like success

  Scenario: A session RPC without a session manager is refused
    Given an fspec service with no session manager attached
    When a scheduled job is added
    Then the request fails saying no session manager is attached

  Scenario: A backend method a backend does not implement is refused
    Given a backend that does not implement isolated session creation
    When an isolated session is created through it
    Then the call fails saying create_isolated_session is not implemented by this backend

  Scenario: An unreadable daemon list is shown as an error
    Given a daemon directory that cannot read its saved daemons
    When the daemon switcher is opened
    Then the daemon switcher shows the daemon directory's error
