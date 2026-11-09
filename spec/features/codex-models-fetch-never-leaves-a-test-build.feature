@done
@providers
@rust
@testing
@BUG-544
Feature: The Codex /models fetch never leaves a test build
  """
  codelet-providers codex/models_catalog/fetch.rs: under cfg(feature = "test-support"), base_url() returns Result and errs when no redirect is installed; fetch_for_selection returns that error before building the request. Non-test-support build unchanged. New feature codex-models-fetch-never-leaves-a-test-build, test rust/providers/tests/bug544_codex_models_fetch_is_hermetic.rs (temp CODEX_HOME with fake tokens, wiremock-style local server for the redirect case).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. In a build with codelet-providers' test-support feature (every test crate that creates Codex sessions enables it), the Codex /models fetch goes only to the redirect a test installed (models_catalog::test_support::redirect); with no redirect it fails at once, before any request, with a reason naming the redirect, which is the selection's no-reasoning-control reason
  #   2. Builds without test-support (the shipped binaries) are unchanged: they fetch from https://chatgpt.com/backend-api/codex
  #   3. Tests that relied on a real /models answer would now see that reason; every test that needs Codex models installs a local mock and the redirect (the existing MODEL-014/PROV-176 tests do)
  #
  # EXAMPLES:
  #   1. a test signs Codex in with fake ChatGPT tokens in a temp CODEX_HOME and selects a Codex model without a redirect: the fetch fails immediately with 'test builds never send the Codex /models request to chatgpt.com: install models_catalog::test_support::redirect' and nothing is sent
  #   2. the same test with a redirect to a local mock: the mock receives GET /models?client_version=0.165.0
  #
  # ========================================
  Background: User Story
    As a fspec developer running the test suites
    I want to have no test reach chatgpt.com when it creates a Codex session or lists Codex models
    So that tests are hermetic: they never use my real Codex sign-in, never depend on the network and never wait out a 10 s timeout

  Scenario: Without a redirect a test build sends no Codex models request
    Given Codex is signed in with ChatGPT in a temporary Codex home
    And no test redirect is installed
    When the Codex models are fetched for a model selection
    Then the fetch fails at once with a reason naming the test redirect

  Scenario: With a redirect the Codex models request goes to the local mock
    Given Codex is signed in with ChatGPT in a temporary Codex home
    And a test redirect to a local mock is installed
    When the Codex models are fetched for a model selection
    Then the local mock receives the models request with the pinned client version
