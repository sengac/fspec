@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex provider deletion clears OAuth without damaging shared credentials
  """
  PROV-159 acceptance row A3. Primary test: rust/fspec-tui/tests/prov159_provider_deletion.rs.
  Session-manager provider deletion delegates to the shared credential authority. Remove preferred and fallback OAuth credentials without destroying independent API keys or unrelated fields in the shared Codex document.
  Use shared isolated-home helpers and a controlled keyring boundary; never access real credentials. Provider-only deletion probes remain supplementary.
  Research: spec/attachments/PROV-159/prov159-oauth-research.md and its two upstream companions.
  """

  Scenario: A3 Provider deletion clears preferred and fallback OAuth credentials
    Given the Codex provider uses OAuth account A from the keychain with a file fallback
    When the user deletes the Codex provider and credentials are detected again after restarting
    Then neither the keychain nor the file fallback restores the deleted OAuth account
    And an independent OpenAI API key outside the deleted provider remains unchanged
    And unrelated fields in the shared Codex credential document remain unchanged
