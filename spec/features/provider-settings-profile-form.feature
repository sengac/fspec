@done
@PROV-110
Feature: Profile create/edit form UI in Provider Settings

  """
  Rust ratatui port of the TS profile form (src/tui/inputHandlers/profileFormModeHandler.ts, providerSettingsHelpers.ts, constants/providerSettings.ts). Adds ProviderSettingsMode::CreateProfile{provider_id} and EditProfile{provider_id,profile_name} carrying a ProfileForm state struct (name + five raw-string fields baseUrl/apiKey/contextWindow/maxOutputTokens/compactionThreshold, a field_index into PROFILE_FORM_FIELDS, and is_editing_name). Mirrors the model_selector CustomModelForm pattern for key routing and parse-on-build. Key handling parity: Esc->List; Down/Up move field_index (Down from name editing exits to field 0; Up from field 0 in create re-enters name editing; Up while editing name is a no-op); Tab ignored (TUI-084); printable ASCII 32..=126 appended; Backspace/Delete pops last char; Enter saves only when baseUrl && apiKey && trimmed name are non-empty, building ProfileDefinition (contextWindow/maxOutputTokens parsed via integer parse with non-numeric omitted, compactionThreshold parsed to type+value) and emitting Action::SaveProfile then returning to List. Form UI + state only; nav routing into the form and the backend write live in PROV-111/PROV-108/PROV-109.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Create mode starts with the name field being edited (isEditingName=true), an empty name, baseUrl prefilled to http://localhost:8888, empty apiKey, and field index 0
  #   2. Edit mode prefills all five connection fields from the existing profile definition, shows the profile name (not editable, isEditingName=false), and starts at field index 0
  #   3. The form has five connection fields in order: baseUrl, apiKey, contextWindow, maxOutputTokens, compactionThreshold (customModels is NOT a form field)
  #   4. Down arrow moves to the next field (index+1, clamped); when editing the name it exits name editing and moves to field 0. Up arrow moves to the previous field (index-1); at field 0 in create mode it re-enters name editing; when editing the name Up does nothing
  #   5. Tab is intentionally ignored in form mode (TUI-084) and does not change the field index or name editing
  #   6. Esc returns the form to List mode without saving
  #   7. Printable ASCII characters (32..=126) are appended to the name when editing the name, otherwise to the focused field's raw text; Backspace/Delete removes the last character from the name or focused field
  #   8. Enter saves only when baseUrl, apiKey, and the trimmed name are all non-empty; it builds a ProfileDefinition (contextWindow/maxOutputTokens parsed as integers, NaN omitted; compactionThreshold parsed to type+value or omitted) and emits Action::SaveProfile{provider_id, profile_name, definition} then returns to List. If invalid, Enter does nothing
  #
  # EXAMPLES:
  #   1. User opens create form: name field is active, baseUrl shows http://localhost:8888, apiKey empty, field index 0
  #   2. User opens edit form for 'fireworks': name shows 'fireworks' (not editable), baseUrl/apiKey/contextWindow prefilled from the stored definition
  #   3. In create mode editing name, pressing Down exits name editing and focuses baseUrl (field 0)
  #   4. On baseUrl (field 0) in create mode, pressing Up re-enters name editing
  #   5. Pressing Tab while on baseUrl leaves the field index unchanged
  #   6. User types into apiKey then presses Enter with baseUrl and name set: a SaveProfile action is emitted and the form returns to List
  #   7. User presses Enter while apiKey is empty: nothing happens, form stays open, no action emitted
  #   8. User types '128000' into contextWindow and saves: the definition carries context_window=128000; typing 'abc' leaves it omitted
  #   9. User presses Esc in the form: it returns to List mode without emitting a save action
  #   10. User types '80%' into compactionThreshold and saves: the definition carries compaction_threshold_type=percentage and value=80
  #
  # ========================================

  Background: User Story
    As a developer configuring provider settings
    I want to open a real create/edit profile form with name and connection fields and type into them
    So that I can author a local-server profile in the TUI instead of a blank read-only placeholder

  @rust @tui @provider-settings @crud
  Scenario: Create form starts editing the name with prefilled base URL
    Given a new create profile form for provider "openai"
    Then the name editing flag is true
    And the name is empty
    And the base URL field shows "http://localhost:8888"
    And the api key field is empty
    And the focused field index is 0

  @rust @tui @provider-settings @crud
  Scenario: Edit form prefills connection fields from the stored profile
    Given an edit profile form for provider "openai" profile "fireworks" with a stored definition
    Then the name editing flag is false
    And the name is "fireworks"
    And the base URL field shows the stored base URL
    And the api key field shows the stored api key
    And the focused field index is 0

  @rust @tui @provider-settings @crud
  Scenario: Down arrow from name editing focuses the first connection field
    Given a new create profile form for provider "openai"
    When the user presses the Down arrow key
    Then the name editing flag is false
    And the focused field index is 0

  @rust @tui @provider-settings @crud
  Scenario: Up arrow on the first field re-enters name editing in create mode
    Given a create profile form for provider "openai" focused on the base URL field
    When the user presses the Up arrow key
    Then the name editing flag is true

  @rust @tui @provider-settings @crud
  Scenario: Tab is ignored and leaves the focused field unchanged
    Given a create profile form for provider "openai" focused on the base URL field
    When the user presses the Tab key
    Then the focused field index is 0
    And the name editing flag is false

  @rust @tui @provider-settings @crud
  Scenario: Saving a valid profile emits a SaveProfile action and returns to list
    Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888"
    When the user types "sk-test" into the api key field
    And the user presses the Enter key
    Then a SaveProfile action is emitted for provider "openai" profile "local"
    And the provider settings mode returns to list

  @rust @tui @provider-settings @crud
  Scenario: Saving with an empty api key does nothing
    Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888"
    When the user presses the Enter key
    Then no SaveProfile action is emitted
    And the provider settings mode stays on the form

  @rust @tui @provider-settings @crud
  Scenario: Numeric fields are parsed on save and non-numeric input is omitted
    Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888" and api key "sk-test"
    When the user types "128000" into the context window field
    And the user presses the Enter key
    Then the emitted definition context window is 128000
    And the emitted definition max output tokens is omitted

  @rust @tui @provider-settings @crud
  Scenario: Escape returns to list without saving
    Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888"
    When the user presses the Escape key
    Then no SaveProfile action is emitted
    And the provider settings mode returns to list

  @rust @tui @provider-settings @crud
  Scenario: Compaction threshold percentage is parsed on save
    Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888" and api key "sk-test"
    When the user types "80%" into the compaction threshold field
    And the user presses the Enter key
    Then the emitted definition compaction threshold type is "percentage"
    And the emitted definition compaction threshold value is 80
