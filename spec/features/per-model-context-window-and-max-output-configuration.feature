@done
@MODEL-005
Feature: Per-Model Context Window and Max Output Configuration
  """
  Rust changes in rust/providers/src/manager.rs: (1) Add model_context_window: Option<usize> and model_max_output_tokens: Option<usize> to ProviderManager struct. (2) context_window() returns self.model_context_window.unwrap_or_else(|| self.provider_constant_context_window()). (3) max_output_tokens() does the same. (4) select_model() extracts ModelInfo.limit.context/output and stores them. (5) set_model_direct() gains optional context_window/max_output params. (6) for_testing() gains optional context_window/max_output params. (7) All constructors (new, with_provider, with_provider_and_model, with_model_support) initialize both fields to None.
  NAPI changes in rust/napi/src/session_manager.rs: (1) session_set_model gains optional context_window: Option<u32> and max_output_tokens: Option<u32> params. For registry-based models, the Rust select_model() already reads from ModelInfo — but the NAPI params serve as overrides (e.g. TypeScript custom model config could override models.dev data). (2) session_set_model_profile gains the same params and passes them to set_model_direct(). These are required for profile models since they have no registry data.
  TypeScript changes in src/tui/services/modelSelectionService.ts: Pass selection.contextWindow and selection.maxOutput as additional arguments to sessionSetModel() and sessionSetModelProfile(). The NAPI bindings (rust/napi/index.d.ts) will need updated type signatures.
  No compaction engine changes needed — calculate_usable_context() in rust/cli/src/compaction_threshold.rs and CompactionHook in rust/core/src/compaction_hook.rs both already read from ProviderManager::context_window() and max_output_tokens(). The fix is entirely at the data source level.
  Private helper method needed: provider_constant_context_window() extracts the existing match arm logic from context_window() into a separate method. Similarly provider_constant_max_output_tokens() for max_output_tokens(). This keeps the fallback chain clean and testable.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ProviderManager must resolve context_window with priority: model-specific override > models.dev per-model metadata > environment variable (e.g. OPENAI_CONTEXT_WINDOW) > provider-level compile-time constant
  #   2. ProviderManager must resolve max_output_tokens with the same priority chain: model-specific override > models.dev per-model metadata > environment variable > provider-level compile-time constant
  #   3. ProviderManager struct must gain model_context_window: Option<usize> and model_max_output_tokens: Option<usize> fields that are set during select_model() and set_model_direct()
  #   4. select_model() must extract LimitInfo (context, output) from the ModelInfo returned by validate_model_for_use() and store both on ProviderManager
  #   5. The NAPI functions session_set_model and session_set_model_profile must accept optional context_window: Option<u32> and max_output_tokens: Option<u32> parameters and pass them through to ProviderManager
  #   6. The TypeScript modelSelectionService must pass ModelSelection.contextWindow and ModelSelection.maxOutput through to the NAPI calls (sessionSetModel / sessionSetModelProfile)
  #   7. For registry-based models (select_model path), the per-model values come from ModelInfo.limit (context, output) from models.dev — the NAPI parameters serve as overrides from the TypeScript side only when the registry is not available (profile models)
  #   8. set_model_direct() (used for profiles/codex) must accept optional context_window and max_output_tokens parameters since it has no registry to look up metadata — these come from the NAPI call which gets them from TypeScript ModelSelection
  #   9. When model_context_window is None (no per-model data), context_window() must fall back to the existing provider-constant behavior including env var reads — full backward compatibility
  #   10. The compaction engine requires no changes — it already reads from ProviderManager::context_window() and max_output_tokens(), so it automatically uses per-model values once ProviderManager propagates them
  #   11. The for_testing() constructor must also accept optional context_window and max_output_tokens for test setup
  #   12. Provider-level compile-time constants (claude::CONTEXT_WINDOW, openai::CONTEXT_WINDOW, etc.) must remain as fallback defaults and must not be removed
  #   13. Yes, NAPI params should override models.dev data. Priority: NAPI override (from TypeScript) > models.dev data (from select_model registry lookup) > env var > provider constant. For select_model path: first store from ModelInfo.limit, then overwrite with NAPI params if Some. This keeps MODEL-004 custom model overrides possible.
  #   14. with_provider_and_model() is used by: (1) deep_search_handler sub-agents, (2) session creation for profile/codex models, (3) graph operations. None of these pass context_window. The fix is to add optional context_window and max_output_tokens params to with_provider_and_model() as well, so callers that have this info can pass it. For deep_search sub-agents, the parent session's ProviderManager already has the correct values — we can extract them before spawning and pass them through.
  #
  # EXAMPLES:
  #   1. OpenAI o3 (200k context from models.dev) currently compacts at 128k (the OpenAI provider constant). After fix: select_model('openai/o3') stores model_context_window=200000 from ModelInfo.limit.context, and context_window() returns 200000. Compaction threshold becomes 200000-8192=191808 instead of 128000-4096=123904.
  #   2. A 32k context local model on a vLLM profile. TypeScript has ModelSelection.contextWindow=32000. sessionSetModelProfile passes context_window=32000. set_model_direct stores model_context_window=Some(32000). context_window() returns 32000. Compaction correctly triggers at ~32000-4096=27904 tokens instead of 128000-4096=123904 (which would cause API rejection).
  #   3. Copilot proxying gemini-2.5-pro (1M context). Currently limited to 200k (copilot::CONTEXT_WINDOW). After fix: When user selects github-copilot/gemini-2.5-pro, select_model looks up ModelInfo and finds limit.context=1000000. context_window() returns 1000000 instead of 200000, dramatically increasing usable context.
  #   4. No model selected yet (fresh ProviderManager). model_context_window is None. context_window() falls through to existing match on provider type, returning claude::CONTEXT_WINDOW (200000) for Claude. Full backward compatibility.
  #   5. OPENAI_CONTEXT_WINDOW env var set to 32000 for a local OpenAI-compatible server, but no per-model data (model_context_window=None). context_window() falls back to provider constant logic which reads the env var and returns 32000. Existing env var workaround still works.
  #   6. Claude model with models.dev context=200000. select_model stores model_context_window=Some(200000). Even though this matches the current constant, the mechanism works — and when Anthropic ships 1M models to broader tiers, models.dev metadata will automatically propagate the larger window without code changes.
  #   7. Codex model selected via sessionSetModelProfile (bypasses registry). TypeScript passes context_window=272000 from ModelSelection. set_model_direct stores model_context_window=Some(272000). This matches current behavior but now flows through the unified mechanism.
  #   8. max_output_tokens resolution: OpenAI o3 has max_output=100000 from models.dev. After select_model, model_max_output_tokens=Some(100000). max_output_tokens() returns 100000. calculate_usable_context(200000, 100000) → 200000 - min(100000, 32000) = 168000 (SESSION_OUTPUT_TOKEN_MAX caps the reservation).
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should select_model() apply an NAPI-passed context_window override ON TOP of the models.dev data? Or should the NAPI override only apply for set_model_direct (profile models)? Current plan: select_model reads from ModelInfo.limit; NAPI context_window override (if provided) takes precedence over both. This means TypeScript custom model config (MODEL-004 future) can override models.dev data.
  #   A: Yes, NAPI params should override models.dev data. Priority: NAPI override (from TypeScript) > models.dev data (from select_model registry lookup) > env var > provider constant. For select_model path: first store from ModelInfo.limit, then overwrite with NAPI params if Some. This keeps MODEL-004 custom model overrides possible.
  #
  #   Q: How should we handle the ProviderManager::with_provider_and_model() constructor used during compaction? It creates a new manager with just provider+model but no registry. Currently context_window() falls through to provider constants. After the fix, the compaction-spawned manager won't have model_context_window set. Should the compaction path also pass context_window/max_output when recreating the manager?
  #   A: with_provider_and_model() is used by: (1) deep_search_handler sub-agents, (2) session creation for profile/codex models, (3) graph operations. None of these pass context_window. The fix is to add optional context_window and max_output_tokens params to with_provider_and_model() as well, so callers that have this info can pass it. For deep_search sub-agents, the parent session's ProviderManager already has the correct values — we can extract them before spawning and pass them through.
  #
  # ASSUMPTIONS:
  #   1. models.dev data is generally accurate for context window and max output values — it is the canonical data source for model metadata
  #   2. The NAPI binding signature change (adding optional params) is backward-compatible since napi-rs supports Option<T> params which map to TypeScript optional params
  #   3. MODEL-004 (Custom Model Registration) has not been implemented yet — this work creates the infrastructure it will plug into
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to have compaction and token limits respect my actual model's context window
    So that my conversation context is managed correctly regardless of which specific model I'm using

  # ---------------------------------------------------------------------------
  # Registry-based model selection (select_model path)
  # ---------------------------------------------------------------------------
  @rust
  @provider-manager
  Scenario: Cloud model gets per-model context window from models.dev registry
    Given the model registry contains "openai/o3" with context=200000 and max_output=100000
    And the OpenAI provider-level constant is 128000
    When I call select_model("openai/o3")
    Then model_context_window should be 200000
    And model_max_output_tokens should be 100000
    And context_window() should return 200000
    And max_output_tokens() should return 100000

  @rust
  @provider-manager
  Scenario: Copilot proxy model gets per-model context from registry
    Given the model registry contains "github-copilot/gemini-2.5-pro" with context=1000000 and max_output=8192
    And the Copilot provider-level constant is 200000
    When I call select_model("github-copilot/gemini-2.5-pro")
    Then context_window() should return 1000000
    And max_output_tokens() should return 8192

  @rust
  @provider-manager
  Scenario: Claude model gets per-model context from registry
    Given the model registry contains "anthropic/claude-sonnet-4" with context=200000 and max_output=8192
    When I call select_model("anthropic/claude-sonnet-4")
    Then model_context_window should be 200000
    And context_window() should return 200000

  @rust
  @provider-manager
  @napi
  Scenario: Profile model gets context window through NAPI parameters
  # ---------------------------------------------------------------------------
  # Profile-based model selection (set_model_direct path via NAPI)
  # ---------------------------------------------------------------------------
    Given a vLLM profile model with ModelSelection.contextWindow=32000 and maxOutput=4096
    When sessionSetModelProfile is called with context_window=32000 and max_output_tokens=4096
    Then set_model_direct stores model_context_window=32000 and model_max_output_tokens=4096
    And context_window() should return 32000
    And max_output_tokens() should return 4096

  @rust
  @provider-manager
  @napi
  Scenario: Codex model gets context window through NAPI parameters
    Given a Codex model with ModelSelection.contextWindow=272000 and maxOutput=4096
    When sessionSetModelProfile is called with context_window=272000 and max_output_tokens=4096
    Then set_model_direct stores model_context_window=272000 and model_max_output_tokens=4096
    And context_window() should return 272000

  @rust
  @provider-manager
  @napi
  Scenario: NAPI override takes priority over models.dev metadata
  # ---------------------------------------------------------------------------
  # NAPI override takes priority over models.dev data
  # ---------------------------------------------------------------------------
    Given the model registry contains "openai/gpt-4o" with context=128000 and max_output=16384
    When session_set_model is called with context_window=64000 and max_output_tokens=8192
    Then context_window() should return 64000
    And max_output_tokens() should return 8192

  @rust
  @provider-manager
  Scenario: No model selected falls back to provider constant
  # ---------------------------------------------------------------------------
  # Backward compatibility: fallback to provider constants
  # ---------------------------------------------------------------------------
    Given a fresh ProviderManager with Claude as the current provider
    And no model has been selected
    Then model_context_window should be None
    And context_window() should return 200000
    And max_output_tokens() should return 8192

  @rust
  @provider-manager
  Scenario: Environment variable override still works when no per-model data
    Given a fresh ProviderManager with OpenAI as the current provider
    And no model has been selected
    And OPENAI_CONTEXT_WINDOW is set to "32000"
    And OPENAI_MAX_OUTPUT_TOKENS is set to "8192"
    Then model_context_window should be None
    And context_window() should return 32000
    And max_output_tokens() should return 8192

  @rust
  @compaction
  Scenario: Compaction threshold uses per-model context window for large-context model
  # ---------------------------------------------------------------------------
  # Compaction threshold correctly uses per-model values
  # ---------------------------------------------------------------------------
    Given a ProviderManager with model_context_window=200000 and model_max_output_tokens=100000
    When the compaction threshold is calculated
    Then calculate_usable_context(200000, 100000) should return 168000
    And compaction triggers when effective tokens exceed 168000

  @rust
  @compaction
  Scenario: Compaction threshold uses per-model context window for small-context model
    Given a ProviderManager with model_context_window=32000 and model_max_output_tokens=4096
    When the compaction threshold is calculated
    Then calculate_usable_context(32000, 4096) should return 27904
    And compaction triggers when effective tokens exceed 27904

  @typescript
  @integration
  Scenario: modelSelectionService passes contextWindow and maxOutput to sessionSetModel
  # ---------------------------------------------------------------------------
  # TypeScript integration: modelSelectionService passes values to NAPI
  # ---------------------------------------------------------------------------
    Given a ModelSelection with providerId="openai" and modelId="o3" and contextWindow=200000 and maxOutput=100000
    And an active session exists
    When selectModel is called
    Then sessionSetModel is called with context_window=200000 and max_output_tokens=100000

  @typescript
  @integration
  Scenario: modelSelectionService passes contextWindow and maxOutput to sessionSetModelProfile
    Given a ModelSelection with profileConfig and contextWindow=32000 and maxOutput=4096
    And an active session exists
    When selectModel is called
    Then sessionSetModelProfile is called with context_window=32000 and max_output_tokens=4096

  @rust
  @provider-manager
  Scenario: with_provider_and_model accepts optional context window parameters
  # ---------------------------------------------------------------------------
  # with_provider_and_model constructor supports optional context params
  # ---------------------------------------------------------------------------
    Given I create a ProviderManager via with_provider_and_model("claude", "claude-sonnet-4", context_window=200000, max_output_tokens=8192)
    Then context_window() should return 200000
    And max_output_tokens() should return 8192

  @rust
  @provider-manager
  Scenario: with_provider_and_model without context params falls back to provider constant
    Given I create a ProviderManager via with_provider_and_model("claude", "claude-sonnet-4") with no context params
    Then context_window() should return 200000
    And max_output_tokens() should return 8192

  @rust
  @provider-manager
  Scenario: for_testing constructor with custom context window
  # ---------------------------------------------------------------------------
  # for_testing constructor supports optional context params
  # ---------------------------------------------------------------------------
    Given I create a test ProviderManager via for_testing(OpenAI, context_window=200000, max_output_tokens=100000)
    Then context_window() should return 200000
    And max_output_tokens() should return 100000

  @rust
  @provider-manager
  Scenario: Provider-level compile-time constants remain unchanged
  # ---------------------------------------------------------------------------
  # Provider constants remain as fallback defaults
  # ---------------------------------------------------------------------------
    Then claude::CONTEXT_WINDOW should be 200000
    And openai::CONTEXT_WINDOW should be 128000
    And gemini::CONTEXT_WINDOW should be 1000000
    And codex::CONTEXT_WINDOW should be 272000
    And zai::CONTEXT_WINDOW should be 128000
    And copilot::CONTEXT_WINDOW should be 200000
