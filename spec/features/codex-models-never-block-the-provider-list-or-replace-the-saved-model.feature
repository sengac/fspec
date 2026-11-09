@done
@tui
@providers
@codex
@model-selection
@critical
@BUG-577
Feature: A slow, failing or empty Codex /models never blocks the provider list or replaces the saved model
  """
  Implementation:
  - codelet_providers models_catalog gets fetch_for_listing(): one shared in-flight /models fetch on its own thread (as the list path's fetch does today), joined by concurrent lists, each waiting on a condvar for at most LIST_BUDGET (3 s; test-support can change it). Failures are typed (not signed in with ChatGPT / request failed / no answer in time / empty) so the list reason differs from the selection reason MODEL-014 keeps. sessions codex_section builds the section from it. startup_model_resolution keeps a persisted model whose cloud section carries unavailable_reason. rpc list_providers runs the handle call in spawn_blocking. fspec-tui header truncates the reason with an ellipsis; toggle_expansion ignores a reason-bearing section with no models.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Product owner decision (2026-11-07): a failed or empty Codex /models fetch never changes tui.lastUsedModel or any saved model choice. At startup a saved model whose Codex section lists nothing because /models failed or was empty is kept, never replaced by the first available model.
  #   2. Building the provider list waits at most 3 s for Codex /models, well under the 10 s RPC deadline. When the answer has not arrived by then, the Codex section is shown with no models and the reason that the model list did not arrive in time, and the rest of the list is returned.
  #   3. Codex /models is still fetched for every provider list (PROV-176 rule 5, product owner 2026-11-06). A list built while a fetch is in flight waits for that fetch instead of starting a second one.
  #   4. A Codex /models answer that lists no models shows the Codex section with no models and a reason saying so, as a failed fetch does (PROV-176 rule 6).
  #   5. The daemon builds the provider list on its blocking pool, so a slow Codex fetch or profile probe never holds an async server worker.
  #   6. The Codex section's reason is about the model list and short: without ChatGPT sign-in it says to sign in with ChatGPT under /provider; a failed request says the model list is unavailable and why. The header shortens a reason that does not fit with an ellipsis.
  #   7. Enter or Right on a section that lists no models because of its reason does not expand it.
  #
  # EXAMPLES:
  #   1. Saved model codex/gpt-6.1-terra; fspec starts while /models answers 503: the startup model stays codex/gpt-6.1-terra and tui.lastUsedModel is unchanged
  #   2. Saved model codex/gpt-6.1-terra; /models answers with an empty list at startup: the saved model is kept
  #   3. Saved model codex/gpt-retired; /models lists only gpt-6.1-sol: the first available model is chosen, as before (the model is really gone)
  #   4. /models does not answer: the provider list returns after the 3 s budget with the profile and cloud sections, and the Codex section says its model list did not arrive in time
  #   5. Two provider lists built while one /models request is in flight: /models is requested once and both lists show its models
  #   6. Codex signed in with an API key only: the Codex header says to sign in with ChatGPT under /provider
  #   7. A long failure reason in an 80-column model selector ends with an ellipsis and starts with 'model list unavailable'
  #   8. Enter on the empty Codex header with a reason leaves it collapsed
  #
  # QUESTIONS (ANSWERED):
  #   Q: the card asks for the Codex catalog to be cached with a refresh, but PROV-176 rule 5 (product owner, 2026-11-06) says /models is fetched every time the provider list is built. Keep fetching per list (bounded by the 3 s budget, joining an in-flight fetch), or cache the last catalog?
  #   A: Conservative choice taken 2026-11-07, pending product owner confirmation: keep PROV-176 rule 5. /models is fetched for every provider list, each list waits at most 3 s for it and joins a fetch already in flight; no catalog is cached, so the list never shows models Codex did not just return.
  #
  # ASSUMPTIONS:
  #   1. Conservative choice taken 2026-11-07, pending product owner confirmation: keep PROV-176 rule 5. /models is fetched for every provider list, each list waits at most 3 s for it and joins a fetch already in flight; no catalog is cached, so the list never shows models Codex did not just return.
  #
  # ========================================
  Background: User Story
    As a user whose saved model is a Codex model, or who opens the model selector on a slow network
    I want to start fspec and list providers while Codex /models is slow, failing or empty
    So that my saved model is never replaced, the provider list always arrives, and the Codex section says plainly why it lists nothing

  Scenario: A saved Codex model is kept when Codex /models fails at startup
    Given my saved model is codex/gpt-6.1-terra
    And the provider list's Codex section lists no models because the Codex /models request failed
    When fspec starts
    Then the startup model is codex/gpt-6.1-terra
    And no other model is saved

  Scenario: A saved Codex model is kept when Codex /models lists no models
    Given my saved model is codex/gpt-6.1-terra and Codex /models answers with no models
    When the provider list is built and the startup model is resolved
    Then the Codex section is shown with no models and the reason that Codex /models listed none
    And the startup model is codex/gpt-6.1-terra

  Scenario: A saved Codex model that Codex /models no longer lists gives way to the first available model
    Given my saved model is codex/gpt-retired and the Codex section lists only gpt-6.1-sol
    When the startup model is resolved
    Then the first available model is chosen

  Scenario: The provider list does not wait past the list budget for Codex /models
    Given Codex is signed in with ChatGPT and Codex /models does not answer
    When the provider list is built
    Then the provider list is returned with the Codex section showing no models
    And the reason says the Codex model list did not arrive in time

  Scenario: Provider lists built during one Codex /models request share it
    Given Codex is signed in with ChatGPT and Codex /models answers only when released
    When two provider lists are built while the request is in flight and the answer is then released
    Then Codex /models was requested once
    And both Codex sections list the models it returned

  Scenario: A slow provider list does not hold up other daemon requests
    Given the daemon has one worker thread and building the provider list is blocked
    When another request is sent while the provider list is being built
    Then the other request is answered before the provider list
    And the provider list is returned once it is unblocked

  Scenario: Without ChatGPT sign-in the Codex section says where to sign in
    Given Codex is signed in with an API key and not with ChatGPT
    When the provider list is built
    Then the Codex section's reason says Codex models need ChatGPT sign-in under /provider

  Scenario: A long reason is shortened with an ellipsis in the model selector
    Given the Codex section lists no models with a reason longer than the row
    When the model selector is drawn 80 columns wide
    Then the Codex header shows the start of the reason and ends with an ellipsis

  Scenario: An empty Codex section with a reason does not expand
    Given the Codex section lists no models with a reason
    When the user presses Enter and then Right on the Codex header
    Then the Codex section stays collapsed
