@BUG-582
@done
@BUG-379
@tui
@connection
Feature: Work units refetch errors
  """
  The TUI re-fetches work units outside the normal broadcast in three places: after a reconnect (BUG-361), when the
  work_units broadcast subscriber in App::spawn_subscriber_tasks (rust/fspec-tui/src/app/bootstrap.rs) lags and
  resyncs, and after a daemon switch in App::handle_daemon_switched (rust/fspec-tui/src/app/dispatch_daemons.rs),
  which also reloads the checkpoint counts. All three use one helper in rust/fspec-tui/src/app/work_units_refetch.rs:
  a failed fetch is logged with tracing::error and sent as Action::SessionOperationFailed, which the session storage
  error dialog shows. A send that fails because the app loop closed is logged instead of dropped.
  """

  Background: User Story
    As a user watching the board
    I want to see an error when the work units or checkpoint counts cannot be reloaded
    So that I know the board may be stale instead of trusting old data

  Scenario: A failed work-unit resync after missed updates is shown as an error
    Given the TUI is subscribed to work-unit updates
    And listing work units fails with "daemon busy"
    When the work-unit updates arrive faster than the TUI reads them
    Then an error dialog reads "Work units could not be reloaded after missed updates: daemon busy"

  Scenario: A failed work-unit reload after a daemon switch is shown as an error
    Given daemon "beta" fails to list work units with "daemon busy"
    When the TUI switches to daemon "beta"
    Then an error dialog reads "Work units could not be reloaded after switching to daemon beta: daemon busy"

  Scenario: A failed checkpoint count reload after a daemon switch is shown as an error
    Given daemon "beta" fails to count checkpoints with "store locked"
    When the TUI switches to daemon "beta"
    Then an error dialog reads "Checkpoint counts could not be reloaded after switching to daemon beta: store locked"
