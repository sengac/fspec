@providers
@context-window
@rust
@BUG-139
Feature: Custom provider NAPI exposes per-model limits
  """
  Rust/NAPI tier of BUG-139. Widens JsProviderInfo.models from Vec<String> to
  Vec<JsProviderModelInfo> so per-model { id, contextWindow, maxOutput,
  supportsTools, supportsStreaming, supportsThinking } flows from the JSON
  ModelDef through the NAPI boundary to the TUI. Also changes
  default_context_window() from 128000 to 200000 and preserves PROV-095's
  Rhai script override chain (script > JSON > default).
  """

  # EXAMPLE MAPPING CONTEXT
  #
  # Rules covered by this slice:
  #   - Rule 0: JSON ModelDef context_window MUST propagate through NAPI listProviders()
  #   - Rule 1: JSON ModelDef max_output_tokens MUST propagate through NAPI
  #   - Rule 2: PROV-095 priority chain preserved (Rhai > JSON > default)
  #   - Rule 6: Widen JsProviderInfo.models from Vec<String> to Vec<JsProviderModelInfo>
  #   - Rule 7: Surface supports_tools/streaming/thinking in the NAPI widening
  #   - Rule 8: default_context_window() changes from 128000 to 200000
  Background: 
    Given a custom Rhai provider "claude-rhai" is registered via ~/.fspec/providers/

  Scenario: NAPI listProviders returns per-model limits for custom providers
    Given the claude-rhai provider config declares model "opus-4.7" with context_window 1000000 and max_output_tokens 128000 and supports_tools true and supports_streaming true and supports_thinking true
    When I call list_providers_info() via the custom-provider NAPI surface
    Then the returned "claude-rhai" entry has models containing one item
    And that item has id "opus-4.7"
    And that item has contextWindow 1000000
    And that item has maxOutput 128000
    And that item has supportsTools true
    And that item has supportsStreaming true
    And that item has supportsThinking true

  Scenario: default_context_window() default value changes to 200000
    Given a ProviderConfig JSON that omits context_window on every model
    When serde deserializes the config
    Then model.context_window equals 200000
    And it does NOT equal the previous default 128000

  Scenario: JSON omits context_window - new default 200k flows through
    Given the claude-rhai provider config declares model "opus-4.7" without a context_window field
    And the Rhai script does not define get_model_limits
    When list_providers_info() resolves per-model limits
    Then the returned model entry's contextWindow equals 200000
    And the contextWindow is NOT 128000
    And the contextWindow is NOT 120000

  Scenario: Rhai script get_model_limits still wins over JSON (PROV-095 no-regression)
    Given the claude-rhai provider config declares model "opus-4.7" with context_window 200000
    And the Rhai script defines get_model_limits returning "#{ context_window: 400000 }"
    When lookup_script_model_limits is invoked for the selected model
    Then the resolved context_window equals 400000
    And the resolved context_window is NOT 200000
