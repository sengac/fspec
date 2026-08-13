@done
@model-selection
@providers
@model
@provider
@rust
@PROV-125
Feature: Cloud providers show empty model lists due to slug/models.dev-id key mismatch
  """
  Fix lives in rust/sessions/src/cloud_models.rs: canonical_to_models_dev() must map every canonical slug whose models.dev key differs. Confirmed divergences from live models.dev/api.json: gemini->google (existing), together->togetherai, moonshot->moonshotai. Verify against the cached catalog keys, do not guess.
  cloud_model_entries() currently swallows registry.list_models() errors with `Err(_) => return Vec::new()`. Replace with a branch that distinguishes a genuine absence (return empty) from a diagnosable miss by logging via the crate's tracing/log facility before returning empty. Do not use println!/eprintln! (production code).
  Distinguishing expected-absent from diagnosable-miss: registry.list_models returns Err(Unknown provider) for BOTH. Maintain an explicit known-not-on-models.dev set (codex, github-copilot, galadriel) that returns empty silently. Any other slug that misses must log tracing::warn! (diagnosable divergence) then return empty. This keeps rule 3 and rule 4 from conflicting.
  Test approach: unit-test cloud_model_entries() and canonical_to_models_dev() directly in rust/sessions/tests/ (see existing rpc073_cloud_model_catalog.rs). Build a ModelRegistry from a fixture ModelsDevResponse containing togetherai/moonshotai/google keys with tool_call models, and assert the canonical slugs together/moonshot/gemini resolve to non-empty entries. No network calls in tests.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A credentialed provider that exists in the models.dev catalog must show its tool-call-capable, non-deprecated models, regardless of whether its canonical slug equals the models.dev provider key
  #   2. Canonical provider slugs that differ from their models.dev key must be mapped to the correct models.dev key before registry lookup (e.g. together->togetherai, moonshot->moonshotai, gemini->google)
  #   3. A registry lookup miss (models.dev has no such key) must not be swallowed silently; it must be surfaced (logged) so future slug/key divergences are diagnosable rather than appearing as an empty provider
  #   4. A provider legitimately absent from the models.dev catalog (e.g. codex, galadriel) must still yield an empty model list without error
  #
  # EXAMPLES:
  #   1. Given credentials for the 'together' provider, when the model list is built, the selector shows Together AI's 25 tool-call models (registry key 'togetherai')
  #   2. Given credentials for the 'moonshot' provider, when the model list is built, the selector shows Moonshot's tool-call models (registry key 'moonshotai')
  #   3. Given credentials for 'gemini', when the model list is built, Google Gemini's tool-call models still appear (existing gemini->google mapping preserved)
  #   4. Given credentials for 'codex' (absent from models.dev), when the model list is built, the provider yields an empty list with no error
  #   5. Given a credentialed canonical slug that is not in the known-absent set and misses the registry, when the model list is built, a tracing warning is emitted and an empty list is returned
  #
  # ========================================
  Background: User Story
    As a fspec user selecting a cloud model
    I want to see every credentialed provider's tool-capable models in the model selector
    So that I can pick models from providers like Together AI and Moonshot instead of staring at empty rows

  Scenario: Together provider resolves its models.dev catalog despite a differing key
    Given the models.dev registry contains tool-call models under the key "togetherai"
    And credentials are configured for the "together" provider
    When the cloud model list is built for "together"
    Then the returned model list is not empty
    And every returned model is tool-call-capable and not deprecated

  Scenario: Moonshot provider resolves its models.dev catalog despite a differing key
    Given the models.dev registry contains tool-call models under the key "moonshotai"
    And credentials are configured for the "moonshot" provider
    When the cloud model list is built for "moonshot"
    Then the returned model list is not empty
    And every returned model is tool-call-capable and not deprecated

  Scenario: Gemini provider still resolves via the existing gemini to google mapping
    Given the models.dev registry contains tool-call models under the key "google"
    And credentials are configured for the "gemini" provider
    When the cloud model list is built for "gemini"
    Then the returned model list is not empty

  Scenario: A provider absent from models.dev yields an empty list without warning
    Given the "codex" provider is not present in the models.dev registry
    And credentials are configured for the "codex" provider
    When the cloud model list is built for "codex"
    Then the returned model list is empty
    And no diagnostic warning is logged

  Scenario: An unexpected registry miss logs a diagnostic warning and returns empty
    Given a credentialed provider slug that is not in the known-absent set
    And that slug is missing from the models.dev registry
    When the cloud model list is built for that slug
    Then a diagnostic warning is logged
    And the returned model list is empty
