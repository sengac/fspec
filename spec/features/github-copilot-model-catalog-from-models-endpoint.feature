@authentication
@done
@model-selection
@providers
@rust
@PROV-056
Feature: GitHub Copilot model catalog, provider options & reasoning effort

  """
  Wire format is always OpenAI-shaped — reuse existing OpenAIFspecFacade for wire translation; only the reasoning-variant selection and options transformation differs per family
  CopilotModelCatalogService lives at codelet/providers/src/copilot/models.rs; exposes fetch_models(base_url, token) -> Result<Vec<ModelInfo>>; the function calls /models, parses the response, filters by model_picker_enabled, and maps each entry to ModelInfo via build_model_info — there is NO merge step and NO existing parameter
  CopilotProvider::list_models() (from PROV-055) calls CopilotModelCatalogService.fetch_models() and returns the resulting Vec<ModelInfo>; the TUI model picker consumes this list via the existing provider registry — no models.dev fallback, no static catalog merge
  Provider-level zero-retention enforcement (store: false) lives at codelet/providers/src/copilot/provider_options.rs as a single pure function apply_store_false(options) that is called for ALL Copilot requests irrespective of model — there is no per-model branching
  Reasoning-variant emission lives at codelet/providers/src/copilot/models.rs as part of build_model_info(remote_entry) → ModelInfo: it copies capabilities.supports.reasoning_effort verbatim from the response into ModelInfo.reasoning_variants (Vec<String>); empty/missing → empty Vec; no transformation, no filtering, no family branching
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CopilotModelCatalogService fetches /models from the configured base URL (github.com or enterprise), parses the Zod-equivalent schema, and times out at 5000ms with a single attempt (no retry)
  #   2. release_date is parsed by stripping the `{id}-` prefix from version (e.g. `gpt-4o-2024-05-13` -> `2024-05-13`); if version does not start with id, use version verbatim
  #   3. sdkKey() mapping: @ai-sdk/github-copilot -> copilot; providerOptions keyed under github-copilot are remapped to the copilot key when building outgoing requests
  #   4. store: false is forced in provider options when providerID=github-copilot or api.npm=@ai-sdk/github-copilot (zero-retention enforcement with client-side reasoning replay)
  #   5. The Copilot /models endpoint is the SOLE source of truth for the model catalog: there is no merge with models.dev or any prior cache; each fetch fully replaces the catalog
  #   6. Only remote models with model_picker_enabled=true are included in the catalog returned to callers; entries with model_picker_enabled=false are filtered out
  #   7. ModelInfo build mapping reads exclusively from the remote response: api.id from id, name from name, family from capabilities.family, limits from capabilities.limits, capabilities from capabilities.supports, release_date derived from version; cost is hard-coded to zero; providerID is github-copilot; api.npm is @ai-sdk/github-copilot
  #   8. If capabilities.supports.reasoning_effort is missing or empty for a model, no reasoning variants are emitted (no id-pattern fallback, no date-gating heuristic)
  #   9. Reasoning variants per model are emitted directly from capabilities.supports.reasoning_effort in the /models response: one variant per array entry, in the same order; each variant carries (model_id, reasoning_effort_tier) only — wire-format-specific metadata (e.g. Responses-API extras for GPT-5 family) is added at request time by the PROV-055 wire facades based on capabilities.family from the response, not at catalog time
  #
  # EXAMPLES:
  #   1. /models response includes a model whose capabilities.supports.reasoning_effort is ['low','medium','high','xhigh']; the catalog exposes that model with exactly 4 reasoning variants in tier order low→medium→high→xhigh, each carrying only (model_id, reasoning_effort_tier)
  #   2. /models response includes a model with no capabilities.supports.reasoning_effort field at all; the catalog exposes that model with zero reasoning variants — no fallback inference, no synthesized tiers
  #   3. /models response includes a model whose capabilities.supports.reasoning_effort is an empty array []; the catalog exposes that model with zero reasoning variants
  #   4. /models response includes a model whose version field is 'whatever-model-2025-09-15'; the catalog exposes that model with release_date '2025-09-15' (derived purely by stripping the '{id}-' prefix from the version returned by the endpoint — no model-specific knowledge)
  #   5. /models response returns one entry with model_picker_enabled=true and another entry with model_picker_enabled=false; the catalog exposed to callers contains only the entry whose model_picker_enabled is true
  #   6. First fetch from /models returns two models; the second fetch from /models returns only one of them; after the second fetch the catalog contains only that one model — the missing model is gone, with no merging or fallback to the prior fetch
  #
  # ========================================

  Background: User Story
    As a codelet user authenticated with GitHub Copilot
    I want to see exactly the model catalog the Copilot /models endpoint returns — with the reasoning effort tiers it advertises and zero retention enforced
    So that the model picker reflects what Copilot actually offers right now without any stale or hardcoded model knowledge in codelet

  Scenario: Reasoning effort tiers come straight from the Copilot endpoint, in order
    Given I am authenticated with GitHub Copilot
    And the Copilot /models endpoint returns one model whose capabilities.supports.reasoning_effort is the array ["low","medium","high","xhigh"]
    When the catalog is fetched
    Then the catalog contains that model
    And the model exposes exactly 4 reasoning variants
    And the variants appear in the order "low", "medium", "high", "xhigh"
    And each variant carries only the model id and the reasoning effort tier
    And no variant adds wire-format extras such as reasoningSummary or include fields

  Scenario: Model with no reasoning_effort field exposes no reasoning variants
    Given I am authenticated with GitHub Copilot
    And the Copilot /models endpoint returns one model whose capabilities.supports object has no reasoning_effort field at all
    When the catalog is fetched
    Then the catalog contains that model
    And the model exposes zero reasoning variants
    And no reasoning tiers are inferred from the model id, version, or any other source

  Scenario: Model with empty reasoning_effort array exposes no reasoning variants
    Given I am authenticated with GitHub Copilot
    And the Copilot /models endpoint returns one model whose capabilities.supports.reasoning_effort is an empty array []
    When the catalog is fetched
    Then the catalog contains that model
    And the model exposes zero reasoning variants

  Scenario: release_date is derived purely from the endpoint's version field
    Given I am authenticated with GitHub Copilot
    And the Copilot /models endpoint returns one model with id "whatever-model" and version "whatever-model-2025-09-15"
    When the catalog is fetched
    Then the catalog model "whatever-model" has release_date "2025-09-15"
    And the release_date is derived by stripping the "{id}-" prefix from the version field returned by the endpoint
    And no hardcoded date table is consulted

  Scenario: Models flagged model_picker_enabled=false are filtered out
    Given I am authenticated with GitHub Copilot
    And the Copilot /models endpoint returns one entry with model_picker_enabled set to true
    And the same response includes a second entry with model_picker_enabled set to false
    When the catalog is fetched
    Then the catalog contains the entry whose model_picker_enabled is true
    And the catalog does not contain the entry whose model_picker_enabled is false

  Scenario: Each fetch fully replaces the catalog with no merging or fallback
    Given I am authenticated with GitHub Copilot
    And the first fetch from the Copilot /models endpoint returns two models with ids "model-a" and "model-b"
    When the catalog is fetched
    Then the catalog contains "model-a" and "model-b"
    Given the next fetch from the Copilot /models endpoint returns only "model-a"
    When the catalog is fetched again
    Then the catalog contains only "model-a"
    And "model-b" is gone with no merging into the prior fetch
    And no fallback to models.dev or any other source occurs

  Scenario: store: false is enforced for every Copilot request regardless of model
    Given I am authenticated with GitHub Copilot
    And the catalog has been populated from the Copilot /models endpoint with multiple models
    When provider options are built for any Copilot model in the catalog
    Then the resulting provider options contain store: false
    And the store: false flag is applied uniformly without inspecting the model id or family
