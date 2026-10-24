@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
@tui
@provider-settings
Feature: Codex local OAuth logout clears all credential sources
  """
  PROV-159 acceptance row A1. Primary test: rust/fspec-tui/tests/prov159_oauth_logout.rs.
  Route embedded logout through RPC to the shared codelet-providers authority. Clear preferred and fallback OAuth credentials idempotently while preserving independent API keys and unrelated document fields.
  Use shared isolated-home fixtures and a controlled keyring boundary; never touch the real user keychain. Local logout does not revoke remote sessions or clear browser cookies.
  Research: spec/attachments/PROV-159/prov159-oauth-research.md and its two upstream companions.
  """

  Scenario: A1 Local logout clears every OAuth source and preserves the API key
    Given Codex account A exists in the preferred keychain and file fallback alongside an independent OpenAI API key
    When the user logs out of Codex OAuth and credentials are read again after restarting
    Then neither credential source restores OAuth account A
    And the independent OpenAI API key remains available
    And repeating local logout succeeds without restoring OAuth credentials
