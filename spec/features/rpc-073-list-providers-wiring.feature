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

  Reference: spec/attachments/RPC-073/research-bug3-list-providers-ts-vs-rust.md
  """

  Background: User Story
    As a fspec user driving the Rust binary
    I want the model selector dialog to list all configured providers from my ~/.fspec/fspec-config.json
    So that I can switch models without falling back to the TS Ink frontend

  Scenario: list_providers returns all built-in providers when no custom providers are configured
    Given a SessionManager is constructed in an environment with no ~/.fspec/providers/ custom configs
    When the test calls handle.list_providers()
    Then the returned Vec<ProviderInfo> contains at least 6 entries
    Then the entries include the built-in provider keys 'claude', 'openai', 'gemini', 'zai', 'codex', and 'github-copilot'

  Scenario: list_providers entries have populated key and display_name fields and a non-null models Vec
    Given list_providers has been called and returned a non-empty Vec
    When the test inspects the 'claude' ProviderInfo entry
    Then the entry has a non-empty 'key' field matching the provider slug 'claude'
    Then the entry has a non-empty 'display_name' field
    Then the entry has a 'models' field of type Vec (which may be empty for built-in providers but is present)

  Scenario: list_providers maps codelet_providers::custom::ProviderInfo into codelet_rpc_types::ProviderInfo with the correct field mapping
    Given a codelet_providers::custom::ProviderInfo with name='openai', display_name=Some('OpenAI'), is_custom=false, and a child model whose supports_thinking=true
    When the trait override list_providers maps the value into a codelet_rpc_types::ProviderInfo
    Then the resulting codelet_rpc_types::ProviderInfo has key='openai', display='OpenAI', and the child ModelEntry has supports_reasoning=true and is_custom=false
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
