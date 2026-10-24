@done
@cli
@validation
@providers
@model-selection
@integration
@BUG-252
Feature: Distinguish unsupported model discovery from provider unavailability
  """
  AST inspected probe_profile_models in sessions/profile_sections.rs plus actual provider list_local_models_with_auth: unsupported already typed for 404/405; all other failures collapse to Api text. ProviderInfo boolean drives selector red header and startup filter. Unsupported with zero models can clear false unreachable claim safely because startup also requires nonempty models. No wire schema expansion needed. Existing profile_sections.rs/openai.rs exceed 300 lines; focused helper extraction only, no broad refactor. Historical runtime/config/executable unestablished; isolated loopback config and built test provenance only.
  """

  Background: User Story
    As a local profile user
    I want to see accurate discovery diagnostics and select configured models
    So that unsupported listing is not mistaken for failed inference

  Scenario: Unsupported discovery without configured models does not claim failed inference
    Given listing returns HTTP 404 or 405 with absent or blank configured models
    When the profile list is enumerated
    Then no models are offered and the profile is not marked unreachable
    And diagnostics report unsupported discovery and unknown inference without secrets

  Scenario: Configured model survives cold startup and switching with unsupported discovery
    Given a persisted configured model has unsupported listing and independently working inference
    When profiles are enumerated at cold startup and after switching the persisted selection
    Then startup restores the configured selection and settings retain the custom model
    And diagnostics report unsupported discovery and unknown inference without secrets

  Scenario: Successful discovery distinguishes populated and empty listings
    Given listing returns valid populated or empty model data
    When the profile list is enumerated
    Then only returned models are offered without invented capabilities
    And diagnostics distinguish supported and empty discovery with unknown inference

  Scenario: Authentication server and protocol failures block selection without leaking payloads
    Given a configured model has authentication server or malformed listing failures
    When the profile list is enumerated
    Then the configured model is retained but unavailable for startup selection
    And diagnostics identify the failure category with unknown inference and no secrets

  Scenario: Transport failures remain visible without optimistic fallback
    Given a configured model endpoint refuses connections or times out
    When the profile list is enumerated
    Then the configured model is retained but unavailable for startup selection
    And diagnostics report transport failure and unknown inference without secrets

  Scenario: Skipped probes cannot authorize configured models
    Given a configured profile is enumerated outside a Tokio runtime
    When the profile list is enumerated
    Then the configured model is retained but unavailable for startup selection
    And diagnostics report skipped discovery and unknown inference without secrets
