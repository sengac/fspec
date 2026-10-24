@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Custom runtime profile capabilities
  """
  Runtime-only profiles inherit the logical custom provider's model capabilities. Session creation and model switching use the shared model resolver, and the tool capability registry must agree with the selected custom model even when its upstream identifier has no catalog entry.
  """

  Background: User Story
    As a developer configuring custom providers
    I want runtime profiles to retain the selected model capabilities
    So that choosing a profile does not disable image support

  Scenario: Custom runtime profiles retain provider model capabilities
    Given a custom provider has a vision-capable model and a named runtime profile without connection overrides
    When that profile model is selected for a session
    Then its vision capability and image limit remain those of the custom model and profile
