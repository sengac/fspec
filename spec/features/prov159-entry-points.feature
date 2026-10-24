@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex shared and legacy refresh entry points respect lifecycle fences
  """
  Supplemental regressions for A7, A8, A9 and A14; primary acceptance scenarios remain in their original PROV-159 features.
  Primary tests: rust/providers/tests/prov159_entry_points.rs. Shared and legacy paths use real isolated storage and acknowledged loopback refresh barriers.
  """

  Scenario: Shared refresh reuses fresh disk credentials without exchange
    Given fresh persisted credentials are available to shared refresh
    When shared refresh is requested
    Then existing tokens are reused without a refresh or exchange request

  Scenario: Legacy exchange reuses fresh disk credentials
    Given fresh persisted credentials are available to legacy API key exchange
    When legacy API key exchange is requested
    Then existing tokens are reused with only the API key exchange request

  Scenario: Entry point refreshes cannot resurrect logout
    Given shared and legacy entry points have acknowledged pending refreshes
    When logout completes before the refresh responses are released
    Then both entry points reject stale completion without restoring tokens or exchanging an API key

  Scenario: Entry point refreshes cannot overwrite a different account
    Given shared and legacy entry points have acknowledged pending refreshes
    When a different account login completes before the refresh responses are released
    Then both entry points reject stale completion and preserve the replacement credentials without API key exchange

  Scenario: Entry point refreshes cannot overwrite a same-account login
    Given shared and legacy entry points have acknowledged pending refreshes
    When a same-account replacement login completes before the refresh responses are released
    Then both entry points reject stale completion and preserve the replacement credentials without API key exchange
