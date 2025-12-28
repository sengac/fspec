@done
@providers
@provider
@model-selection
@cache
@MODEL-001
Feature: Dynamic Model Selection via models.dev
  """

  LAYER ARCHITECTURE:
  1. User/CLI Layer: --model provider/model-id flag
  2. ModelRegistry: parse_model_string(), get_model(), list_providers(), list_models(), filter_by_capability()
  3. ModelCache: get() returns cached data, refresh() forces API fetch, indefinite cache at ~/.fspec/cache/models.json (configurable via set_cache_directory)
  4. ProviderFacade: Defines HOW to talk to provider (api_format, auth_headers, thinking_facade, system_prompt_facade)
  5. Existing Facades: ThinkingConfigFacade, SystemPromptFacade, ToolFacade

  DATA FLOW - Startup:
  1. codelet starts
  2. ModelCache.get() - use cache if valid, fetch if missing/corrupted
  3. If fetch needed and network fails, use embedded fallback snapshot
  4. ModelRegistry.init() - parse providers, build capability index

  DATA FLOW - Model Selection:
  1. Parse 'provider/model-id' format (no aliases)
  2. Validate provider exists in registry
  3. Validate model exists in provider
  4. Validate model.tool_call == true (required)
  5. Get ProviderFacade for provider
  6. ProviderFacade returns appropriate ThinkingConfigFacade based on model.reasoning
  7. Configure agent with model limits (context, output) from metadata

  FILE STRUCTURE:
  - codelet/providers/src/models/cache.rs - ModelCache
  - codelet/providers/src/models/registry.rs - ModelRegistry
  - codelet/providers/src/models/types.rs - Provider, Model, Capabilities structs
  - codelet/providers/src/facade/traits.rs - ProviderFacade trait
  - codelet/providers/src/facade/anthropic.rs - AnthropicFacade
  - codelet/providers/src/facade/google.rs - GoogleFacade
  - codelet/providers/src/facade/openai.rs - OpenAIFacade

  BUILD INTEGRATION:
  - build.rs fetches models.dev and embeds as FALLBACK_MODELS using include_bytes!

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Model metadata MUST come from models.dev API, never hardcoded
  #   2. Cache is indefinite - only refetch when cache missing, corrupted, or user requests refresh
  #   3. ProviderFacade defines HOW to talk to a provider (API format, auth, thinking config), NOT what models exist
  #   4. ModelRegistry provides lookup, alias resolution, and capability filtering
  #   5. Each model has capabilities from models.dev: reasoning, tool_call, attachment, temperature, modalities
  #   6. Unknown model should return error with fuzzy match suggestions
  #   7. Cache stored at ~/.fspec/cache/models.json (default, configurable via NAPI bindings for fspec integration)
  #   8. Provider facades integrate with existing ThinkingConfigFacade and SystemPromptFacade
  #   9. Build process fetches latest models.dev data and embeds as fallback snapshot in binary
  #   10. Selected models MUST have tool_call=true capability, reject models without it
  #   11. Model strings use full format only: provider/model-id (e.g., anthropic/claude-sonnet-4)
  #
  # EXAMPLES:
  #   1. User runs 'codelet --model google/gemini-2.5-pro' and full model path is used directly
  #   2. First run with no cache fetches from models.dev and creates ~/.fspec/cache/models.json
  #   3. Subsequent runs use cached models.json without network call
  #   4. User runs 'codelet models --refresh' to force fetch fresh data from models.dev
  #   5. User runs 'codelet models anthropic' to list all Anthropic models with names and capabilities
  #   6. User runs 'codelet --model anthropic/claud' (typo) and gets error with suggestions
  #   7. User runs 'codelet models --reasoning' to list only models with reasoning capability
  #   8. Corrupted cache file triggers automatic refetch from models.dev
  #   9. Selected model's capabilities determine which ThinkingConfigFacade is used
  #   10. User runs 'codelet --model anthropic/claude-sonnet-4' and model is selected from registry
  #   11. User runs 'codelet models --providers' to list all available providers with model counts
  #   12. User runs 'codelet models --vision' to filter models with image input capability
  #   13. User runs 'codelet models --search "claude"' to fuzzy search across all providers
  #   14. User runs 'codelet models anthropic/claude-sonnet-4 --verbose' for detailed model info
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we embed a fallback snapshot of models.dev for first-run when network is unavailable?
  #   A: Yes, embed a fallback snapshot. Include a build step that fetches latest from models.dev and embeds it in the binary.
  #
  #   Q: Should codelet require tool_call=true capability for all selected models?
  #   A: Yes, require tool_call=true. Only allow models that support function calling to ensure codelet features work.
  #
  #   Q: Should we support custom/self-hosted providers not in models.dev?
  #   A: Defer to later phase. Not in scope for MODEL-001, add as separate work unit.
  #
  #   Q: What default aliases should we ship with? (sonnet, opus, haiku, flash, pro, gpt4, o1?)
  #   A: No built-in aliases. Users must use full model paths (e.g., anthropic/claude-sonnet-4).
  #
  # ========================================
  Background: User Story
    As a developer using codelet
    I want to select from available LLM models across providers
    So that I can choose the best model for my task without hardcoded limitations

  Scenario: Select model using full provider/model path
    Given the models cache contains anthropic provider with claude-sonnet-4 model
    When I run codelet with --model anthropic/claude-sonnet-4
    Then the model should be selected from the registry
    And the appropriate provider facade should be used

  Scenario: First run fetches models from API and creates cache
    Given no models cache file exists at ~/.fspec/cache/models.json
    When I run codelet
    Then models.dev API should be called to fetch model data
    And the cache file should be created at ~/.fspec/cache/models.json

  Scenario: Subsequent runs use cached models without network call
    Given a valid models cache file exists
    When I run codelet
    Then models.dev API should NOT be called
    And models should be loaded from the cache file

  Scenario: Force refresh fetches fresh data from API
    Given a models cache file exists
    When I run codelet models --refresh
    Then models.dev API should be called
    And the cache file should be updated with fresh data

  Scenario: List models for a specific provider
    Given the models cache contains anthropic provider with multiple models
    When I run codelet models anthropic
    Then all Anthropic models should be displayed
    And each model should show name and capabilities

  Scenario: Unknown model shows error with fuzzy suggestions
    Given the models cache contains anthropic provider with claude models
    When I run codelet with --model anthropic/claud
    Then an error should be displayed: "Model 'claud' not found in provider 'anthropic'"
    And fuzzy match suggestions should be shown: claude-sonnet-4, claude-opus-4, claude-haiku

  Scenario: Corrupted cache triggers automatic refetch
    Given the cache file exists but contains invalid JSON
    When I run codelet
    Then models.dev API should be called to fetch fresh data
    And the cache file should be replaced with valid data

  Scenario: Reject model without tool_call capability
    Given the cache contains google/gemini-2.0-flash-thinking-exp with tool_call=false
    When I run codelet with --model google/gemini-2.0-flash-thinking-exp
    Then an error should be displayed: "Model google/gemini-2.0-flash-thinking-exp does not support tool_call"
    And the command should exit with code 1

  Scenario: Filter models by reasoning capability
    Given the cache contains models with and without reasoning capability
    When I run codelet models --reasoning
    Then only models with reasoning=true should be displayed

  Scenario: Use embedded fallback when network unavailable on first run
    Given no cache file exists
    And models.dev API is unreachable
    When I run codelet
    Then the embedded fallback snapshot should be used
    And codelet should function with the fallback data

  Scenario: List all available providers
    Given the models cache contains anthropic, google, and openai providers
    When I run codelet models --providers
    Then a list of providers should be displayed
    And each provider should show id, name, and model count

  Scenario: Filter models by vision capability
    Given the cache contains models with and without vision capability
    When I run codelet models --vision
    Then only models with image input modality should be displayed

  Scenario: Search models with fuzzy matching
    Given the cache contains multiple models from various providers
    When I run codelet models --search "claude"
    Then all models containing "claude" in name or id should be displayed
    And results should include anthropic/claude-sonnet-4 and anthropic/claude-opus-4

  Scenario: Show verbose model details
    Given the cache contains anthropic/claude-sonnet-4 model
    When I run codelet models anthropic/claude-sonnet-4 --verbose
    Then detailed model information should be displayed
    And the output should include context limit, output limit, capabilities, and cost
