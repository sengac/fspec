@done
@model-selection
@rust
@providers
@codex
@high
@PROV-176
Feature: The Codex section lists only the models Codex /models returns
  """
  Today: sessions cloud_models.rs codex_reparented_models (:179) lists the whole models.dev OpenAI catalog under Codex, called from handle_impl_5.rs:118 (list_providers, synchronous). MODEL-014 added codelet_providers::codex::models_catalog (fetch_for_selection, CodexCatalog by slug, test-support redirect) that this card can reuse; CodexCatalog keeps only the reasoning fields today and would need slug, display_name, visibility and (depending on the limits answer) context_window. Facts: spec/attachments/MODEL-014/codex-models-endpoint-research.md and codex-models-response-capture.json.
  Implementation (2026-11-06): codelet_providers::codex::models_catalog keeps each /models entry's slug, display_name and visibility in response order (the capture's order is its priority order) besides the reasoning fields. sessions list_providers fetches /models (fetch_for_selection, through block_in_place like set_model) every time it builds the list, when Codex has credentials, and builds the Codex section from the listed entries; PROV-129's re-parenting of the models.dev OpenAI catalog goes, and the openai provider keeps PROV-146 (no cloud section). A failed or impossible fetch keeps the Codex section with no models and the reason in a new ProviderInfo.unavailable_reason field, which the PROV-127 drop-empty filter keeps and the model selector shows in the section header. A listed model's context window, reasoning and vision flags come from the same models.dev OpenAI entry the limits come from when it exists (none otherwise). Selecting a Codex model that models.dev does not know is refused naming the missing limits (codex_catalog_limit_resolution's uncatalogued case moves to the new rule).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Codex (ChatGPT) section lists the models the Codex /models response returns (MODEL-014's fetch with ChatGPT sign-in and the pinned client version), not the models.dev OpenAI catalog re-parented by PROV-129; nothing from models.dev is substituted (no silent fallbacks)
  #   2. A /models entry with visibility hide is not listed, as Codex itself does and as PROV-034's rule (models with visibility hide do not appear) already says
  #   3. Codex API-key sign-in (no ChatGPT tokens) cannot call /models, so it lists no Codex models from /models (MODEL-014, product owner 2026-11-06 confirmed API-key mode has no Codex /models)
  #   4. Confirmed by the product owner (2026-11-06): PROV-146 is kept and only the Codex section changes; it lists what Codex /models returns. There is no OpenAI cloud section, and the models.dev OpenAI catalog leaves the model selector.
  #   5. Codex /models is fetched each time the provider list is built, i.e. every open of the model selector (product owner, 2026-11-06, option a)
  #   6. When /models cannot be reached (network down, 401, API-key-only sign-in) the Codex section is shown with no models and the reason; models.dev is never used (product owner, 2026-11-06, option b)
  #   7. A /models model that models.dev does not know is listed, but refused at selection with an error naming the missing limits; no limits are invented or derived from /models fields (product owner, 2026-11-06)
  #
  # EXAMPLES:
  #   1. Signed in with ChatGPT, /models returns gpt-6.1-sol, gpt-6-luna (visibility list) and gpt-reserve (visibility hide): the Codex section lists gpt-6.1-sol and gpt-6-luna; gpt-reserve, gpt-5.5 and gpt-4o are not listed
  #   2. Codex signed in with ChatGPT and an OpenAI API key set: no OpenAI cloud section is shown, and gpt-5.5 and gpt-4o appear in no section
  #   3. The model selector is opened twice and /models returns gpt-6-luna added the second time: the provider list is fetched each time, and the second list includes gpt-6-luna
  #   4. Signed in with ChatGPT, /models answers 401: the Codex section is shown with no models and the reason (the Codex /models request failed: status 401); no models.dev OpenAI model appears under Codex
  #   5. Codex signed in with an API key only: the Codex section is shown with no models and the reason that Codex models come from /models, which needs ChatGPT sign-in
  #   6. /models lists gpt-6.1-sol, which models.dev does not know: it is listed under Codex, and selecting it is refused with an error naming its missing context window and output limit
  #
  # QUESTIONS (ANSWERED):
  #   Q: When is Codex /models fetched for the model selector? No session exists while the selector lists providers, and MODEL-014 fetches once per model selection. Options: (a) each time the provider list is built (every open of the model selector); (b) once per daemon start, kept in memory until the next start; (c) reuse the response the latest Codex session fetched, else fetch; (d) another policy.
  #   A: Codex /models is fetched each time the provider list is built, i.e. every open of the model selector (product owner, 2026-11-06, option a)
  #
  #   Q: What does the Codex section show when /models cannot be reached (network down, 401, API-key-only sign-in)? Options: (a) no Codex section at all; (b) a Codex section with no models and the reason; (c) something else. models.dev is not used either way (no silent fallback).
  #   A: When /models cannot be reached (network down, 401, API-key-only sign-in) the Codex section is shown with no models and the reason; models.dev is never used (product owner, 2026-11-06, option b)
  #
  #   Q: Many /models models are not in models.dev (gpt-6-luna, gpt-6.1-sol, gpt-5.6-* in the 2026-11-05 capture), and fspec takes a Codex model's context window and max output from models.dev today; a model with no known limits cannot start a session (PROV-165, codex-declares-no-static-model-limits). /models gives context_window and max_context_window but no max output. Should Codex limits come from /models (which field: context_window or max_context_window, and what for max output), or should a /models model that models.dev does not know be listed but refused at selection with the reason?
  #   A: A /models model that models.dev does not know is listed, but refused at selection with an error naming the missing limits; no limits are invented or derived from /models fields (product owner, 2026-11-06)
  #
  #
  #   Q: Rule 3 (an OpenAI section keeping the models.dev catalog) contradicted PROV-146 (no standalone OpenAI cloud section). Keep PROV-146 (a) or reverse it (b)?
  #   A: Confirmed by the product owner (2026-11-06): (a). PROV-146 is kept and only the Codex section changes; it lists what Codex /models returns. There is no OpenAI cloud section, and the models.dev OpenAI catalog leaves the model selector.
  # ========================================
  Background: User Story
    As a user signed in to Codex with ChatGPT
    I want to see in the Codex section only the models Codex /models returns for my account
    So that I never pick a model Codex does not serve, and every listed model has its Codex thinking levels

  Scenario: The Codex section lists the models Codex /models returns
    Given Codex is signed in with ChatGPT and /models returns gpt-6.1-sol and gpt-6-luna with visibility list and gpt-reserve with visibility hide
    When the provider list is built
    Then the Codex section lists gpt-6.1-sol and gpt-6-luna
    And gpt-reserve, gpt-5.5 and gpt-4o are not listed under Codex

  Scenario: The OpenAI provider keeps its sections
    Given Codex is signed in with ChatGPT and an OpenAI API key is set
    When the provider list is built
    Then no standalone OpenAI cloud section is shown, as PROV-146 decided
    And no section lists the models.dev OpenAI catalog's gpt-5.5 or gpt-4o

  Scenario: Codex /models is fetched every time the provider list is built
    Given Codex is signed in with ChatGPT and /models returns gpt-6.1-sol
    When the provider list is built, /models then also returns gpt-6-luna, and the provider list is built again
    Then /models was requested twice
    And the second Codex section lists gpt-6.1-sol and gpt-6-luna

  Scenario: An unreachable /models leaves the Codex section empty with the reason
    Given Codex is signed in with ChatGPT and /models answers 401
    When the provider list is built
    Then the Codex section is shown with no models and the reason the Codex /models request failed with status 401
    And no models.dev OpenAI model is listed under Codex

  Scenario: Codex signed in with an API key only lists no Codex models
    Given Codex is signed in with an API key and not with ChatGPT
    When the provider list is built
    Then the Codex section is shown with no models and the reason that Codex models need ChatGPT sign-in

  Scenario: A listed model models.dev does not know is refused at selection
    Given Codex is signed in with ChatGPT and /models returns gpt-6.1-sol, which models.dev does not know
    When the provider list is built and the user selects codex/gpt-6.1-sol
    Then gpt-6.1-sol is listed under Codex
    And the selection is refused with an error naming its missing context window and output limit
