@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex login cancellation and local SSO isolation
  """
  PROV-159 acceptance rows A10 and A17. Primary tests: rust/providers/tests/prov159_login_lifecycle.rs. A11's primary coverage belongs to prov159-rpc-lifecycle, not the supplementary provider-only port-reuse test here.
  Backend attempts own cancellation, resource cleanup, supersession and account/generation-fenced commits. UI generation guards supplement backend cancellation, not replace it.
  Local logout neither clears browser SSO cookies nor promises an account chooser or remote revocation. Reauthorization may legitimately choose the same account.
  Use shared isolated-home helpers, controlled keyring fixtures, loopback issuers and deterministic pending exchanges; never live credentials.
  Research: spec/attachments/PROV-159/prov159-oauth-research.md and its two upstream companions.
  """

  Scenario: A10 Cancellation stops browser and device authentication at every pending phase
    Given a Codex login attempt is waiting for a browser callback, polling a device code, exchanging tokens or awaiting persistence
    When the user cancels the attempt, logs out or starts a replacement login before releasing the pending operation
    Then the cancelled attempt does not persist credentials or report successful login
    And its callback listener, polling tasks and pending exchange work are stopped and cleaned up
    And cancelling an older attempt cannot cancel its replacement
    And UI generation guards also discard late results from the cancelled attempt

  Scenario: A17 Local logout is isolated from retained browser SSO
    Given Codex account A is logged in locally and the browser retains its OpenAI SSO session
    When the user logs out locally and completes a new browser authorization using the retained SSO session
    Then the new login may legitimately select the same account without another password prompt
    And the new local login supersedes the old credential generation and cancelled attempts
    And old local credentials and account-scoped client state cannot authorize new requests after logout
    And local logout does not claim to clear browser cookies, force an account chooser or revoke remote sessions
