@done
@session-management
@provider-management
@tui
@TUI-001
Feature: Resolve model display name, capability badges and compaction-threshold size badge in get_model_info

  """
  Pure helper resolve_model_info(registry: Option<&ModelRegistry>, provider_id, model_id, context_window, compaction_threshold) -> ModelInfo encapsulates the catalog lookup + fallback so it is unit-testable without a live session.
  Do NOT modify the rendering layer (header.rs colours/ordering). Only header_build.rs size-badge value selection changes, plus the server data-feed (handle_impl.rs, cloud_models.rs helper, rpc-types ModelInfo field).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. get_model_info resolves the friendly display_name from the models.dev catalog via registry.get_model(canonical_to_models_dev(provider_id), model_id) when the model is found
  #   2. On a catalog hit, supports_reasoning = catalog.reasoning and supports_vision = catalog.has_capability(Vision)
  #   3. On a catalog miss (unknown provider/model or no registry), fall back to the raw model_id with supports_reasoning=false and supports_vision=false
  #   4. ModelInfo carries a new compaction_threshold: u32 field populated from the session's cached_compaction_threshold
  #   5. The size badge value is compaction_threshold when > 0, otherwise context_window, formatted via format_context_window (192000 -> 192k)
  #
  # EXAMPLES:
  #   1. A known anthropic model (provider_id=anthropic, model_id=claude-opus-4-8) resolves to display_name 'Claude Opus 4.8' with [R] and [V] badges
  #   2. A gemini provider maps via canonical_to_models_dev to google before the registry lookup
  #   3. An unknown model id keeps the raw slug as display_name with no reasoning/vision flags
  #   4. A ModelInfo with compaction_threshold=192000 and context_window=200000 produces a [192k] size badge
  #   5. A ModelInfo with compaction_threshold=0 and context_window=200000 falls back to a [200k] size badge
  #
  # ========================================

  Background: User Story
    As a developer using the Rust ratatui AgentView
    I want to see the friendly model name, [R]/[V] capability badges and a compaction-threshold size badge in the SessionHeader
    So that the Rust TUI header reaches parity with the TypeScript Ink reference

  Scenario: Known catalog model resolves friendly name and capability flags
    Given a model registry containing the anthropic model "claude-opus-4-8" with name "Claude Opus 4.8", reasoning true and vision true
    When resolve_model_info runs for provider "anthropic" and model "claude-opus-4-8"
    Then the resolved display_name is "Claude Opus 4.8"
    And the resolved supports_reasoning is true
    And the resolved supports_vision is true

  Scenario: Gemini provider slug is mapped to google before lookup
    Given a model registry containing the google model "gemini-2.5-pro" with name "Gemini 2.5 Pro"
    When resolve_model_info runs for provider "gemini" and model "gemini-2.5-pro"
    Then the resolved display_name is "Gemini 2.5 Pro"

  Scenario: Unknown model falls back to the raw slug with no capability flags
    Given a model registry that does not contain the model "totally-unknown-model"
    When resolve_model_info runs for provider "anthropic" and model "totally-unknown-model"
    Then the resolved display_name is "totally-unknown-model"
    And the resolved supports_reasoning is false
    And the resolved supports_vision is false

  Scenario: Size badge uses the compaction threshold when it is greater than zero
    Given a ModelInfo with compaction_threshold 192000 and context_window 200000
    When the size badge value is computed for the left header line
    Then the size badge shows "192k"

  Scenario: Size badge falls back to the context window when compaction threshold is zero
    Given a ModelInfo with compaction_threshold 0 and context_window 200000
    When the size badge value is computed for the left header line
    Then the size badge shows "200k"
