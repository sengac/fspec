@done
@PROV-159
@phase-1
@providers
@authentication
@oauth
@rust
Feature: Codex OAuth credential storage authority
  """
  PROV-159 acceptance rows A2, A15 and A16. Primary tests: rust/providers/tests/prov159_credential_storage.rs.
  Centralize preferred-keyring and file-fallback lifecycle policy in codelet-providers. Preserve independent API keys and unrelated document fields; coordinate complete private replacements with account and login-generation fencing.
  Use shared test helpers, isolated CODEX_HOME and a controlled keyring boundary; never access real credentials. Supplementary entry-point and settings tests do not replace these primary mappings.
  Research: spec/attachments/PROV-159/prov159-oauth-research.md and its upstream storage/refresh and login/lifecycle companions.
  """

  Scenario: A2 Browser and device login replace the preferred account consistently
    Given Codex account A exists in the preferred keychain and file fallback
    When the user completes browser or device login as account B
    Then the settings UI, a newly constructed provider and a restarted application select account B
    And no preferred or fallback source restores account A
    And independent API keys and unrelated credential fields are preserved

  Scenario: A15 Credential persistence is complete private and failure-safe
    Given parallel readers and lifecycle writers share an existing Codex credential document with independent API keys and unrelated fields
    When credentials are replaced and a persistence failure is injected before replacement completes
    Then every successful read observes a complete valid credential document
    And the failed replacement reports an error and leaves the prior valid document intact
    And successful updates preserve independent API keys and unrelated fields
    And new credential files have Unix mode 0600 and replacement does not expose credentials through temporary files
    And credential replacement respects concurrent logout and newer login transitions

  Scenario: A16 Keyring failures are reported without hidden account fallback
    Given the preferred Codex keyring contains account A and the fallback file contains account B
    When a keyring read, login save, OAuth clear or provider deletion fails
    Then the operation reports a clear storage failure rather than success
    And a keyring failure is not treated as missing credentials or permission to select fallback account B
    And the settings UI does not claim successful logout or login while the storage operation has failed
    And partial cleanup failures identify the incomplete operation without exposing credential contents
