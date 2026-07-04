@done
@RPC-073
@providers
@model-selection
@bug
Feature: RPC-073 List Providers Wiring
  """
  Bug 3: SessionManager::list_providers in
  codelet/sessions/src/handle_impl.rs:709-715 unconditionally returns
  Vec::new(), so the model selector dialog opens empty.

  Fix: delegate to codelet_providers::custom::list_providers_info()
  (already used by NAPI binding session_bindings.rs:3469 and by the
  sibling handle_impl methods at lines 798 and 944). Map the 9-field
  codelet_providers::custom::ProviderInfo into the 3-field
  codelet_rpc_types::ProviderInfo: key = name, display = display_name OR name,
  models = list of ModelEntry with supports_thinking → supports_reasoning,
  is_custom propagated from parent into every child ModelEntry, usize → u32
  saturating cast.

  On Err: tracing::error log + Vec::new() — no panic, no UI failure surface
  (model dialog status area handles user-visible failures inline).

  PROV-127 (drop-empty cloud sections, TS parity): built-in cloud providers
  are populated from the models.dev registry gated on configured credentials
  (codelet/sessions/src/cloud_models.rs). After the cloud map is built,
  handle_impl.rs::list_providers applies
  profile_sections::retain_populated_cloud_sections() so a built-in cloud
  provider appears ONLY when it exposes at least one model (credentialed +
  present in the cache). Zero-model cloud sections are dropped rather than
  rendered as dead "Provider (0 models)" headers — mirroring TS
  cloudSectionBuilder.ts (filter(s => s.hasCredentials)) +
  modelInitializationService.ts (filter(s => s.models.length > 0)). Local
  server profiles are appended AFTER the filter and are never dropped for
  having zero models (RPC-338/MODEL-004).

  Reference: spec/attachments/RPC-073/research-bug3-list-providers-ts-vs-rust.md
  Reference: spec/attachments/PROV-127/spec.md
  """

  Background: User Story
    As a fspec user driving the Rust binary
    I want the model selector dialog to list all configured providers from my ~/.fspec/fspec-config.json
    So that I can switch models without falling back to the TS Ink frontend

  Scenario: list_providers returns built-in cloud providers only when they have at least one model
    Given a SessionManager is constructed with a seeded models.dev cache and credentials for openai, anthropic and gemini
    When the test calls handle.list_providers()
    Then every returned cloud ProviderInfo entry has at least one model
    Then the entries include the credentialed built-in provider keys 'openai' and 'anthropic'
    Then zero-model built-in cloud providers such as 'codex' and 'zai' are dropped from the result

  Scenario: list_providers entries have populated key and display_name fields and a non-empty models Vec
    Given list_providers has been called with seeded credentials and returned a non-empty Vec
    When the test inspects the 'anthropic' ProviderInfo entry
    Then the entry has a non-empty 'key' field matching the canonical provider slug 'anthropic'
    Then the entry has a non-empty 'display_name' field
    Then the entry has a 'models' field of type Vec containing at least one model

  Scenario: list_providers maps codelet_providers::custom::ProviderInfo into codelet_rpc_types::ProviderInfo with the correct field mapping
    Given a seeded models.dev cache and credentials populate the built-in 'openai' provider with a reasoning-capable model whose supports_thinking=true
    When the trait override list_providers maps the value into a codelet_rpc_types::ProviderInfo
    Then the resulting codelet_rpc_types::ProviderInfo has key='openai', a non-empty display, and a child ModelEntry with supports_reasoning=true and is_custom=false
    Then context_window and max_output_tokens are converted from usize to u32 with saturating cast

  Scenario: list_providers degrades gracefully to Vec::new() and logs via tracing::error when list_providers_info returns Err
    Given an environment is set up such that list_providers_info returns Err (e.g. corrupt ~/.fspec/providers/foo.json)
    When the test calls handle.list_providers()
    Then the call returns Vec::new() and does not panic
    Then a tracing::error event with target 'handle_impl' and the underlying error is emitted

  Scenario: Source-shape regression: handle_impl.rs list_providers body calls list_providers_info and no longer returns the empty Vec::new() stub
    Given the file codelet/sessions/src/handle_impl.rs
    When the test reads the source bytes and extracts the body of fn list_providers
    Then the body contains the substring 'list_providers_info'
    Then the body does not match the deprecated stub pattern of bare 'Vec::new()' as the sole expression
