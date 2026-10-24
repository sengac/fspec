@done
@PROV-159
@providers
@authentication
@oauth
@rust
@phase-1
Feature: Codex credential interoperability preserves local lifecycle isolation
  """
  PROV-159 review regressions supplement A1, A2, A4 and A6 with realistic
  preferred documents, upstream credential rewrites and identical re-login tokens.
  Tests use isolated homes, controlled keyring storage and loopback HTTP/WebSockets.
  Primary tests: rust/providers/tests/prov159_review_regressions.rs.
  """

  Scenario: Null preferred API keys do not erase independent fallback keys
    Given preferred OAuth credentials contain a null API key and the fallback has an independent key
    When OAuth logout or a replacement login is persisted
    Then the independent API key and unrelated fields remain available

  Scenario: External credential rewrites without private metadata are reloaded safely
    Given an existing client and an upstream same-account rotation without private metadata
    When the existing client prepares another request
    Then the new credentials are reused without refreshing the stale token
    And a completed local logout and same-account login still revoke the old client

  Scenario: Identical re-login credentials cannot reuse prior socket or routing state
    Given a completed socket response for an authenticated account
    When the account logs out and logs back in with identical token strings
    Then a newly bound client opens a new socket without prior continuation state
    And account routing affinity is renewed without an intervening revoked request
