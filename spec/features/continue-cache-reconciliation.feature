@tui
@codelet
@completion
@BUG-225
Feature: Continue cache hydration and reconciliation
  """
  Create and attach chrome reads backend continue state, never profile defaults.
  Session-keyed revision guards discard stale reads and failure rollbacks.
  Unknown relative commands await hydration; backend writes retain command order.
  These chrome settings still apply on the next user dispatch, not to an active
  stream. Profile autoContinue remains a creation-only seed.
  """

  Scenario: Create and attach hydrate backend continue settings
    Given backend sessions with enabled budget 300 and disabled budget 47
    When the TUI creates or attaches to those sessions without stream updates
    Then each session cache matches its backend settings without writing defaults

  Scenario: An early relative command waits for backend state
    Given an unknown continue cache and backend enabled budget 300
    When a bare toggle arrives before hydration completes
    Then the backend is disabled with budget 300 rather than enabled with budget 10

  Scenario: A failed getter refuses guessed relative changes
    Given an unknown continue cache whose backend getter fails
    When a bare toggle is submitted
    Then no continue setter runs and the originating session receives an error

  Scenario: A failed setter reconciles or rolls back its matching cache
    Given a hydrated enabled budget 300 and a failing continue setter
    When a budget command fails with a successful or failed reconciliation getter
    Then the cache returns to enabled budget 300 and reports the failure

  Scenario: Late hydration cannot overwrite commands or live updates
    Given a pending hydration response with enabled budget 300
    When a newer budget command or live continue update arrives
    Then the late hydration leaves the newer state intact

  Scenario: Responses remain bound to their originating session
    Given hydration and a relative command pending for one session
    When focus switches to a second session before the response arrives
    Then only the originating session is hydrated and toggled

  Scenario: Late setter failure cannot roll back newer state
    Given a pending failing budget command
    When a newer command or live continue update changes the cache
    Then the older failure does not roll back the newer state

  Scenario: Rapid continue commands preserve submission order
    Given an unknown continue cache and backend enabled budget 300
    When two bare toggles and a budget command arrive before hydration
    Then the writes run in command order and finish enabled with budget 77

  Scenario: Chrome refresh waits for an already submitted continue write
    Given a hydrated enabled budget 300 with a budget 77 write still pending
    When the session chrome refreshes before that write finishes
    Then hydration waits for the write and retains enabled budget 77

  Scenario: An unconfirmed absolute command remains unknown after failed reconciliation
    Given an unknown continue cache whose getter and setter both fail
    When a numeric budget command fails and a bare toggle follows
    Then the cache remains unknown and the toggle never submits a guessed state
