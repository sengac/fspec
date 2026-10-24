@done
@providers
@integration
@provider-abstraction
@PROV-147
Feature: Codex-only adapter boundary
  """
  Codex owns a composed CompletionModel under rust/providers/src/codex/.
  Both direct completions and production agents use that model. The existing
  provider factory selects Codex by resolved identity, not by model spelling.
  Initially the adapter delegates requests and events unchanged to rig.
  No shared history, retry, UI or OpenAI defaults change. Future typed events
  must cross explicit Codex-only seams with non-Codex regression controls.
  Architecture decision: spec/attachments/PROV-147/architecture.md.
  """

  Scenario: Provider identity selects the Codex boundary
    Given Codex and OpenAI providers configured with identical model names
    When their production agents are created
    Then only Codex uses the Codex-owned compatibility boundary
    And model names without Codex still use the boundary when Codex is selected

  Scenario: Existing streamed responses remain unchanged
    Given a local response containing text reasoning function calls and usage
    When the response is streamed through the Codex boundary
    Then its request and events match the existing Responses behavior
    And ordinary OpenAI Responses requests remain unchanged

  Scenario: Existing direct completions remain unchanged
    Given a local completed response with text and usage
    When a completion is requested through the Codex boundary
    Then its request and result match the existing Responses behavior
    And ordinary OpenAI Responses requests remain unchanged
