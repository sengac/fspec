@done
@session-management
@provider-settings
@agent-view
@rpc
@tui
@rust
@RPC-054
Feature: /provider provider-credentials source-shape regression

  """
  Source-shape assertions for the new RPC-054 surface — guards the public symbols + module layout that the rest of the test suite depends on. Scans the actual source files at compile time so a future refactor that renames or relocates ProviderCredentialInfo / ProviderCredentialInput / TestConnectionResult / ProviderSettingsView / ProviderSettingsMode breaks loudly.
  """

  Background: User Story
    As a Rust ratatui frontend maintainer
    I want a source-shape regression test for the RPC-054 surface
    So that refactors that quietly relocate the new wire types or view module fail the test suite instead of silently rotting the spec

  Scenario: New wire types live in codelet/rpc-types/src/lib.rs
    Given the file codelet/rpc-types/src/lib.rs is compiled
    Then it declares public types ProviderCredentialInfo, ProviderCredentialInput, and TestConnectionResult
    And each type has Serialize + Deserialize derives
    And ProviderCredentialInfo is gated by #[cfg_attr(feature = "napi", napi_derive::napi(object))]

  Scenario: ProviderSettingsView module exists with the expected source shape
    Given the file codelet/fspec-tui/src/views/provider_settings/mod.rs exists
    When the file is compiled as part of codelet-fspec-tui
    Then it declares pub struct ProviderSettingsView
    And it declares an enum or state describing list-mode and edit-api-key-mode
    And codelet/fspec-tui/src/views/mod.rs declares pub mod provider_settings
