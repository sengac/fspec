@PROV-163
@providers
@provider-abstraction
Feature: GitHub Copilot HTTP backend is verified against current Copilot behaviour
  """
  fspec keeps its direct HTTP Copilot backend (OAuth device flow, Copilot token exchange, /models catalog, rig
  completion pipeline). The model catalog (rust/providers/src/copilot/models/builder.rs) now also hides entries whose
  `policy.state` is "disabled" (CopilotModelEntry gains an optional `policy`), in addition to model_picker_enabled.
  An opt-in live smoke test (#[ignore], FSPEC_COPILOT_LIVE=1, stored fspec Copilot credentials) checks token exchange,
  the models endpoint and one completion against a real account; ordinary runs never need a subscription.
  Research: spec/attachments/PROV-163/fspec-copilot-comparison.md.
  """

  Background: User Story
    As an fspec user with a GitHub Copilot subscription
    I want fspec's Copilot backend to match what Copilot actually serves
    So that I only see models I can use and every fspec feature works with Copilot

  Scenario: Policy-disabled models are hidden from the Copilot catalog
    Given a Copilot models response with one enabled, one policy-disabled and one policy-less picker model
    When the catalog is built
    Then the enabled and the policy-less models are listed
    And the policy-disabled model is not listed

  Scenario: The live Copilot backend completes a request with stored credentials
    Given stored fspec GitHub Copilot credentials and FSPEC_COPILOT_LIVE set
    When the backend exchanges its token, lists models and sends one short completion
    Then the token exchange succeeds, at least one model is listed and the completion returns text
