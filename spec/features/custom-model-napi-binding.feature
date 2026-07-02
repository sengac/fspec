@done
@RPC-347
@napi
@model-selection
Feature: Custom-model add NAPI binding
  """
  RPC-347 slice: the add_custom_model NAPI binding accepts a
  CustomModelDefinition-shaped object and persists it through the RPC-346
  profile_sections writer, preserving every supplied field across the NAPI
  boundary. Test: codelet/napi/tests/custom_model_crud_napi_test.rs
  """

  Background: User Story
    As the JS/TS host driving the model selector
    I want a NAPI add_custom_model binding that persists a CustomModelDefinition
    So that the Node side can create custom models through the same write path

  Scenario: CustomModelDefinition round-trips through the NAPI boundary
    Given an openai profile "work-vllm" exists with no custom models
    When the NAPI add_custom_model binding receives a CustomModelDefinition-shaped object for "work-vllm"
    Then the call resolves successfully
    And the persisted entry carries the same id and optional fields that were supplied
