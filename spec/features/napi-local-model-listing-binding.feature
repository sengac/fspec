@napi
@PROV-006
@providers
Feature: NAPI Local Model Listing Binding
  """
  NAPI binding models_list_local_openai wraps OpenAIProvider::list_local_models
  """

  Background: User Story
    As a TUI developer
    I want to call a NAPI function to list models from a local OpenAI-compatible server
    So that I can display available models in the model selection dialog

  @model-list
  @integration
  Scenario: NAPI binding exposes local model listing to TUI
    Given I have a local server running at "http://localhost:8888"
    When I call the NAPI function models_list_local_openai("http://localhost:8888")
    Then the function should return an array of model IDs
    And the server's /v1/models endpoint returns models "model-a" and "model-b"
    And the array should contain "model-a" and "model-b"
