@done
@BUG-548
@BUG-448
@providers
@test-maintenance
@rust
Feature: The test-only stub provider resolves the limits it declares
  """
  Tests depended on the developer machine: real ~/.fspec model profiles
  (openai:spark), credentials, catalog limits and test order. The fixes:
  stub credentials and seeded catalogs where tests create sessions,
  declared profile model limits (PROV-171), an isolated user directory and
  start workspace for resume_session_removal_bug, self-contained git021,
  every mux004 test rooting its own data directory, provider source pins
  that read module trees, and test expectations that follow PROV-174 and
  BUG-332 (complete envelopes carry requestId).
  The one product change: the test-support stub provider ("stub/canned")
  registers as a custom provider, and PROV-165 gives custom providers no
  default limits, so every stub turn was refused with "context window
  unavailable for stub/canned" (bug216, bug220, rig015, rpc072, rpc386).
  Under the test-support feature the provider manager resolves the stub's
  declared limits (STUB_CONTEXT_WINDOW / STUB_MAX_OUTPUT_TOKENS); production
  builds are unchanged.
  Tests: rust/providers/tests/bug448_stub_provider_limits.rs.
  BUG-548: the stub test still built its manager with ProviderManager::new(), which counts the
  machine's Claude and Codex logins and refuses to pick one when several exist. It now builds the
  manager for the stub provider explicitly under credential homes (FSPEC_HOME, CODEX_HOME,
  FSPEC_USER_DIR) that hold both logins. Tests in cli and fspec-tui that picked the first
  credentialed provider, called Session::new(None) and skipped without a provider, or needed real
  Codex logins now build an explicitly selected offline session. A source guard over every crate's
  test files keeps it that way.
  Tests: rust/providers/tests/bug548_tests_ignore_machine_credentials.rs.
  """

  Background: User Story
    As a maintainer running the test suites on any machine
    I want to have tests use stub credentials, seeded catalogs, explicit limits and their own config directories
    So that the suites pass or fail the same way on every machine

  Scenario: The stub provider's declared limits are resolved
    Given the credential homes hold Claude and Codex logins and the test-only stub provider is registered and selected
    When its context window and output limit are resolved
    Then they are the stub's declared 200000 and 4096 tokens, with no ambiguous-provider error

  Scenario: Offline test sessions select their provider explicitly and run
    Given the test files of every crate
    When the guard scans them for tests that skip without a provider or pick the first credentialed provider in a loop
    Then none is found, because offline sessions select their provider explicitly and run

  Scenario: The source guard reports tests that read the machine's credential set
    Given the test files of every crate
    When the guard scans them for ProviderManager::new(), Session::new(None) and Codex sessions
    Then every such use is in a file that points FSPEC_HOME and CODEX_HOME at a test directory
