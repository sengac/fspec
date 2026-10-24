@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex refresh freshness survives contradictory metadata and unchanged tokens
  """
  Supplemental regressions for A5 and A6; primary acceptance scenarios remain in prov159-refresh-races.feature.
  Primary tests: rust/providers/tests/prov159_refresh_freshness.rs. Use isolated homes and acknowledged loopback refresh responses.
  """

  Scenario: Expired JWTs override stale future expiry metadata
    Given an external rotation has an expired JWT and future stored expiry metadata
    When an existing client sends another request
    Then the issuer is contacted once and the refreshed credentials are persisted

  Scenario: Unchanged refresh tokens still publish renewed freshness
    Given two clients share expired credentials and the issuer returns identical token strings with renewed lifetime
    When both clients send requests in sequence
    Then both API requests succeed with only one issuer refresh
