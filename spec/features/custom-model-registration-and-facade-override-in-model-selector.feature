@done
@MODEL-004
Feature: Custom Model Registration and Facade Override in Model Selector
  """
  Config layer: Extend ProfileConfig in src/utils/provider-config.ts with optional customModels: CustomModelDefinition[]. Add CustomModelDefinition interface with fields: id (string, required), displayName (string, optional), facade ('openai'|'codex'|'claude'|'gemini'|'zai', optional), contextWindow (number, optional), maxOutputTokens (number, optional), reasoning (boolean, optional), hasVision (boolean, optional). Add loadCustomModels() and saveCustomModel() functions.
  Model initialization: In loadProfileSections() (src/tui/services/modelInitializationService.ts), after the /v1/models fetch, load custom models from the profile config and merge them into localModels[]. Custom models override auto-discovered ones with matching IDs. If only custom models exist, the section should NOT have isUnreachable: true. Custom models get their NapiModelInfo built from CustomModelDefinition fields.
  NAPI boundary: Extend sessionSetModelProfile NAPI binding to accept optional facadeOverride parameter (string or null). In Rust set_model_direct (codelet/providers/src/manager.rs), add facade_override: Option<String> field on ProviderManager. The agent loop in session_manager.rs checks facade_override before matching on current_provider to select tool facades.
  Facade dispatch: The facade override ONLY affects tool definition selection (which facades are registered for the agent). It does NOT change the HTTP transport — profile models always use the OpenAI-compatible HTTP client. This mirrors Copilot's pattern where behavior facades control tool schemas independently from the HTTP layer. In practice: facade_override='gemini' means the agent gets GeminiReadFileFacade etc. tool schemas, but the actual API call goes through the OpenAI endpoint with OpenAI wire format.
  TUI Model Selector: Add custom model form mode to useModelSelectorState hook. Keybind 'a' on a profile section header → opens add form. Keybind 'e' on a custom model → opens edit form (pre-filled). Keybind 'd' on a custom model → shows delete confirmation. Form fields: Model ID (required), Display Name, Facade (dropdown: default/openai/codex/claude/gemini/zai), Context Window, Max Output, Reasoning toggle, Vision toggle. Reuse ProviderSettings form pattern with arrow key navigation.
  ModelSelection type: Extend ModelSelection interface (src/tui/types/provider.ts) with optional facade?: string field. When selectModel() in modelSelectionService.ts processes a model with facade override, it passes the facade string through to sessionSetModelProfile's new parameter. The facade is NOT persisted in lastUsedModel string — it's looked up from config when the model is loaded.
  View rendering: In ModelSelectorView.tsx, custom models are distinguished by a [C] badge rendered in yellow. The badge appears AFTER the model name and BEFORE the [R]/[V]/[context] badges. Custom models are identified by a new isCustom boolean on NapiModelInfo (or tracked via a separate Set<string> of custom model IDs in the section).
  Dependency on MODEL-005: MODEL-005 adds per-model context window and max output to the NapiModelInfo → ModelSelection flow. MODEL-004 builds on this by also adding facade override. The CustomModelDefinition interface includes contextWindow and maxOutputTokens which are the same fields MODEL-005 makes configurable. Wait for MODEL-005 to land before implementing the config layer.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Custom models are defined per-profile in fspec-config.json under providers.openai.profiles.<name>.customModels as an array of CustomModelDefinition objects
  #   2. Custom models appear alongside auto-discovered /v1/models models in the same profile section in the Model Selector — they are visually distinguished with a [C] badge
  #   3. Each custom model requires an id (string sent to the API) and optionally specifies: displayName, facade override, contextWindow, maxOutputTokens, reasoning flag, and hasVision flag
  #   4. Facade override accepts one of: 'openai', 'codex', 'claude', 'gemini', 'zai' — this changes which tool schemas (names, parameter formats) are sent to the model via the Rust ProviderType dispatch
  #   5. When a custom model has the same id as an auto-discovered model from /v1/models, the custom model's settings take precedence (override behavior) — the model appears only once in the selector with custom metadata
  #   6. Custom models still appear when /v1/models endpoint is unreachable or returns an empty list — the profile section shows custom models even with isUnreachable: true
  #   7. Facade override is propagated through the NAPI boundary from TypeScript to Rust — sessionSetModelProfile must accept an optional facade parameter that overrides the default ProviderType::OpenAI mapping for that session
  #   8. The TUI form for adding/editing custom models is accessed from the Model Selector screen via keybinds: 'a' to add a new custom model when focused on a profile section, 'e' to edit an existing custom model, 'd' to delete a custom model
  #   9. Custom model config persists in fspec-config.json and survives application restarts — existing ProfileConfig entries remain backward-compatible (customModels is optional)
  #   10. If no facade override is specified on a custom model, the default provider-level facade applies (ProviderType::OpenAI for all openai profiles) — facade override is purely opt-in
  #   11. The custom model form should live in the Model Selector screen. Rationale: (1) Users are already browsing models when they realize one is missing — the add action should be in-context. (2) Provider Settings manages profile-level config (baseUrl, apiKey), while custom models are per-model config within a profile. (3) The keybind pattern (a/e/d) mirrors typical list management UX. (4) The edit form can reuse the same profile form pattern from ProviderSettingsScreen (arrow key field navigation).
  #   12. No warning on facade assignment — trust the user. The user explicitly chose a facade for their model, and they know their server's capabilities. Adding validation would require probing the server which may not be running during config time. Instead, document facade effects clearly in the form and in error messages if tool calls fail at runtime.
  #   13. Defer cloud model overrides to a future work unit. Cloud models from models.dev have correct metadata already and facade is determined by their provider (anthropic → Claude, openai → OpenAI, etc.). The override use case is rare for cloud models. Keep MODEL-004 focused on profile/local models where the problem is acute. Cloud model overrides can use a separate modelOverrides section later if needed.
  #   14. When facade: 'claude' is assigned, thinking config should NOT be activated for profile models. The facade override only controls tool schema format — it doesn't enable provider-specific features like Claude thinking or Codex Responses API. The Rust dispatch should use the facade for tool definition selection only, while keeping the underlying HTTP transport as OpenAI-compatible. This matches how Copilot handles model-family facades.
  #   15. Option A: Add optional facade_override param to sessionSetModelProfile. This is the cleanest approach — the facade is an attribute of the model selection, not a separate concern. The NAPI signature becomes sessionSetModelProfile(sessionId, providerId, modelId, facadeOverride?). In Rust, set_model_direct stores the override alongside the selected model, and the agent loop checks it before defaulting to provider-type-based facade dispatch.
  #   16. No server validation when adding a custom model — trust the user entirely. The whole point of custom models is to add models that might not be listed anywhere. The server may not be running during config. Model ID is a free-form string.
  #   17. When custom models exist but /v1/models fails, show the profile section WITHOUT '(unreachable)' in the header. The unreachable status should only prevent showing auto-discovered models, not custom ones. The section is usable because the user has manually configured models. This makes profiles useful even with servers that never implement /v1/models.
  #
  # EXAMPLES:
  #   1. User adds 'my-fine-tuned-gpt' to their 'work-vllm' profile with displayName 'Fine-Tuned GPT' — after save, Model Selector shows the model in the 'openai: work-vllm' section with [C] badge and it can be selected for a session
  #   2. User sets facade to 'codex' for a custom model 'Qwen/Qwen3-80B' — when that model is active, the Rust agent loop dispatches Codex-native tool schemas (exec_command, shell, read_file, grep_files, list_dir) instead of standard OpenAI function calling
  #   3. vLLM server /v1/models returns empty list but user has 3 custom models defined — profile section shows 3 models (all with [C] badge) and the section header shows 'openai: work-vllm' without the '(unreachable)' suffix since custom models exist
  #   4. User overrides contextWindow for auto-discovered model 'meta-llama/Meta-Llama-3.1-405B' to 32768 by adding it as a custom model with the same id — in the selector, the model shows [32k] instead of the default [128k] and the [C] badge
  #   5. User edits an existing custom model — presses 'e' on 'my-fine-tuned-gpt' in Model Selector, changes facade from default (openai) to 'claude', saves — the config is updated and the next session with this model uses Claude-native tool schemas
  #   6. User presses 'd' on a custom model, confirms deletion — the model is removed from customModels array in fspec-config.json, and if it also exists in /v1/models, it reverts to showing with default auto-discovered metadata (no [C] badge)
  #   7. User adds a custom model with reasoning: true and hasVision: true — in the Model Selector it displays both [R] and [V] badges alongside the [C] badge, and the ModelSelection passed to session includes these capability flags
  #   8. User presses 'a' while focused on a cloud provider section (e.g., 'anthropic') — nothing happens because custom model addition is only available for profile sections (openai profiles), not cloud providers
  #   9. Existing fspec-config.json without customModels field loads normally — backward-compatible, no migration needed. When user adds first custom model, the customModels array is created automatically
  #   10. User selects a custom model with facade: 'gemini', the session starts — Rust agent loop creates GeminiReadFileFacade, GeminiWriteFileFacade, GeminiRunShellCommandFacade etc. instead of OpenAI facades, and tool names/schemas match Gemini native format (read_file, write_file, run_shell_command, search_file_content, list_directory)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the custom model add/edit form live in the Model Selector screen (keybinds a/e/d on model items) or in the Provider Settings screen (alongside profile CRUD)? The Model Selector is where users browse models so it's the natural place for per-model config, but Provider Settings is where profile-level config lives.
  #   A: The custom model form should live in the Model Selector screen. Rationale: (1) Users are already browsing models when they realize one is missing — the add action should be in-context. (2) Provider Settings manages profile-level config (baseUrl, apiKey), while custom models are per-model config within a profile. (3) The keybind pattern (a/e/d) mirrors typical list management UX. (4) The edit form can reuse the same profile form pattern from ProviderSettingsScreen (arrow key field navigation).
  #
  #   Q: Should we show a warning when a user assigns a facade that may be incompatible with their server (e.g., setting facade: 'codex' for a vLLM server that doesn't understand Codex-specific parameters)?
  #   A: No warning on facade assignment — trust the user. The user explicitly chose a facade for their model, and they know their server's capabilities. Adding validation would require probing the server which may not be running during config time. Instead, document facade effects clearly in the form and in error messages if tool calls fail at runtime.
  #
  #   Q: Should cloud model overrides (changing facade for a models.dev model like openai/o3-pro) use the same CustomModelDefinition mechanism or a separate modelOverrides config section? Same mechanism is simpler but cloud models aren't tied to a profile.
  #   A: Defer cloud model overrides to a future work unit. Cloud models from models.dev have correct metadata already and facade is determined by their provider (anthropic → Claude, openai → OpenAI, etc.). The override use case is rare for cloud models. Keep MODEL-004 focused on profile/local models where the problem is acute. Cloud model overrides can use a separate modelOverrides section later if needed.
  #
  #   Q: When facade: 'claude' is assigned to a custom model, should thinking config (ClaudeThinkingFacade) be activated? The model behind vLLM likely doesn't support Claude's thinking protocol. Should we skip thinking config for non-native providers?
  #   A: When facade: 'claude' is assigned, thinking config should NOT be activated for profile models. The facade override only controls tool schema format — it doesn't enable provider-specific features like Claude thinking or Codex Responses API. The Rust dispatch should use the facade for tool definition selection only, while keeping the underlying HTTP transport as OpenAI-compatible. This matches how Copilot handles model-family facades.
  #
  #   Q: How should the NAPI boundary propagate the facade override? Option A: Add optional facade_override param to sessionSetModelProfile. Option B: Add a separate sessionSetFacadeOverride NAPI call. Option C: Encode facade in the model string (e.g., 'openai:work-vllm/model@codex').
  #   A: Option A: Add optional facade_override param to sessionSetModelProfile. This is the cleanest approach — the facade is an attribute of the model selection, not a separate concern. The NAPI signature becomes sessionSetModelProfile(sessionId, providerId, modelId, facadeOverride?). In Rust, set_model_direct stores the override alongside the selected model, and the agent loop checks it before defaulting to provider-type-based facade dispatch.
  #
  #   Q: Should we validate the custom model id against the server when adding it (fire a test request to /v1/models or /v1/chat/completions), or trust the user's input entirely? Validation adds confidence but requires a running server.
  #   A: No server validation when adding a custom model — trust the user entirely. The whole point of custom models is to add models that might not be listed anywhere. The server may not be running during config. Model ID is a free-form string.
  #
  #   Q: When vLLM /v1/models endpoint fails but custom models exist, should the profile section still show '(unreachable)' in the header? Or should it show normally since the user has manually configured models? The unreachable flag affects UX — user might think the profile is broken.
  #   A: When custom models exist but /v1/models fails, show the profile section WITHOUT '(unreachable)' in the header. The unreachable status should only prevent showing auto-discovered models, not custom ones. The section is usable because the user has manually configured models. This makes profiles useful even with servers that never implement /v1/models.
  #
  # ASSUMPTIONS:
  #   1. MODEL-005 (per-model context window/max output configuration) will land before MODEL-004 enters implementing phase, providing the infrastructure for per-model settings on NapiModelInfo and ModelSelection
  #   2. All facade implementations already exist in the Rust codebase (Claude, OpenAI, Codex, Gemini, ZAI tool facades) — MODEL-004 only needs to add the dispatch override, not create new facades
  #   3. The ProviderSettingsScreen profile form pattern (arrow key field navigation, Enter to save, Escape to cancel) is a proven UX pattern that users already understand — we reuse it for the custom model form
  #
  # ========================================
  Background: User Story
    As a developer using a custom OpenAI-compatible server
    I want to manually add models and configure their facade type
    So that I can use models not listed in /v1/models and get correct tool call formatting

  # ========================================
  # Custom Model Registration
  # ========================================
  @config
  @happy-path
  Scenario: Add a custom model to a profile
    Given I have a profile "work-vllm" configured with baseUrl "http://localhost:8888"
    And the profile has no custom models defined
    When I add a custom model with id "my-fine-tuned-gpt" and displayName "Fine-Tuned GPT" to the "work-vllm" profile
    Then the custom model "my-fine-tuned-gpt" appears in the "openai: work-vllm" section of the Model Selector
    And the model displays a yellow "[C]" badge to indicate it is a custom model
    And the model can be selected to start a session

  @config
  Scenario: Custom model persists in fspec-config.json
    Given I have a profile "work-vllm" configured with baseUrl "http://localhost:8888"
    When I add a custom model with id "my-fine-tuned-gpt" to the "work-vllm" profile
    Then the fspec-config.json file contains a "customModels" array under the "work-vllm" profile
    And the "customModels" array contains an entry with id "my-fine-tuned-gpt"
    And the custom model is present after reloading the Model Selector

  @config
  @backward-compat
  Scenario: Existing config without customModels field loads normally
    Given I have a profile "work-vllm" in fspec-config.json without a "customModels" field
    When the Model Selector loads the profile sections
    Then the profile loads successfully without errors
    And no migration is required
    And when I add the first custom model, the "customModels" array is created automatically

  @config
  Scenario: Custom model with all optional metadata fields
    Given I have a profile "work-vllm" configured with baseUrl "http://localhost:8888"
    When I add a custom model with the following settings:
      | field           | value              |
      | id              | my-reasoning-model |
      | displayName     | My Reasoning Model |
      | facade          | codex              |
      | contextWindow   | 65536              |
      | maxOutputTokens | 8192               |
      | reasoning       | true               |
      | hasVision       | true               |
    Then the model displays "[C]", "[R]", "[V]", and "[65k]" badges in the Model Selector
    And the ModelSelection includes reasoning: true, hasVision: true, contextWindow: 65536

  # ========================================
  # Custom Model Override of Auto-Discovered Models
  # ========================================
  @config
  @override
  Scenario: Custom model overrides an auto-discovered model with matching ID
    Given I have a profile "work-vllm" configured with baseUrl "http://localhost:8888"
    And the server /v1/models endpoint returns "meta-llama/Meta-Llama-3.1-405B"
    And I add a custom model with id "meta-llama/Meta-Llama-3.1-405B" and contextWindow 32768
    When the Model Selector loads the profile sections
    Then the model "meta-llama/Meta-Llama-3.1-405B" appears only once in the section
    And the model shows "[32k]" instead of the default "[128k]"
    And the model displays the "[C]" badge indicating custom override

  @config
  @override
  Scenario: Deleting a custom model that overrides an auto-discovered model
    Given I have a profile "work-vllm" with a custom model "meta-llama/Meta-Llama-3.1-405B" overriding the auto-discovered version
    And the server /v1/models endpoint returns "meta-llama/Meta-Llama-3.1-405B"
    When I delete the custom model "meta-llama/Meta-Llama-3.1-405B"
    Then the model reverts to showing with default auto-discovered metadata
    And the "[C]" badge is no longer displayed
    And the context window shows the default "[128k]"

  # ========================================
  # Unreachable Server with Custom Models
  # ========================================
  @resilience
  Scenario: Custom models appear when /v1/models endpoint is unreachable
    Given I have a profile "work-vllm" configured with baseUrl "http://localhost:8888"
    And the profile has 3 custom models defined
    And the server /v1/models endpoint is unreachable
    When the Model Selector loads the profile sections
    Then the "openai: work-vllm" section shows the 3 custom models
    And all models display the "[C]" badge
    And the section header does NOT show "(unreachable)" because custom models exist

  @resilience
  Scenario: Empty /v1/models with custom models shows profile section normally
    Given I have a profile "work-vllm" configured with baseUrl "http://localhost:8888"
    And the server /v1/models endpoint returns an empty list
    And the profile has 2 custom models defined
    When the Model Selector loads the profile sections
    Then the section shows exactly 2 models (the custom models)
    And the section header shows "openai: work-vllm" without any error indicator

  # ========================================
  # Facade Override — Tool Schema Selection
  # ========================================
  @facade
  @integration
  Scenario: Facade override to Codex changes tool schemas
    Given I have a custom model "Qwen/Qwen3-80B" with facade set to "codex"
    When I select the custom model and start a session
    Then the Rust agent loop dispatches Codex-native tool schemas
    And the tool names include "exec_command", "shell", "read_file", "grep_files", and "list_dir"
    And the HTTP transport still uses the OpenAI-compatible endpoint

  @facade
  @integration
  Scenario: Facade override to Gemini changes tool schemas
    Given I have a custom model "my-gemini-compat" with facade set to "gemini"
    When I select the custom model and start a session
    Then the Rust agent loop dispatches Gemini-native tool facades
    And the tool names include "read_file", "write_file", "run_shell_command", "search_file_content", and "list_directory"
    And the HTTP transport still uses the OpenAI-compatible endpoint

  @facade
  Scenario: No facade override uses default OpenAI tool schemas
    Given I have a custom model "my-model" with no facade override specified
    When I select the custom model and start a session
    Then the Rust agent loop dispatches standard OpenAI tool schemas
    And the default ProviderType::OpenAI facade is used

  @facade
  Scenario: Facade override does not activate provider-specific features
    Given I have a custom model "my-claude-compat" with facade set to "claude"
    When I select the custom model and start a session
    Then the tool schemas use Claude-native format
    And thinking config is NOT activated for this profile model
    And the HTTP transport remains OpenAI-compatible

  @facade
  @napi
  Scenario: Facade override propagates through NAPI boundary
    Given I have a custom model with facade set to "gemini"
    When the model selection service calls sessionSetModelProfile
    Then the facadeOverride parameter "gemini" is passed through the NAPI binding
    And the Rust ProviderManager stores the facade override alongside the selected model
    And the agent loop checks the facade override before defaulting to provider-type dispatch

  # ========================================
  # TUI Keybinds and Form Flow
  # ========================================
  @tui
  Scenario: Add custom model via 'a' keybind on profile section
    Given the Model Selector is open and focused on the "openai: work-vllm" profile section header
    When I press the "a" key
    Then a custom model form opens with empty fields
    And the form displays fields: Model ID, Display Name, Facade, Context Window, Max Output, Reasoning, and Vision
    And the cursor starts on the Model ID field

  @tui
  Scenario: Edit custom model via 'e' keybind
    Given the Model Selector is open and I have a custom model "my-fine-tuned-gpt" in the "work-vllm" profile
    And the cursor is on the custom model "my-fine-tuned-gpt"
    When I press the "e" key
    Then a custom model form opens pre-filled with the existing settings
    And I can change the facade from the default to "claude"
    And pressing Enter saves the updated configuration

  @tui
  Scenario: Delete custom model via 'd' keybind with confirmation
    Given the Model Selector is open and I have a custom model "my-fine-tuned-gpt" in the "work-vllm" profile
    And the cursor is on the custom model "my-fine-tuned-gpt"
    When I press the "d" key
    Then a deletion confirmation prompt appears
    And confirming the deletion removes the model from the "customModels" array in fspec-config.json

  @tui
  @boundary
  Scenario: Add keybind is ignored on cloud provider sections
    Given the Model Selector is open and focused on the "anthropic" cloud provider section
    When I press the "a" key
    Then nothing happens
    And the Model Selector remains in its current state
    And no custom model form opens

  @tui
  Scenario: Cancel custom model form with Escape
    Given the custom model form is open for adding a new model
    When I press the Escape key
    Then the form closes without saving
    And no changes are made to fspec-config.json
    And the Model Selector returns to the normal browsing state

  @tui
  Scenario: Form field navigation with arrow keys
    Given the custom model form is open
    When I press the Down arrow key
    Then the cursor moves to the next form field
    And when I press the Up arrow key, the cursor moves to the previous field
    And this matches the ProviderSettingsScreen profile form navigation pattern
