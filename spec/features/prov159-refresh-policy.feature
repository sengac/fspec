@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex refresh policy rejects stale bindings and honors authoritative freshness
  """
  Supplemental regressions for A4, A6 and A14; the original acceptance scenarios remain in their primary features.
  Primary tests: rust/providers/tests/prov159_refresh_policy.rs. Use isolated homes, controlled keyrings and loopback issuers.
  """

  Scenario: Stale constructor credentials cannot create an unbound client
    Given persisted credentials belong to account B
    When a client constructed with stale account A credentials sends a request
    Then the request fails without contacting the API

  Scenario: Shared refresh reuses fresh persisted credentials
    Given persisted account A credentials have an hour of remaining lifetime
    When the shared refresh entry point is called twice
    Then both calls reuse the persisted tokens without contacting the issuer

  Scenario: Expired external JWTs cannot borrow freshness from last refresh
    Given an existing client observes an external rotation with an expired access token
    When the client prepares another request
    Then the rotated credentials are refreshed once with renewed expiry and unrelated fields preserved

  Scenario: Unrepresentable token lifetimes use conservative freshness
    Given explicit, persisted and refreshed credentials have an unrepresentable lifetime
    When clients prepare their token expiry
    Then expiry preparation succeeds without panicking
    And the credentials expire within the conservative one-hour fallback
