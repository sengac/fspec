@model-selection
@wip
@ts-parity
@model-selector
@tui
@PROV-127
Feature: Drop empty/uncredentialed cloud sections from model selector (TS parity)
  """
  Fix location 1: rust/sessions/src/handle_impl.rs list_providers() (~lines 958-991). After the map that fills each canonical provider's models via cloud_model_entries, drop any cloud section (is_custom == false, profile_name == None) whose models Vec is empty — matching TS cloudSectionBuilder.ts filter(s => s.hasCredentials) + modelInitializationService.ts filter(s => s.models.length > 0). Local profile sections (appended later) are NOT affected by this filter.
  Fix location 2: pluralization. rust/fspec-tui/src/views/model_selector/rows.rs (~76-80) header label uses hardcoded "({} models)". Add a pluralization helper so count==1 renders "(1 model)" and count!=1 renders "(N models)". state.rs title_text (~177) uses "Select Model ({} models)" for the total — the title total is typically plural but should also use the helper for count==1 correctness.
  Scope guard: this card drops on models.len()==0 (credential-agnostic, safe). Out of scope: Codex re-parenting (PROV-129), unifying split credential gate (PROV-128), section ordering/default (PROV-130). Regression net: e2e/prov-126-cloud-sections.test.ts asserts no dead (0 models) headers; Rust unit tests in rust/sessions can assert the drop-empty filter directly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A cloud provider section with zero models is not rendered in the selector
  #   2. A cloud provider section with one or more models is rendered unchanged
  #   3. Section header model counts are pluralized correctly: (1 model) singular, (2 models) plural
  #
  # EXAMPLES:
  #   1. Fixture with creds for openai/anthropic/together/moonshot renders exactly those four cloud sections; cohere and mistral (catalogued but uncredentialed) are dropped
  #   2. No cloud provider header shows a zero-model count at all in the rendered selector buffer
  #   3. A provider with exactly one model shows (1 model) not (1 models)
  #   4. A provider with two models shows (2 models)
  #
  # ========================================
  Background: User Story
    As a developer using the /model selector
    I want to see only cloud provider sections that actually have models
    So that the picker is not cluttered with dead "(0 models)" headers and matches the TypeScript reference

  @e2e
  Scenario: Uncredentialed cloud sections are dropped from the selector
    Given the fspec binary is launched with a temp HOME whose models.dev cache lists openai, anthropic, togetherai, moonshotai, cohere and mistral
    And credentials are configured only for openai, anthropic, together and moonshot
    When I open the /model view
    Then the credentialed providers render populated sections
    And no uncredentialed cloud provider section appears with zero models
    And no cloud provider header shows a zero-model count at all
