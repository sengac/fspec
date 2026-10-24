@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex refresh races preserve the current credential authority
  """
  PROV-159 acceptance rows A5-A9. Primary tests: rust/providers/tests/prov159_refresh_races.rs.
  Serialize refresh by credential authority, reload a winning same-account rotation and fence commits after network work by account AND login generation. RPC, NAPI and legacy refresh delegate to shared codelet-providers policy.
  Use isolated CODEX_HOME, shared test helpers, controlled keyring fixtures, real loopback issuers and deterministic barriers. Supplementary cross-process, entry-point and lifecycle regressions do not add primary mappings in other physical files.
  Research: spec/attachments/PROV-159/prov159-oauth-research.md and its two upstream companions.
  """

  Scenario: A5 Independent clients coordinate refresh of rotating tokens
    Given two independently constructed Codex clients share expired credentials with rotating refresh token R0
    When both clients request fresh credentials concurrently
    Then the issuer receives only one exchange of refresh token R0
    And both clients use the winning refreshed credentials
    And renewed freshness is shared even when the issuer returns unchanged token strings
    And persisted credentials contain the winning rotation and retain independent API keys and unrelated fields

  Scenario: A6 Shared-home clients reload external rotations without crossing accounts
    Given a Codex client cached account A refresh token R0 and another process sharing CODEX_HOME persisted rotation R1
    When the cached client encounters token expiry or a rejected access token
    Then the client reloads and reuses newer same-account credentials before attempting an exchange of stale R0
    And a concurrent rotation by another cooperating process cannot overwrite the winning credentials
    And an externally persisted different account causes a clear reauthentication outcome rather than silent account adoption
    And independent CODEX_HOME directories remain isolated and shared-home credential compatibility is retained
    And an expired access-token claim is not masked by stale future expiry metadata

  Scenario: A7 Late refresh cannot resurrect logout
    Given a refresh of Codex account A is paused after its network request starts
    When local logout completes before the delayed refresh response is released
    Then persisted and live OAuth state remain logged out after the refresh finishes
    And the delayed refresh cannot authorize a later HTTP or WebSocket request
    And the same fencing holds for provider, RPC, NAPI and legacy refresh entry points
    And independent API keys and unrelated credential fields are preserved

  Scenario: A8 Late refresh cannot overwrite a different-account login
    Given a refresh of Codex account A is paused after its network request starts
    When logout and login as account B complete before the delayed account A refresh response is released
    Then storage, the settings UI and newly authorized requests remain bound to account B
    And existing account A clients reauthenticate or deliberately rebind without reusing account A state
    And the same fencing holds for provider, RPC, NAPI and legacy refresh entry points
    And independent API keys and unrelated credential fields are preserved

  Scenario: A9 Same-account login supersedes older refresh generations
    Given a refresh from Codex account A login generation A1 is paused after its network request starts
    When a new login generation A2 for the same account commits before the delayed A1 response is released
    Then stored and live credentials remain those of login generation A2
    And an identical account ID does not permit the older refresh to commit
    And the same fencing holds for NAPI, provider, RPC and legacy refresh entry points
    And independent API keys and unrelated credential fields are preserved
