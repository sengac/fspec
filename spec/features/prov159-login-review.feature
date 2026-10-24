@done
@PROV-159
@providers
@authentication
@oauth
@rust
@phase-1
Feature: Codex login expiry and synchronized cancellation
  """
  Review findings 4 and 5 only. Login responses carry optional access-token lifetimes independently of JWT claims.
  Persist absolute expiry together with tokens under the login attempt's cancellation and generation fences, preserving unrelated credentials.
  Preserve the public token-only commit API and discard stale expiry when no new lifetime is supplied.
  Tests use isolated homes, controlled keyrings, loopback issuers and acknowledged pending operations. Cancellation must complete normally without aborting backend tasks.
  Primary tests: rust/providers/tests/prov159_login_review.rs.
  """

  Scenario: Browser login persists an opaque access token with its short expiry
    Given existing credentials have a stale expiry and unrelated authentication fields
    When browser authorization returns an opaque access token with a short lifetime
    Then the new tokens and their absolute expiry are persisted together without losing unrelated fields

  Scenario: Direct device login persists an opaque access token with its short expiry
    Given existing credentials have a stale expiry and unrelated authentication fields
    When direct device authorization returns an opaque access token with a short lifetime
    Then the new tokens and their absolute expiry are persisted together without losing unrelated fields

  Scenario: Split device login persists an opaque access token with its short expiry
    Given existing credentials have a stale expiry and unrelated authentication fields
    When split device authorization returns an opaque access token with a short lifetime
    Then the new tokens and their absolute expiry are persisted together without losing unrelated fields

  Scenario: Token-only login commits discard stale expiry
    Given existing credentials have a stale expiry and unrelated authentication fields
    When a caller commits replacement tokens without a lifetime through the public login attempt API
    Then the replacement tokens have no inherited expiry and unrelated fields remain unchanged

  Scenario: Cancellation completes pending device polling without persisting credentials
    Given a direct or split device login has reached an acknowledged pending poll request
    When the login is cancelled before the issuer releases the poll response
    Then the backend returns a cancellation error within a bounded wait without task abortion
    And no credentials are persisted and no subsequent polling or token exchange occurs
    And a cancelled split device session cannot be completed again

  Scenario: Cancellation completes a pending callback listener and releases its resources
    Given a browser callback listener has acknowledged a probe and has an unfinished connection
    When the login is cancelled before any authorization callback arrives
    Then the backend returns a cancellation error within a bounded wait without task abortion
    And the callback port is immediately reusable and the unfinished connection is closed
    And no credentials are persisted and no token exchange occurs
