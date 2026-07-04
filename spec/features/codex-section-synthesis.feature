@done
@ts-parity
@model-selection
@providers
@PROV-129
Feature: Synthesize Codex (ChatGPT) section by re-parenting + allowlist-filtering OpenAI cloud models (TS parity)

  """
  Fix location: codelet/sessions/src/handle_impl.rs list_providers() — insert a Codex synthesis step over the assembled cloud sections BEFORE retain_populated_cloud_sections (PROV-127) and before local profiles are appended.
  New: codelet/sessions/src/codex_allowlist.rs ports src/tui/services/codexAllowlistService.ts (matchesCodexAllowlist/filterByCodexAllowlist). Allowlist SOURCE is the bundled default src/tui/data/codex-models.json embedded via include_str! (single source of truth with the TS build), with a ~/.fspec/codex-models.json user override. Matching = exact slug OR slug + '-' + date suffix (YYYY-MM-DD or YYYYMMDD), visibility must equal 'list'; results sorted by ascending priority.
  New helpers in codelet/sessions/src/cloud_models.rs: codex_reparented_models(registry) = filter_by_codex_allowlist(cloud_model_entries(registry, "openai", true)); synthesize_codex_section(sections, registry) gated on provider_has_credentials("codex") (reuses PROV-128 predicate — the OpenAI catalog is sourced Codex-gated, independent of OPENAI_API_KEY, which fixes #6). When non-empty it fills the existing 'codex' ProviderInfo header and removes the standalone 'openai' section. Mirrors TS cloudSectionBuilder.ts extractCodexSection (:191-237) + the openai.hasCredentials override (:117-119).
  Out of scope: 'Codex section pushed first' ordering (PROV-130). This card fills the existing codex header in place; it does not reorder sections. No unwrap/panic on the production path — allowlist load failures degrade to showing all OpenAI models (TS parity).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When Codex OAuth or a Codex API key is present, the OpenAI cloud models are re-parented under a single synthetic 'Codex (ChatGPT)' section
  #   2. The re-parented OpenAI model list is filtered to the Codex allowlist (only visibility='list' slugs, matched exactly or by a date-suffixed variant) and sorted by allowlist priority
  #   3. When the Codex (ChatGPT) section is synthesized, the standalone OpenAI API section is not rendered (models moved, not duplicated)
  #   4. Codex synthesis reuses the PROV-128 unified credential predicate (provider_has_credentials("codex")) so Codex OAuth alone yields a NON-empty selectable model list
  #   5. When no Codex credentials are present, no re-parenting occurs and the standalone OpenAI API section behaves exactly as before (API-key gated)
  #
  # EXAMPLES:
  #   1. Codex OAuth file present and no OPENAI_API_KEY: the Codex (ChatGPT) section lists the allowlisted OpenAI models and the OpenAI API section is absent (fixes #6)
  #   2. A models.dev OpenAI model that is not in the allowlist (e.g. gpt-5-mini) is excluded from Codex (ChatGPT); an allowlisted one (e.g. gpt-5.4) is kept
  #   3. Two allowlisted models present: they appear in Codex (ChatGPT) ordered by allowlist priority (gpt-5.4 priority 0 before gpt-5.2-codex priority 3)
  #   4. No Codex creds, only OPENAI_API_KEY: the OpenAI API section is shown with its catalog and no Codex (ChatGPT) section is synthesized
  #   5. A hidden allowlist entry (visibility='hide', e.g. gpt-5.1-codex) does not match, so that model is excluded from Codex (ChatGPT) even though its slug is in the allowlist
  #
  # ========================================

  Background: User Story
    As a developer signed in with Codex (ChatGPT) OAuth
    I want to see my available OpenAI models grouped under a single Codex (ChatGPT) section
    So that I can actually select a model to run instead of facing empty OpenAI and Codex sections

  Scenario: Codex OAuth alone yields a populated Codex (ChatGPT) section
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And no OPENAI_API_KEY is set in the environment
    And the models.dev catalog offers OpenAI models including allowlisted ones
    When I open the model selector
    Then a "Codex (ChatGPT)" section is shown with at least one selectable model
    And no standalone "OpenAI API" section is shown

  Scenario: Non-allowlisted OpenAI models are excluded from Codex (ChatGPT)
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And the models.dev catalog offers the OpenAI models "gpt-5.4" and "gpt-5-mini"
    When I open the model selector
    Then the "Codex (ChatGPT)" section lists "gpt-5.4"
    And the "Codex (ChatGPT)" section does not list "gpt-5-mini"

  Scenario: Allowlisted Codex models are ordered by allowlist priority
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And the models.dev catalog offers the OpenAI models "gpt-5.4" and "gpt-5.2-codex"
    When I open the model selector
    Then the "Codex (ChatGPT)" section lists "gpt-5.4" before "gpt-5.2-codex"

  Scenario: A hidden allowlist entry is excluded from Codex (ChatGPT)
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And the models.dev catalog offers the OpenAI model "gpt-5.1-codex" that is a hidden allowlist entry
    When I open the model selector
    Then the "Codex (ChatGPT)" section does not list "gpt-5.1-codex"

  Scenario: Without Codex credentials the standalone OpenAI API section is preserved
    Given I have no Codex credentials
    And only an OPENAI_API_KEY is set in the environment
    And the models.dev catalog offers OpenAI models
    When I open the model selector
    Then a standalone "OpenAI API" section is shown with its models
    And no "Codex (ChatGPT)" section is synthesized
