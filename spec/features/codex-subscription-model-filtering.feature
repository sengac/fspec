@done
@PROV-034
Feature: Filter models.dev catalog to only show models available in Codex subscription
  """
  Create a codex-models.json config file (or equivalent data source) in the provider config directory that lists the Codex-supported model slugs. This file is read by modelInitializationService.ts during extractCodexSection() to filter the models.dev catalog. When Codex OAuth is active, only models whose slug matches (or is prefixed by) an entry in this allowlist pass through to the Codex (ChatGPT) section.
  The initial allowlist should be seeded from the research analysis of the openai/codex repository (see attached codex-model-catalog-research.md). The 12 model slugs from codex-rs/core/models.json become the baseline: gpt-5.3-codex, gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.1-codex, gpt-5.2, gpt-5.1, gpt-5-codex, gpt-5, gpt-oss-120b, gpt-oss-20b, gpt-5.1-codex-mini, gpt-5-codex-mini. These should also carry priority and visibility metadata from the Codex catalog.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When Codex OAuth is active, the model selector must only display models that exist in the Codex-supported catalog — not the full models.dev OpenAI provider list. Models like o3-pro, o4-mini, gpt-4.1, gpt-5-mini, gpt-5-nano etc. must be filtered out.
  #   2. The Codex model allowlist must NOT be hardcoded — it should be derived from a maintainable data source (e.g. a JSON config file or a fetched endpoint) so it can be updated as OpenAI adds/removes models from the Codex catalog without code changes.
  #   3. Filtering must use slug prefix matching (like the Codex CLI does) — e.g. a Codex catalog entry 'gpt-5.2-codex' should match models.dev entries like 'gpt-5.2-codex-2026-03-01'. Exact match alone is insufficient.
  #   4. When no Codex OAuth is active (API key only), the full models.dev catalog must be shown unfiltered — the Codex allowlist only applies when Codex subscription auth is in use.
  #   5. Local model profiles (openai: profilename with 📁 icon) must NOT be filtered — only cloud models from the OpenAI provider on models.dev are subject to Codex allowlist filtering.
  #
  # EXAMPLES:
  #   1. User has Codex OAuth active. models.dev returns 19 OpenAI models including o3-pro, gpt-4.1, gpt-5-mini. Model selector shows only 5 picker-visible Codex models: gpt-5.3-codex, gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.2, gpt-5.1-codex-mini. The 14 unsupported models are not shown.
  #   2. User has NO Codex OAuth but HAS an OpenAI API key. models.dev returns 19 models. All 19 models are shown in the OpenAI section because API key users can access any model from the OpenAI platform — no Codex filtering applied.
  #   3. OpenAI releases a new model 'gpt-6-codex' and adds it to the Codex /models endpoint. Without a code release, the allowlist config is updated to include gpt-6-codex, and the model appears in the selector on next startup.
  #
  # ========================================
  Background: User Story
    As a user with a Codex subscription
    I want to see only models my subscription can actually use in the model selector
    So that I don't select a model that will fail at runtime

  Scenario: Codex OAuth active filters models to only Codex-supported catalog entries
    Given I have authenticated with Codex via OAuth
    And models.dev returns 19 OpenAI models including o3-pro, gpt-4.1, gpt-5-mini, gpt-5.2-codex, gpt-5.2, and gpt-5.1-codex-max
    And the Codex allowlist contains slugs gpt-5.3-codex, gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.1-codex, gpt-5.2, gpt-5.1, gpt-5-codex, gpt-5, gpt-oss-120b, gpt-oss-20b, gpt-5.1-codex-mini, gpt-5-codex-mini
    When models are loaded for the model selector
    Then the Codex (ChatGPT) section should only contain picker-visible models matching the allowlist
    And models with visibility hide in the allowlist should not appear in the selector
    And models like o3-pro, gpt-4.1, gpt-5-mini should not appear
    And models should be sorted by allowlist priority

  Scenario: No Codex OAuth shows full unfiltered models.dev catalog
    Given I have not authenticated with Codex via OAuth
    And I have an OpenAI API key configured
    And models.dev returns 19 OpenAI models including o3-pro, gpt-4.1, gpt-5-mini, and gpt-5.2
    When models are loaded for the model selector
    Then I should see an OpenAI section with all 19 models unfiltered
    And no Codex allowlist filtering should be applied

  Scenario: Allowlist is loaded from external config file not hardcoded in source
    Given I have authenticated with Codex via OAuth
    And a codex-models.json config file exists with the Codex-supported model slugs
    When models are loaded for the model selector
    Then the allowlist should be read from the config file
    And the filtering behavior should match the config file contents

  Scenario: Adding a new model to the allowlist config makes it appear without code changes
    Given I have authenticated with Codex via OAuth
    And the Codex allowlist config does not contain gpt-6-codex
    And models.dev returns a model with slug gpt-6-codex
    When models are loaded for the model selector
    Then gpt-6-codex should not appear in the Codex section
    When the allowlist config is updated to include gpt-6-codex
    And models are reloaded for the model selector
    Then gpt-6-codex should appear in the Codex section

  Scenario: Slug prefix matching filters dated model variants correctly
    Given I have authenticated with Codex via OAuth
    And the Codex allowlist contains slug gpt-5.2-codex
    And models.dev returns a model with slug gpt-5.2-codex-2026-03-01
    When models are loaded for the model selector
    Then gpt-5.2-codex-2026-03-01 should appear in the Codex section because it prefix-matches gpt-5.2-codex

  Scenario: Local model profiles are never filtered by the Codex allowlist
    Given I have authenticated with Codex via OAuth
    And I have a local OpenAI profile named work-vllm with models Qwen3-80B and Llama-4-Scout
    And the Codex allowlist does not contain Qwen3-80B or Llama-4-Scout
    When models are loaded for the model selector
    Then the local profile section openai: work-vllm should still display both models
    And the Codex allowlist filtering should only apply to cloud models from models.dev
