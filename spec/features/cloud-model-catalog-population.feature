@done
@RPC-073
@providers
@model-selection
Feature: Cloud Model Catalog Population

  Background: User Story
    As a fspec user driving the Rust binary
    I want the model selector to list each configured cloud provider's models
    So that I can pick a model, exactly like the TypeScript frontend that sources models from models.dev

  Scenario: Credentialed provider lists tool-call models newest-first and drops deprecated ones
    Given a models.dev registry where "anthropic" lists claude-opus-4 (tool_call, 2026-01-01), claude-sonnet-4 (tool_call, 2025-05-01) and an old deprecated model
    And the "anthropic" provider is treated as credentialed
    When cloud_model_entries is built for "anthropic"
    Then the entries contain exactly claude-opus-4 and claude-sonnet-4
    And claude-opus-4 appears before claude-sonnet-4 (newest-first)
    And the deprecated model is not present

  Scenario: Only tool-call-capable models are listed
    Given a models.dev registry where "anthropic" lists a tool_call=false chat model alongside tool_call=true models
    And the "anthropic" provider is treated as credentialed
    When cloud_model_entries is built for "anthropic"
    Then the tool_call=false model is excluded
    And every returned entry is tool-call capable

  Scenario: Gemini canonical slug is sourced from the models.dev google entry
    Given a models.dev registry where "google" lists a tool_call gemini model
    And the "gemini" provider is treated as credentialed
    When cloud_model_entries is built for "gemini"
    Then the entries are non-empty
    And they come from the models.dev "google" provider

  Scenario: Uncredentialed provider yields no models
    Given a models.dev registry where "mistral" lists tool_call models
    And the "mistral" provider is treated as NOT credentialed
    When cloud_model_entries is built for "mistral"
    Then the entries are empty

  Scenario: Provider absent from models.dev yields no models without error
    Given a models.dev registry that has no "codex" or "google" mapping for codex
    And the "codex" provider is treated as credentialed
    When cloud_model_entries is built for "codex"
    Then the entries are empty
    And no error is raised

  Scenario: Model entry fields are mapped from the models.dev model
    Given a models.dev registry where "anthropic" lists a tool_call model with name "Claude Opus 4", context 200000, reasoning true and image input modality
    And the "anthropic" provider is treated as credentialed
    When cloud_model_entries is built for "anthropic"
    Then the entry display_name is "Claude Opus 4"
    And the entry context_window is 200000
    And the entry supports_reasoning is true
    And the entry supports_vision is true

  Scenario: list_providers wires the cloud model catalog into the model selector
    Given the handle_impl.rs source for SessionManager::list_providers
    When the source is inspected
    Then it references crate::cloud_models::cloud_model_entries
    And it references crate::cloud_models::provider_has_credentials
    And it no longer leaves built-in providers with unconditionally empty models
