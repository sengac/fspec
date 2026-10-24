@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex live clients observe authoritative authentication and freshness
  """
  PROV-159 acceptance rows A4 and A14. Primary tests: rust/providers/tests/prov159_live_clients.rs.
  HTTP and WebSocket consumers must reject revoked account/generation bindings and discard account-scoped connections and continuation state. Rebinding must be deliberate, never silent account retention.
  Freshness uses a 30-second buffer and the earliest usable stored/JWT expiry. If both are absent or malformed, use last_refresh plus a one-hour lifetime when valid, otherwise immediately coordinate refresh. Unverified claims only inform freshness, never authorization.
  Use isolated CODEX_HOME, controlled keyring fixtures and loopback HTTP/WebSocket endpoints. Supplementary refresh-policy and freshness tests remain supplementary.
  Research: spec/attachments/PROV-159/prov159-oauth-research.md and its two upstream companions.
  """

  Scenario: A4 Existing HTTP and WebSocket clients cannot retain a logged-out account
    Given existing Codex HTTP and WebSocket clients have authenticated requests and continuation state for account A
    When the user logs out and completes login as account B before another request
    Then no subsequent HTTP request or WebSocket message uses account A credentials
    And each existing client deliberately rebinds to account B or returns a clear reauthentication error
    And account A connections, continuation identifiers and routing state are not reused for account B
    And requests between logout and the new login fail without sending account A credentials

  Scenario: A14 Disk-loaded credentials follow buffered freshness without refresh storms
    Given shared Codex credentials have fresh, near-expiry, expired, missing or malformed expiry metadata
    When independent providers load those credentials and prepare requests
    Then fresh credentials outside the 30-second safety window are reused without contacting the issuer
    And near-expiry and expired credentials are refreshed through the shared coordination policy
    And missing or malformed expiry follows a documented conservative freshness fallback
    And provider construction does not cause independent clients to refresh the same credentials repeatedly
    And unverified token claims are not used as proof of authorization
