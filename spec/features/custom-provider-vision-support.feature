@tui
@providers
@PROV-096
Feature: Custom (Rhai) provider models never advertise vision support — [V] badge missing in SessionHeader

  """
  Spans three tiers: Rust (ModelDef.supports_vision + ProviderModelInfo.supports_vision), NAPI bridge (JsProviderModelInfo.supportsVision + From impl), TypeScript (customProviderSectionBuilder forwards to NapiModelInfo.hasVision). Each tier needs its own test.
  Naming choice: JSON field `supports_vision` (snake_case, matches existing supports_tools/supports_streaming/supports_thinking). Rust field `supports_vision`. NAPI field `supports_vision` (auto-camelCased to `supportsVision` by napi-rs). TS NapiModelInfo field remains `hasVision` for SessionHeader compatibility (existing boundary contract).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The custom provider JSON ModelDef MUST support a `supports_vision` field (default false) on ModelDef so authors can declare vision capability per model
  #   2. ProviderModelInfo and JsProviderModelInfo MUST carry a `supports_vision` / `supportsVision` field propagated verbatim from ModelDef.supports_vision
  #   3. customProviderSectionBuilder.buildCustomModelInfo MUST forward `entry.supportsVision` to `NapiModelInfo.hasVision` (defaulting to false when absent) instead of hardcoding false
  #   4. When a custom provider JSON declares `supports_vision: true`, SessionHeader MUST render the blue [V] badge for that model
  #   5. When a custom provider JSON omits `supports_vision` (or sets it to false), SessionHeader MUST NOT render the [V] badge (backward compatible default)
  #
  # EXAMPLES:
  #   1. listProviders() returns a claude-rhai custom provider with model { id: 'opus-4.7', contextWindow: 200000, supportsTools: true, supportsThinking: true, supportsVision: true }; loadCustomProviderSections() yields a NapiModelInfo with hasVision === true
  #   2. listProviders() returns a claude-rhai custom provider whose only model omits supportsVision; the resulting NapiModelInfo has hasVision === false and the SessionHeader shows only [R] (no [V])
  #   3. A claude-rhai provider JSON config sets `supports_vision: true` on its `opus-4.7` model; after selecting that model in /model, the developer sees `#1: claude-rhai [R] [V] [200k]` in the SessionHeader
  #   4. An existing provider JSON config with no `supports_vision` field still loads cleanly after the change (serde default = false) and the [V] badge does not appear — no regression
  #
  # ========================================

  Background: User Story
    As a developer using a Rhai-scripted custom provider whose underlying model supports vision
    I want to see the [V] badge in the SessionHeader
    So that I can tell at a glance that my session can accept image input

  @rust @napi
  Scenario: Custom provider JSON with supports_vision true propagates to NapiModelInfo.hasVision
    Given a custom provider JSON config with a model definition
    And the model definition sets "supports_vision" to true
    When the TUI's custom provider section builder loads the model
    Then the resulting NapiModelInfo has hasVision set to true

  @rust @napi
  Scenario: Custom provider model without supports_vision defaults hasVision to false
    Given a custom provider JSON config with a model definition
    And the model definition omits the "supports_vision" field
    When the TUI's custom provider section builder loads the model
    Then the resulting NapiModelInfo has hasVision set to false

  @tui @integration
  Scenario: SessionHeader renders [V] badge for vision-enabled custom model
    Given a custom provider "claude-rhai" is registered with model "opus-4.7"
    And the model "opus-4.7" declares "supports_vision" as true
    And the developer selects the "opus-4.7" model in the /model selector
    When the AgentView renders the SessionHeader for the active session
    Then the SessionHeader shows the blue "[V]" badge alongside "[R]"

  @regression @rust @napi
  Scenario: Legacy config without supports_vision loads cleanly without regression
    Given an existing custom provider JSON config that has no "supports_vision" field anywhere
    When the provider system deserializes and loads the config
    Then all models deserialize successfully with supports_vision defaulting to false
    And the SessionHeader does not show the "[V]" badge for any of those models
