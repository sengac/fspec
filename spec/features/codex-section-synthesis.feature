@done
@ts-parity
@model-selection
@providers
@PROV-129
Feature: Synthesize Codex (ChatGPT) section by re-parenting OpenAI cloud models (TS parity)
  """
  Fix location: rust/sessions/src/handle_impl.rs list_providers() — insert a Codex synthesis step over the assembled cloud sections BEFORE retain_populated_cloud_sections (PROV-127) and before local profiles are appended.
  New helpers in rust/sessions/src/cloud_models.rs: codex_reparented_models(registry) = cloud_model_entries(registry, "openai", true) (the full OpenAI models.dev catalog, no allowlist filtering); synthesize_codex_section(sections, registry) gated on provider_has_credentials("codex") (reuses PROV-128 predicate — the OpenAI catalog is sourced Codex-gated, independent of OPENAI_API_KEY, which fixes #6). When non-empty it fills the existing 'codex' ProviderInfo header and removes the standalone 'openai' section. Mirrors TS cloudSectionBuilder.ts extractCodexSection (:191-237) + the openai.hasCredentials override (:117-119).
  Out of scope: 'Codex section pushed first' ordering (PROV-130). This card fills the existing codex header in place; it does not reorder sections. No unwrap/panic on the production path.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When Codex OAuth or a Codex API key is present, the OpenAI cloud models are re-parented under a single synthetic 'Codex (ChatGPT)' section
  #   2. The re-parented OpenAI model list is the FULL OpenAI models.dev catalog (no allowlist filtering), ordered newest-first by release date
  #   3. When the Codex (ChatGPT) section is synthesized, the standalone OpenAI API section is not rendered (models moved, not duplicated)
  #   4. Codex synthesis reuses the PROV-128 unified credential predicate (provider_has_credentials("codex")) so Codex OAuth alone yields a NON-empty selectable model list
  #   5. When no Codex credentials are present, no re-parenting occurs. PROV-146: the standalone OpenAI API section is NEVER populated from the cloud catalog regardless of OPENAI_API_KEY — it is suppressed entirely (the openai provider is for local OpenAI-protocol servers only)
  #
  # EXAMPLES:
  #   1. Codex OAuth file present and no OPENAI_API_KEY: the Codex (ChatGPT) section lists the full OpenAI models.dev catalog and the OpenAI API section is absent (fixes #6)
  #   2. Any models.dev OpenAI model (e.g. gpt-5-mini, gpt-4o, o3) appears in the Codex (ChatGPT) section — there is no allowlist
  #   3. Two OpenAI models present: they appear in Codex (ChatGPT) ordered newest-first by release date
  #   4. No Codex creds, only OPENAI_API_KEY: the standalone OpenAI API section is NOT shown (PROV-146 reversal — the openai cloud section is never populated from the catalog; the openai provider is for local OpenAI-protocol servers only) and no Codex (ChatGPT) section is synthesized
  #
  # ========================================
  Background: User Story
    As a developer signed in with Codex (ChatGPT) OAuth
    I want to see my available OpenAI models grouped under a single Codex (ChatGPT) section
    So that I can actually select a model to run instead of facing empty OpenAI and Codex sections

  Scenario: Codex OAuth alone yields a populated Codex (ChatGPT) section
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And no OPENAI_API_KEY is set in the environment
    And the models.dev catalog offers OpenAI models
    When I open the model selector
    Then a "Codex (ChatGPT)" section is shown with at least one selectable model
    And no standalone "OpenAI API" section is shown

  Scenario: All OpenAI models from models.dev appear in Codex (ChatGPT)
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And the models.dev catalog offers the OpenAI models "gpt-5.4", "gpt-5-mini" and "gpt-4o"
    When I open the model selector
    Then the "Codex (ChatGPT)" section lists "gpt-5.4"
    And the "Codex (ChatGPT)" section lists "gpt-5-mini"
    And the "Codex (ChatGPT)" section lists "gpt-4o"

  Scenario: Codex (ChatGPT) models are ordered newest-first by release date
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And the models.dev catalog offers the OpenAI models "gpt-5.4" and "gpt-5.2-codex"
    When I open the model selector
    Then the "Codex (ChatGPT)" section lists "gpt-5.4" before "gpt-5.2-codex"

  Scenario: Without Codex credentials the standalone OpenAI API section is suppressed
    Given I have no Codex credentials
    And only an OPENAI_API_KEY is set in the environment
    And the models.dev catalog offers OpenAI models
    When I open the model selector
    Then no standalone "OpenAI API" cloud section is shown
    And no "Codex (ChatGPT)" section is synthesized
