@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex pending login fencing survives process boundaries and blocked persistence
  """
  Supplemental regressions for A10; its primary acceptance scenario remains in prov159-login-lifecycle.feature.
  Primary tests: rust/providers/tests/prov159_lifecycle_regressions.rs. Child processes exercise external transitions and storage lock contention against isolated real homes.
  """

  Scenario: External authentication transitions fence pending login
    Given a pending login with or without previously persisted credentials
    When another process deletes credentials, logs in, begins login or replaces the credential document
    Then the pending login cannot overwrite the authoritative external state

  Scenario: Equivalent home paths share cancellation
    Given a pending login associated with a credential home
    When login is cancelled through an equivalent filesystem path
    Then the original attempt cannot persist credentials

  Scenario: Stale device completion preserves a replacement session
    Given a new device session replaces an older session
    When completion is attempted for the old session and then the replacement
    Then the old session fails without preventing the replacement from reaching the issuer

  Scenario: Login deadline remains effective while persistence is blocked
    Given browser authorization has reached token exchange and another process holds the storage lock
    When the login deadline expires while persistence waits for that lock
    Then login fails and no credentials appear after the lock is released

  Scenario: Cancellation remains effective while persistence is blocked
    Given a login commit waits for a storage lock held by another process
    When the pending login is cancelled
    Then commit promptly fails and no credentials appear after the lock is released
