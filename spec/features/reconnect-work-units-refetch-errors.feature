@done
@BUG-361
@tui
@reconnect
@connection
Feature: Reconnect work units refetch errors
  """
  On Action::Reconnected, App::handle_reconnected (rust/fspec-tui/src/app/dispatch_reconnect.rs) re-fetches the work
  units with backend.list_work_units() so the board re-seeds on the fresh client. Before BUG-361 a failed refetch was
  dropped (if let Ok), leaving a stale board with no feedback. Now the failure is logged with tracing::error and sent
  as Action::SessionOperationFailed, which the existing session storage error dialog shows. Sends that fail because
  the app loop closed (the refetch and the reconnect notice auto-dismiss timer) are logged instead of dropped.
  """

  Background: User Story
    As a user whose TUI reconnects to the daemon
    I want to see an error when the work units cannot be reloaded after the reconnect
    So that I know the board may be stale instead of trusting old work units

  Scenario: A failed work-unit refetch after reconnect is shown as an error
    Given the TUI is connected to a daemon
    And listing work units fails with "daemon busy"
    When the TUI reconnects
    Then an error dialog reads "Work units could not be reloaded after reconnect: daemon busy"
