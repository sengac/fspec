@BUG-218
@done
@providers
@model-selection
@integration
Feature: Optional model discovery for configured local profiles
  """
  Discovery and inference are separate capabilities. Only HTTP 404 and 405 from
  the composed models URL are typed as unsupported discovery. With a nonblank
  configured custom model this is nonfatal, but never proof of inference health.
  Other HTTP, transport, and response failures remain failures even with custom
  models. No alternate routes or inference requests are attempted by discovery.
  This narrows the historical MODEL-004 blanket custom-model override while
  preserving PROV-131 runtime safety. Fixtures use loopback only, not RunPod.
  """

  Background: User Story
    As a local OpenAI-compatible profile user
    I want to select a configured model when optional model discovery is unsupported
    So that a model-listing 404 is not mistaken for failed inference

  Scenario: Optional discovery preserves an independently usable configured model
    Given a profile has a nonblank custom model and discovery returns HTTP 404 or 405
    And the local chat endpoint independently accepts the configured model
    When the server builds the local profile sections
    Then the configured model remains present and the profile is not marked unreachable
    And discovery does not call inference or try alternate listing routes

  Scenario: Authentication and server failures remain visible with configured models
    Given a profile has a custom model and discovery returns HTTP 401 403 or 500 or malformed JSON
    When the server builds the local profile sections
    Then the profile is marked unreachable and its custom model is retained

  Scenario: Transport failures remain visible with configured models
    Given a profile has a custom model and its local endpoint refuses connections
    When the server builds the local profile sections
    Then the profile is marked unreachable and its custom model is retained

  Scenario: Unsupported discovery without a usable configured model remains unavailable
    Given discovery returns HTTP 404 or 405 and custom models are absent or blank
    When the server builds the local profile sections
    Then the profile remains present and is not marked unreachable
    And blank model identifiers are not offered for selection

  Scenario: Successful empty discovery does not invent configured models
    Given a profile has no custom models and discovery returns an empty model list
    When the server builds the local profile sections
    Then the profile remains present with no models and is not marked unreachable

  Scenario: A configured model cannot bypass a skipped probe without a runtime
    Given a profile has a custom model and no tokio runtime is present
    When the server builds the local profile sections
    Then the profile is marked unreachable without attempting an HTTP request

  Scenario: A configured model cannot bypass a skipped probe on a current thread runtime
    Given a profile has a custom model on a current thread tokio runtime
    When the server builds the local profile sections
    Then the profile is marked unreachable without attempting an HTTP request
