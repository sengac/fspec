@done
@crud
@tui
@model-selector
@rust
@RPC-344
Feature: Model selector missing custom-model CRUD (a/e/d keybinds)

  """
  Backend write surface already exists from RPC-347 (FspecBackend::add/update/delete_custom_model + Action::AddCustomModel/EditCustomModel/DeleteCustomModel + CustomModelDefinition wire type). RPC-344 adds only the UI: a custom-model mode enum + form-state struct on ModelSelectorView, a/e/d keybind guards in handle_key, form/confirm input sub-handlers, two overlay renderers under views/model_selector/, and App::dispatch arms that spawn the three backend calls followed by a list_providers refresh.
  The a/e/d guards reuse the existing row projection: focused_provider_key + the focused row's selectable/is_custom flags + provider profile_name. ModelSelectorRow currently lacks an is_custom flag (the [C] badge is derived in build_badges from ModelEntry.is_custom); the projection must carry is_custom (and the focused profile_name/provider_id) so the e/d guard and edit-prefill can read it without re-walking providers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The form/confirm overlays intercept key input BEFORE the browse/filter handlers: when a custom-model mode is active, a/e/d/typing route to the form, not the model-list keybinds (TS handleDeleteConfirmInput + handleCustomModelFormInput run first, ModelSelectorScreen.tsx:124-131).
  #   2. 'a' (add) is honoured only when the focused row is a profile-section header (provider with profile_name set); it switches to add-custom-model mode with empty form values and field index 0. On any non-profile row it is a consumed no-op.
  #   3. 'e' (edit) is honoured only when the focused row is a SELECTABLE custom model (is_custom) inside a profile section; it switches to edit-custom-model mode carrying the original model id and prefills the form from the focused ModelEntry. On a non-custom model or a non-profile section it is a consumed no-op.
  #   4. 'd' (delete) is honoured only when the focused row is a SELECTABLE custom model (is_custom) inside a profile section; it switches to delete-custom-model-confirm mode carrying provider id, profile name, model id and display name. On a non-custom model or non-profile section it is a consumed no-op.
  #   5. The form has eight fields in display order: Model ID (text, REQUIRED), Display Name (text), Facade (select: openai/codex/claude/gemini/zai), Context Window (number), Max Output Tokens (number), Compaction Trigger (text), Reasoning (boolean), Vision (boolean) — matching CUSTOM_MODEL_FORM_FIELDS.
  #   6. Form input routing: Up/Down move the focused field index (clamped to 0..=7); Left/Right cycle select options (wrapping) and toggle boolean fields; printable ASCII 32-126 is appended to the focused text/number field; Backspace and Delete remove the last char; number fields parse as integers and drop the value when the result is not a valid number; Esc returns to browse; Enter saves.
  #   7. Saving requires a non-empty trimmed Model ID; an empty id is a no-op that leaves the form open. On a valid save the view builds a CustomModelDefinition from the form values, emits Action::AddCustomModel (add mode) or Action::EditCustomModel with original_model_id (edit mode), resets the form, returns to browse mode, and triggers a provider-list refresh.
  #   8. The delete-confirm overlay accepts y or Enter to confirm (emits Action::DeleteCustomModel then returns to browse and refreshes) and n or Esc to cancel (returns to browse with no action).
  #   9. The Compaction Trigger text is parsed into the wire fields compaction_threshold_type/compaction_threshold_value when building the CustomModelDefinition: a trailing-% string (e.g. "80%") → type "percentage" value 80; a bare integer (e.g. "200000") → type "tokens" value 200000; blank or unparseable → both None (omitted).
  #   10. The Add Custom Model form renders the title, the profile name, all eight fields with the active field highlighted (required Model ID marked with *), placeholders for empty fields, the select/boolean cycle hints on the active field, and a footer hint line; the Edit form is identical but titled "Edit Custom Model" with prefilled values.
  #
  # EXAMPLES:
  #   1. Cursor on a profile-section header; pressing 'a' opens the Add Custom Model form with empty values and the Model ID field focused.
  #   2. Cursor on a cloud provider header (no profile); pressing 'a' does nothing and the model list stays in browse mode.
  #   3. Cursor on a custom model row (marked [C]) inside a profile; pressing 'e' opens the Edit Custom Model form prefilled with that model's id, display name, context window, reasoning and vision.
  #   4. Cursor on a non-custom (built-in) model row; pressing 'e' or 'd' does nothing and the list stays in browse mode.
  #   5. In the Add form I type a Model ID, arrow down to Facade and press Right to cycle openai→codex→claude, arrow down to Reasoning and press Right to toggle it true, then press Enter and the new custom model is saved and appears in the list.
  #   6. In the Add form I leave the Model ID empty and press Enter; nothing is saved and the form stays open with the Model ID field still marked required.
  #   7. In the Add form I type "80%" into Compaction Trigger and save; the saved model carries a percentage compaction threshold of 80. Typing "200000" instead saves a tokens threshold of 200000.
  #   8. In the Add form I press Esc; the form closes and I am back in the browse list with no model saved.
  #   9. Cursor on a custom model; pressing 'd' shows "Delete Custom Model" confirming the model display name and profile; pressing y deletes it and returns to the list; pressing n instead cancels and keeps it.
  #   10. While the Add form is open I press 'r' and '/'; the characters are typed into the focused text field instead of triggering refresh or filter, because the form intercepts input first.
  #   11. In the Edit form I clear the Display Name and press Enter; the model is saved in place under the same id with the updated display name (edit replaces the entry via original_model_id).
  #
  # ASSUMPTIONS:
  #   1. KNOWN DIVERGENCE (user decision): the Edit form prefills only from the wire ModelEntry (id, displayName, contextWindow, reasoning, vision). Facade, Max Output Tokens and Compaction Trigger start blank because ProviderInfo/ModelEntry does not carry them; re-saving without re-entering those fields drops them. Lossless prefill would require extending the wire (deferred).
  #
  # ========================================

  Background: User Story
    As a user managing local-server (openai) profiles in the model selector
    I want to add, edit, and delete custom models via a/e/d keybinds with a form and a delete-confirmation overlay
    So that I can manage my profile's custom models without leaving the TUI

  Scenario: Pressing 'a' on a profile-section header opens the Add Custom Model form
    Given the model selector is showing a local-server profile section
    And the cursor is on that profile-section header
    When I press "a"
    Then the Add Custom Model form opens
    And every field is empty
    And the Model ID field is focused

  Scenario: Pressing 'a' on a cloud provider header does nothing
    Given the model selector is showing a cloud provider header with no profile
    And the cursor is on that cloud provider header
    When I press "a"
    Then no form opens
    And the model selector stays in browse mode

  Scenario: Pressing 'e' on a custom model opens the Edit Custom Model form prefilled
    Given the model selector is showing a profile section with a custom model
    And the cursor is on that custom model row
    When I press "e"
    Then the Edit Custom Model form opens
    And the form is prefilled with the model's id, display name, context window, reasoning and vision

  Scenario: Pressing 'e' or 'd' on a built-in model does nothing
    Given the model selector is showing a profile section with a built-in non-custom model
    And the cursor is on that built-in model row
    When I press "e"
    Then no form opens
    And the model selector stays in browse mode
    When I press "d"
    Then no delete confirmation opens
    And the model selector stays in browse mode

  Scenario: Adding a custom model with a facade and reasoning enabled saves it
    Given the Add Custom Model form is open for a profile section
    When I type a Model ID
    And I move down to the Facade field and press the right arrow twice
    And I move down to the Reasoning field and press the right arrow once
    And I press Enter
    Then a custom model is saved with the typed id, the selected facade and reasoning enabled
    And the form closes and the provider list is refreshed

  Scenario: Saving the Add form with an empty Model ID is rejected
    Given the Add Custom Model form is open for a profile section
    And the Model ID field is empty
    When I press Enter
    Then no custom model is saved
    And the Add Custom Model form stays open

  Scenario: A "80%" Compaction Trigger saves a percentage threshold
    Given the Add Custom Model form is open for a profile section
    And I have typed a Model ID
    When I enter "80%" into the Compaction Trigger field
    And I press Enter
    Then the saved custom model carries a percentage compaction threshold of 80

  Scenario: A bare integer Compaction Trigger saves a tokens threshold
    Given the Add Custom Model form is open for a profile section
    And I have typed a Model ID
    When I enter "200000" into the Compaction Trigger field
    And I press Enter
    Then the saved custom model carries a tokens compaction threshold of 200000

  Scenario: Pressing Esc in the Add form cancels without saving
    Given the Add Custom Model form is open for a profile section
    When I press Esc
    Then the form closes
    And I am back in the browse list
    And no custom model is saved

  Scenario: Deleting a custom model after confirming
    Given the model selector is showing a profile section with a custom model
    And the cursor is on that custom model row
    When I press "d"
    Then a delete confirmation shows the model display name and profile name
    When I press "y"
    Then the custom model is deleted
    And I am returned to the browse list and the provider list is refreshed

  Scenario: Cancelling the delete confirmation keeps the custom model
    Given the model selector is showing a profile section with a custom model
    And the cursor is on that custom model row
    When I press "d"
    Then a delete confirmation shows the model display name and profile name
    When I press "n"
    Then no custom model is deleted
    And I am returned to the browse list

  Scenario: The open form intercepts keys that are browse shortcuts
    Given the Add Custom Model form is open with the Model ID field focused
    When I press "r"
    And I press "/"
    Then "r/" is typed into the Model ID field
    And neither a refresh nor a filter is triggered

  Scenario: Editing a custom model saves it in place under the same id
    Given the Edit Custom Model form is open for a custom model
    When I clear the Display Name field
    And I press Enter
    Then the custom model is saved in place under its original id with the updated display name
    And the form closes and the provider list is refreshed
