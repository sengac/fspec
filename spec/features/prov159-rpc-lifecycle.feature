@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
@tui
@integration
Feature: Embedded Codex OAuth cancellation retry and operation deadlines
  """
  PROV-159 acceptance rows A11-A13. Primary tests: rust/fspec-tui/tests/prov159_rpc_lifecycle.rs.
  Exercise the real embedded backend and tarpc service, not MockBackend-only behavior. Interactive browser login, device start and device polling receive an operation-specific 300-second budget; unrelated RPC defaults stay short.
  Cancellation and client disconnect must stop backend work and release the callback listener before immediate retry. Attempt identity and durable generation fencing reject stale callbacks and token commits.
  Use shared isolated-home fixtures, a controlled keyring boundary and real loopback OAuth endpoints; never real user credentials.
  Research: spec/attachments/PROV-159/prov159-oauth-research.md and its two upstream companions.
  """

  Scenario: A11 Esc permits immediate browser retry without a stale callback commit
    Given Codex browser login attempt A is waiting on the local callback port
    When the user presses Esc and immediately starts browser login attempt B before delivering the old A callback
    Then the requesting task for attempt A is cancelled rather than merely hiding its result
    And attempt B can bind the same callback port without waiting for attempt A to time out
    And the late callback for attempt A cannot persist credentials or complete attempt B
    And only the valid callback for attempt B can commit credentials and show success
    And the cancellation and retry execute through the real embedded backend

  Scenario: A12 Interactive OAuth outlives the default RPC deadline
    Given Codex browser authorization or device polling runs through the real embedded tarpc transport
    When authorization completes after more than 10 seconds but within the supported 300-second OAuth budget
    Then the login succeeds and the committed credentials are available to the settings UI and providers
    And unrelated RPC calls retain their existing short default deadlines
    And the operation budget includes callback waiting, polling and token exchange

  Scenario: A13 OAuth deadlines and client disconnects clean up backend work
    Given a Codex browser or device login is pending through the real embedded tarpc transport
    When its operation deadline expires or the requesting client disconnects before authorization completes
    Then the operation has a clear timeout or cancellation outcome and reports no successful login
    And the callback listener, polling tasks and exchange work are cleaned up
    And delayed authorization cannot persist credentials after cleanup
    And a subsequent login can immediately reuse the callback port
