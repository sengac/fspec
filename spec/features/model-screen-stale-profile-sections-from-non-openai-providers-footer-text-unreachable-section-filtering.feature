@PROV-031
@tui
@model-selector
@providers
@bug-fix
Feature: Model Screen — stale profile sections from non-OpenAI providers, footer text, unreachable section filtering
  """
  Fix 1 — loadProfileSections() guard (modelInitializationService.ts ~line 247). Replace the loop over
  SUPPORTED_PROVIDERS with a loop over ['openai'] only. This is the single correct fix — prevent bad data
  at the source rather than filtering in the TUI layer.

  Fix 2 — Unreachable + 0 models filter (modelInitializationService.ts initializeModels()). After combining
  profileSections + cloudSections, filter out sections that are both unreachable AND have 0 models.
  Keep unreachable sections if they have models (partial server failure). Drop them silently if empty.

  Fix 3 — Footer text (ModelSelectorView.tsx). Change 'Tab: settings' to 'Tab: Switch to providers'.
  Symmetric with ProviderSettingsPanel.tsx which already says 'Tab: Switch to models' (PROV-029 Rule [12]).

  Fix 4 — Item count display (ModelSelectorView.tsx). Count only model items, not section headers.
  Change flatItems.length to flatItems.filter(i => i.type === 'model').length and label 'N models'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. loadProfileSections() in modelInitializationService.ts must only load profiles for the 'openai' provider — skip all other providers entirely. The loop over SUPPORTED_PROVIDERS should be replaced with a single check of the 'openai' provider only.
  #   2. Profile sections that are unreachable AND have 0 models must be silently filtered out of the sections array in initializeModels() — they are useless clutter in the model selector. A local server that can't be reached and returns no models provides zero value.
  #   3. ModelSelectorView.tsx footer must say 'Tab: Switch to providers' (not 'Tab: settings'). This mirrors the ProviderSettingsPanel.tsx fix from PROV-029 Rule [12] which already says 'Tab: Switch to models'. Both panels must use the symmetric label.
  #   4. The model screen header count '(N items)' must count only model items, not section headers. Change flatItems.length to flatItems.filter(i => i.type === 'model').length and label it 'N models' to match user mental model. Provider section headers are navigation structure, not selectable items.
  #
  # EXAMPLES:
  #   1. User config has stale anthropic profile (anthropic → localhost:8888) from OAuth dev — model screen opens, ZERO profile sections for anthropic appear. The screen shows only cloud providers and any valid OpenAI API profiles.
  #   2. User config has openai profile 'qwen3-coder-next' pointing to an offline server — model screen opens, that profile section does NOT appear. No '(unreachable) (0 models)' clutter.
  #   3. User has openai profile 'work-vllm' on a live server with 5 models — model screen shows 'openai: work-vllm (5 models)' at top of list. Reachable profiles with models always appear.
  #   4. Model screen footer shows: 'Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | / filter | Esc: close'
  #   5. Model screen header shows 'Select Model (76 models)' — counts only selectable model items, not section header rows. Before this fix it showed '(27 items)' mixing section headers with model rows.
  #
  # ========================================
  Background: User Story
    As a developer
    I want to open the model selector screen
    So that I only see relevant, reachable models without stale or unreachable clutter

  Scenario: Stale non-OpenAI provider profiles are never loaded into the model screen
    Given the user config has a stale anthropic profile pointing to localhost:8888 from OAuth dev work
    And the user config has a stale gemini profile pointing to a local server
    When loadProfileSections() runs in modelInitializationService.ts
    Then zero profile sections are generated for anthropic
    And zero profile sections are generated for gemini
    And only 'openai' provider profiles are iterated during profile loading

  Scenario: Unreachable OpenAI profile with zero models is filtered from the model screen
    Given the user config has an openai profile 'qwen3-coder-next' pointing to an offline server
    When the model screen initializes
    Then the profile section for 'qwen3-coder-next' does not appear in the model list
    And no '(unreachable) (0 models)' entry is shown

  Scenario: Reachable OpenAI profile with models always appears in the model screen
    Given the user config has an openai profile 'work-vllm' pointing to a live server
    And the live server returns 5 models
    When the model screen initializes
    Then the profile section 'openai: work-vllm (5 models)' appears at the top of the model list

  Scenario: Model screen footer shows symmetric Tab hint
    Given the model selector screen is rendered
    When the footer is displayed
    Then the footer text reads 'Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | / filter | Esc: close'

  Scenario: Model screen header counts only selectable model items, not section headers
    Given the model selector screen is rendered with provider sections and model rows
    When the header count is displayed
    Then the count label reads 'N models' where N is the number of model rows only
    And section header rows are not included in the count
