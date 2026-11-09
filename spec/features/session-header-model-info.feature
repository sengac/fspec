@done
@TUI-001
@PROV-166
@tui
@model-selection
Feature: Session header model info and size badge
  """
  The SessionHeader's model segment comes from get_model_info: resolve_model_info
  (rust/sessions/src/cloud_models.rs) promotes the raw slug to the catalog's friendly name and
  capability flags, falling back to the raw slug. The size badge (size_badge_label,
  rust/fspec-tui/src/views/agent/header_build.rs) shows the compaction threshold, else the context
  window. PROV-166: both are Option — an unavailable value is None, never 0 — and when both are
  unavailable the header shows no size badge. This feature records the scenarios that
  rust/fspec-tui/tests/tui001_session_header_model_info.rs already named but that had no feature file.
  """

  Background: User Story
    As a fspec-tui user looking at the session header
    I want the model's friendly name, capabilities and size to be shown only from real data
    So that the header never shows a guessed or zero context size

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

  Scenario: Size badge uses the compaction threshold when it is available
    Given a ModelInfo with compaction_threshold 192000 and context_window 200000
    When the size badge value is computed for the left header line
    Then the size badge shows "192k"

  Scenario: Size badge falls back to the context window when the compaction threshold is unavailable
    Given a ModelInfo with no compaction_threshold and context_window 200000
    When the size badge value is computed for the left header line
    Then the size badge shows "200k"

  Scenario: No size badge is shown when both limits are unavailable
    Given a ModelInfo with no compaction_threshold and no context_window
    When the size badge value is computed for the left header line
    Then no size badge is shown
