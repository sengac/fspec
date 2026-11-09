@BUG-582
@done
@rust
@test-maintenance
@providers
@BUG-550
Feature: Providers lib tests race on FSPEC_HOME across modules
  """
  serial_test's default key is the one guard because it already serialises most env tests in the lib and works for async tests; a std Mutex guard held across .await trips clippy::await_holding_lock. The copilot, manager and oauth tests can't pass the home directory explicitly: read/write_copilot_auth and the cred:: module resolve it only from FSPEC_HOME (production API), so they keep the env redirect under the guard.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every providers lib test that changes FSPEC_HOME, HOME, CODEX_HOME, FSPEC_USER_DIR or a provider credential or limit env var, directly or through a helper or fixture, runs under the one shared guard: serial_test's default #[serial] key
  #   2. No module-local environment lock (a private Mutex, an env_lock() helper or a keyed #[serial(name)] group) exists in the providers lib tests, because separate locks do not exclude each other
  #   3. The FSPEC_HOME redirect used by the lib tests is one shared fixture (test_fixtures::env), not a copy per module
  #   4. A source guard over the providers lib test code fails when a test that changes the environment, directly or through a same-file helper or a shared env fixture, lacks the default #[serial] attribute, or when a module-local lock or keyed serial group appears
  #
  # EXAMPLES:
  #   1. cred_path_returns_absolute_path_without_side_effects (oauth) and test_copilot_proxy_model_gets_per_model_context_from_registry (manager) both redirect FSPEC_HOME; under the shared guard neither sees the other's directory
  #   2. A test that calls a same-file helper which sets OPENAI_API_KEY, without #[serial], is reported by the guard with its file and name
  #
  # ========================================
  Background: User Story
    As a maintainer running the providers lib tests
    I want to have every test that changes the process environment exclude every other such test
    So that the lib suite gives the same result under any parallelism or load

  Scenario: Tests in different modules redirect FSPEC_HOME through one shared fixture
    Given the oauth, copilot and manager lib tests redirect FSPEC_HOME through the shared env fixture under the default serial guard
    When a test redirects FSPEC_HOME and resolves the credential paths
    Then the cred, Copilot and Claude credential paths all resolve inside its own directory, and the previous FSPEC_HOME is back when the fixture is dropped

  Scenario: Every providers lib test that changes the environment runs under the shared guard
    Given the providers lib test sources
    When the guard scans every test that changes the environment directly, through a same-file helper or through a shared env fixture
    Then each such test carries the default #[serial] attribute, and no module-local lock or keyed serial group exists

  Scenario: The guard reports a test that changes the environment through a helper without the shared guard
    Given a lib test source whose test calls a same-file helper that sets OPENAI_API_KEY and has no #[serial] attribute
    When the guard scans it
    Then the test is reported with its file and name
