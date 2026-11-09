@done
@tui
@model-selector
@critical
@BUG-576
Feature: The model selector keeps its place across loads, refreshes and edits
  """
  Implementation:
  - the App numbers every list_providers request for the selector (model_list_requests counter); ListProvidersLoaded and a new ListProvidersFailed carry the number and the view applies only the newest requested (stale answers dropped; is_refreshing cleared only by the newest). OpenModelSelectorView resets the view (reset_for_open) and marks it as opening, so the first list of a visit applies the collapse default; later lists keep the expanded set (pruned) and the cursor's (provider_key, model_id) row. Custom-model writes pass the target section/model so it is expanded and focused. The current model is (provider, model) from SessionInfo, stored in the agent view store; matching uses both. ModelSelectionLoaded re-expands and seeds the cursor while has_selection is false. MissingLimitRetry::ModelSelected carries the ModelSync revision; the retry is skipped when the session's revision moved on. No-session selections queue behind the one in flight and are applied to the session it created. Tests: App + MockBackend with call gates (tests/common/call_gates.rs) on list_providers, paused clock.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Product owner decision (2026-11-07): sections collapse only when /model opens, with the current model's section expanded; a refresh, a late list and adding, editing or deleting a custom model keep the expanded sections and the cursor, and the added or edited model's section stays open. This replaces RPC-342's rule that every reload collapses again.
  #   2. Opening /model starts from a clean list: no rows, filter, custom model form or refresh mark from an earlier visit; it says 'Loading models…' until this visit's list answers.
  #   3. Every provider list is numbered when it is requested, and only the newest one requested is shown: an older list that answers later is dropped, and the refresh mark stays until the newest list answers.
  #   4. A list that fails ends 'Loading models…' and '(refreshing...)', and the selector says the models could not be loaded and why (the RPC-451 dialog still shows).
  #   5. The cursor is always on a row of the list; while nothing is selected no row is drawn selected, and e and d do nothing.
  #   6. The current model is the session's provider and model together: only that section opens and only that row is marked (current); when the session's model becomes known after the list loaded and the user has not moved the cursor, its section opens and the cursor goes to it.
  #   7. A missing-limit prompt's answer saves the limit, but selects its model again only when no other model was selected for that session since.
  #   8. With no session open, a second model selection made while the first is still creating its session waits for it and is then applied to the session the first one created, so only one session is created.
  #   9. While a filter is being typed the footer shows the filter keys (Type to filter | Enter Apply | Esc Clear); otherwise it lists every browse key, including Tab Providers and a/e/d Custom model (AK-9 of the 2026-11-07 TUI dead-actions audit).
  #
  # EXAMPLES:
  #   1. Expand Anthropic and press r: after the refresh Anthropic is still expanded and the cursor is on the same row
  #   2. Add custom model m2 under the collapsed profile local: after the reload local is expanded and the cursor is on m2
  #   3. Open /model, type the filter gpt, press Enter and then Esc, and open /model again: it says 'Loading models…' with no filter until the new list answers
  #   4. Press r and add a custom model at once; the add's list answers before the refresh's list: the new model is still listed
  #   5. Press r twice; the first refresh answers: '(refreshing...)' stays until the second answers
  #   6. The provider list fails: the dialog shows, and the selector says the models could not be loaded and why, not 'Loading models…'
  #   7. The cursor was on row 40 of an expanded list; the list reloads collapsed with no current model: the cursor is inside the list, no row is drawn selected and d does nothing
  #   8. codex and openai:local both list m1 and the session uses openai:local m1: only openai:local opens and only its m1 says (current)
  #   9. Open /model before the new session's model is known; the model arrives after the list: its section opens and the cursor is on it
  #   10. Select X, which is missing a limit, then select Y; answer X's prompt: the limit is saved and the session stays on Y
  #   11. With no session, select a model and select another before the session is created: one session is created and it uses the second model
  #   12. Press / in /model: the footer says 'Type to filter | Enter Apply | Esc Clear' until Enter or Esc leaves the filter
  #   13. The browse footer of /model lists Tab Providers and a/e/d Custom model
  #
  # ========================================
  Background: User Story
    As a user choosing a model in /model
    I want to keep my place in the list while it loads, refreshes and changes
    So that the sections I opened, my cursor and the current model are never lost or shown wrongly

  Scenario: A refresh keeps the sections I opened and my cursor
    Given the model selector is open with Anthropic expanded and the cursor on one of its models
    When I press r and the refreshed list answers
    Then Anthropic is still expanded and the cursor is on the same model

  Scenario: An added custom model's section opens on it
    Given the profile local is collapsed in the model selector
    When I add the custom model m2 under local and the reloaded list answers
    Then local is expanded and the cursor is on m2

  Scenario: Reopening the selector starts from a clean list
    Given I opened /model, typed the filter gpt, pressed Enter and then Esc
    When I open /model again before its list answers
    Then it says "Loading models…" and shows no filter

  Scenario: An older list that answers last is dropped
    Given I pressed r and added a custom model before the refresh's list answered
    When the add's list answers before the refresh's list
    Then the new custom model is still listed

  Scenario: The refresh mark stays until the newest refresh answers
    Given I pressed r twice
    When the first refresh's list answers
    Then the title still says "(refreshing...)"
    And once the second refresh's list answers it does not

  Scenario: A failed list says the models could not be loaded
    Given the provider list cannot be read
    When I open /model
    Then the provider list error dialog shows
    And the selector says the models could not be loaded and why, not "Loading models…"

  Scenario: With nothing selected the cursor stays inside the list and no row is drawn selected
    Given I left /model with the cursor on row 40 of an expanded list
    When I open /model again with no current model
    Then the cursor is on a row of the list, no row is drawn selected and d does nothing

  Scenario: The current model is matched by provider and model
    Given codex and openai:local both list m1 and the session uses openai:local m1
    When I open /model
    Then only openai:local is expanded and only its m1 is marked (current)

  Scenario: A current model that becomes known after the list opens its section
    Given I opened /model before the session's model was known and the list has answered
    When the session's model arrives
    Then its section is expanded and the cursor is on it

  Scenario: An answer to an older missing-limit prompt does not select its model again
    Given I selected model X, which is missing a limit, and then selected model Y
    When I answer X's missing-limit prompt
    Then the limit is saved and the session stays on Y

  Scenario: Two selections with no session create one session
    Given no session is open
    When I select a model and select another before the session is created
    Then one session is created and it uses the second model

  Scenario: The footer shows the filter keys while I type a filter
    Given the model selector is open with its list loaded
    When I press /
    Then the footer says "Type to filter | Enter Apply | Esc Clear"
    And after Enter the footer lists the browse keys again

  Scenario: The browse footer lists every browse key
    Given the model selector is open with its list loaded
    When the selector is drawn
    Then the footer lists "Tab Providers" and "a/e/d Custom model" next to the other browse keys
