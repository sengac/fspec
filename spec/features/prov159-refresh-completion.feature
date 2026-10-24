@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex refresh completion respects deletion and request deadlines
  """
  Supplemental A3, A4 and A13 regressions found during final validation.
  Deleted credentials retain a durable revocation marker even without auth.json.
  Refresh persistence must yield while waiting for shared-home writers so transport cancellation remains effective.
  Primary tests: rust/providers/tests/prov159_refresh_completion.rs.
  """

  Scenario: Deleted credentials cannot construct an unfenced client
    Given account A credentials were captured before provider deletion
    When stale clients are constructed after deletion and a subsequent login
    Then HTTP streaming and socket requests fail without contacting the issuer or API

  Scenario: Refresh persistence respects streaming credential deadlines
    Given an expired client receives refreshed credentials while another writer holds the credential lock
    When the streaming or socket credential deadline expires before the lock is released
    Then the request fails promptly without blocking the runtime
    And releasing the lock does not persist the cancelled refresh
