@BUG-582
@done
@providers
@context-window
@rust
@BUG-139
@PROV-170
@PROV-172
Feature: Custom provider NAPI exposes per-model limits
  """
  Rust/NAPI tier of BUG-139. Widens JsProviderInfo.models from Vec<String> to
  Vec<JsProviderModelInfo> so per-model { id, contextWindow, maxOutput,
  supportsTools, supportsStreaming, supportsThinking } flows from the JSON
  ModelDef through the NAPI boundary to the TUI, and preserves PROV-095's
  Rhai script override chain (script > JSON).

  PROV-170: an undeclared context_window or max_output_tokens is unavailable
  (None) instead of a guessed 200000/4096. An undeclared model
  max_output_tokens resolves to an explicitly declared
  defaults.max_output_tokens; a declared 0 is rejected when the config loads.

  PROV-172: the provider listing carries limits as u32 (the RPC wire width). A
  declared limit that does not fit is an error naming the model, the limit and
  the value, never clamped to 4294967295.
  """

  # EXAMPLE MAPPING CONTEXT
  #
  # Rules covered by this slice:
  #   - Rule 0: JSON ModelDef context_window MUST propagate through NAPI listProviders()
  #   - Rule 1: JSON ModelDef max_output_tokens MUST propagate through NAPI
  #   - Rule 2: PROV-095 priority chain preserved (Rhai > JSON > default)
  #   - Rule 6: Widen JsProviderInfo.models from Vec<String> to Vec<JsProviderModelInfo>
  #   - Rule 7: Surface supports_tools/streaming/thinking in the NAPI widening
  #   - Rule 8 (PROV-170): an undeclared limit is unavailable, never a guessed default
  #   - Rule 9 (PROV-170): an undeclared model max_output_tokens resolves to defaults.max_output_tokens when declared
  #   - Rule 10 (PROV-170): a declared 0 limit is rejected at config load, naming the field
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

  Scenario: A ProviderConfig model that declares no limits has them unavailable
    Given a ProviderConfig JSON that omits context_window and max_output_tokens on every model
    When serde deserializes the config
    Then every model's context_window is unavailable
    And every model's max_output_tokens is unavailable

  Scenario: JSON omits context_window - list_providers_info reports it unavailable
    Given the claude-rhai provider config declares model "opus-4.7" without a context_window field
    And the Rhai script does not define get_model_limits
    When list_providers_info() resolves per-model limits
    Then the returned model entry's contextWindow is unavailable
    And the returned model entry's maxOutput is unavailable

  Scenario: Session provider listing sends no context window for a custom model that declares none
    Given a custom provider "acme" whose model "opus-4.7" declares no context_window
    When the session manager lists providers
    Then the "opus-4.7" model entry has no context window

  Scenario: RhaiCustomProvider reports undeclared limits as unavailable
    Given a Rhai provider config whose model "opus-4.7" declares no context_window and no max_output_tokens
    And the config declares no defaults.max_output_tokens
    And the Rhai script does not define get_model_limits
    When the RhaiCustomProvider is constructed for "opus-4.7"
    Then its context window is unavailable
    And its max output tokens is unavailable

  Scenario: A model without max_output_tokens uses the declared defaults.max_output_tokens
    Given a Rhai provider config whose model "opus-4.7" declares no max_output_tokens
    And the config declares defaults.max_output_tokens 8192
    When the RhaiCustomProvider is constructed for "opus-4.7"
    Then its max output tokens is 8192

  Scenario: A declared zero limit is rejected when the config is loaded
    Given a ProviderConfig JSON whose model "opus-4.7" declares context_window 0
    When the config is loaded from disk
    Then loading fails with an error naming "models.opus-4.7.context_window"
    And a config declaring defaults.max_output_tokens 0 fails naming "defaults.max_output_tokens"

  Scenario: A declared limit larger than the wire maximum fails the provider listing
    Given a custom provider "acme" whose model "opus-4.7" declares context_window 5000000000
    When list_providers_info() resolves per-model limits
    Then listing fails with an error naming "models.opus-4.7.context_window" and 5000000000
    And no model entry reports a context window of 4294967295

  Scenario: Rhai script get_model_limits still wins over JSON (PROV-095 no-regression)
    Given the claude-rhai provider config declares model "opus-4.7" with context_window 200000
    And the Rhai script defines get_model_limits returning "#{ context_window: 400000 }"
    When lookup_script_model_limits is invoked for the selected model
    Then the resolved context_window equals 400000
    And the resolved context_window is NOT 200000
