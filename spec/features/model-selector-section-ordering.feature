@done
@ts-parity
@model-selection
@providers
@PROV-130
Feature: Provider section ordering + default-model selection parity (profiles/custom before cloud)
  """
  Fix location: rust/sessions/src/handle_impl.rs list_providers() (~L957-1023) — partition mapped ProviderInfo into cloud vs custom by the source p.is_custom flag; apply PROV-129 synthesize_codex_section + PROV-127 retain_populated_cloud_sections to the cloud group; assemble final order as build_local_profile_sections() ++ custom_sections ++ cloud_sections (TS modelInitializationService.ts:196-200).
  Codex-leads-cloud-group: rust/sessions/src/cloud_models.rs synthesize_codex_section() prepends the synthesized Codex section to the FRONT of the cloud sections (TS cloudSectionBuilder.ts:150-155 pushes codex first, then remaining), instead of filling the codex header in place. PROV-129 tests assert presence/contents only (not position), so they remain green.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The list_providers() display order places local-server profile sections first, then custom provider sections, then built-in cloud provider sections (TS parity: [...profileSections, ...customSections, ...cloudSections]).
  #   2. The auto-selected default model is the first model of the first section in the ordered list (first section that has models), matching TS selectDefaultModel.
  #   3. Within the cloud group, the synthesized Codex (ChatGPT) section leads the cloud sections when Codex credentials are present (TS extractCodexSection returns [codexSection, ...remaining]); the drop-empty (PROV-127) and Codex-synthesis (PROV-129) behaviours still hold after reordering.
  #
  # EXAMPLES:
  #   1. With a local-server openai profile (with a custom model) and a credentialed cloud provider both present, list_providers() returns the profile section before any cloud section, and resolve_startup_model picks the profile's first model.
  #   2. With no local-server profiles and no custom providers, only credentialed cloud sections appear (canonical order) and the default resolves to the first populated cloud section's first model.
  #   3. With Codex OAuth credentials and no profiles/customs, the Codex (ChatGPT) section leads the cloud group so it appears before other cloud sections such as Anthropic.
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to have the /model provider sections ordered profiles-first then custom then cloud
    So that the auto-selected default model matches the TypeScript reference

  Scenario: A local-server profile section precedes cloud sections and provides the default model
    Given a local-server openai profile with a custom model is configured
    And a credentialed cloud provider is present in the models.dev catalog
    When list_providers() assembles the provider list
    Then the local-server profile section appears before every cloud section
    And the auto-selected default model resolves to the profile section's first model

  Scenario: With no profiles or custom providers the default comes from the first cloud section
    Given no local-server profiles and no custom providers are configured
    And two credentialed cloud providers are present in canonical order
    When list_providers() assembles the provider list
    Then only credentialed cloud sections appear in canonical order
    And the auto-selected default model resolves to the first populated cloud section's first model

  Scenario: The synthesized Codex section leads the cloud group
    Given Codex OAuth credentials are present and no profiles or custom providers are configured
    And another credentialed cloud provider Anthropic is present
    When list_providers() assembles the provider list
    Then the Codex (ChatGPT) section appears before the Anthropic cloud section
    And the standalone OpenAI section is not shown
