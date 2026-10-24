@done
@PROV-159
@phase-1
@tui
@authentication
@oauth
@rust
Feature: Provider settings safely reports failed OAuth logout
  """
  Supplemental UI regression for A16; its primary storage acceptance scenario remains in prov159-credential-storage.feature.
  Primary tests: rust/fspec-tui/tests/prov159_settings_failures.rs. The backend reports a failure containing a secret to verify safe presentation.
  """

  Scenario: Failed logout reports a sanitized settings error
    Given the OAuth logout backend returns an error containing a secret token
    When provider settings completes the logout action
    Then settings reports logout failure without exposing backend internals or the secret token
