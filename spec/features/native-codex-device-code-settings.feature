@done
@BUG-279
@providers
@authentication
@oauth
@integration
@validation
@critical
Feature: Native Codex device-code settings
  """
  Decomposed from the original BUG-279 feature only to preserve Fspec one-feature/one-test-file traceability. No new criteria: synthetic provider listing uses the real projection; actual menu Enter and settings actions call production embedded RPC and native start/poll/exchange against loopback official-protocol endpoints. Preserve LoginAttempt ownership, shared 300-second deadline, account extraction and credential persistence. User-visible error text must not include server bodies or secrets. Original causal RED precedes all production changes.
  """

  Background: User Story
    As a Codex user
    I want device login through native settings
    So that authorization succeeds safely or reports a clear redacted error

  Scenario: Native settings completes official device authorization
    Given a native Codex settings login backed by isolated loopback authorization
    When the user selects device login and completes the official protocol
    Then settings shows success and only the authorized synthetic credentials are persisted

  Scenario: Native settings reports redacted errors and permits retry
    Given a device authorization server returning a secret bearing start failure
    When native settings starts device login and the user retries after failure
    Then settings displays a useful redacted error and a fresh authorized retry can succeed
