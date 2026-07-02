@done
@ts-parity
@rust
@dispatch
@profiles
@provider-settings
@tui
@PROV-109
Feature: Transport + app dispatch wiring for profile save/delete
  """
  Mirrors the RPC-054 provider-credentials dispatch pattern (dispatch_provider_settings.rs) and the RPC-347 custom-model transport pattern (transport mod/embedded/websocket). New handlers live in dispatch_provider_settings.rs (spawn write -> on Ok send a follow-up list_provider_credentials whose ProviderCredentialsLoaded fold reloads the openai profile slice). Transport trait gains save_profile/delete_profile default no-op bodies; embedded + websocket delegate to FspecService.save_profile/delete_profile (already wired in PROV-108). Tests use the MockBackend with new save_profile/delete_profile counters + error scripting, driven through App::dispatch like provider_settings_dispatch_rpc054.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The FspecBackend transport trait exposes save_profile(provider_id, profile_name, definition) and delete_profile(provider_id, profile_name) with default no-op Ok bodies; the embedded and websocket transports override them to delegate to FspecService.save_profile / delete_profile
  #   2. Three Action variants are added: SaveProfile { provider_id, profile_name, definition }, DeleteProfile { provider_id, profile_name }, and ConfirmDeleteProfile { provider_id, profile_name }
  #   3. Dispatching SaveProfile spawns backend.save_profile(...) and on Ok dispatches a follow-up backend.list_provider_credentials() refresh whose ProviderCredentialsLoaded fold reloads the openai profile slice from fspec-config.json
  #   4. Dispatching DeleteProfile spawns backend.delete_profile(...) followed by the same list_provider_credentials refresh; ConfirmDeleteProfile routes through the identical delete handler so the destructive call only fires after explicit confirmation
  #   5. Backend save/delete errors are folded into the view as an Action::ProviderSettingsStatus('✗ ...') inline status; they never panic and never leak the RPC method name or raw error into the AgentView scrollback
  #
  # EXAMPLES:
  #   1. Saving a new openai profile dispatches SaveProfile, awaits backend.save_profile once, sets a '✓ ... profile saved' status, and triggers a list_provider_credentials refresh
  #   2. Editing an existing profile dispatches SaveProfile with the new connection settings and the openai profile row repaints after the refresh
  #   3. Confirming a profile deletion dispatches ConfirmDeleteProfile, awaits backend.delete_profile once, sets a '✓ ... profile deleted' status, and the deleted profile is gone after the list refresh
  #   4. A failed save (backend returns Err) shows a '✗ ...' inline status, does not panic, and leaks no RPC method name into the agent scrollback
  #   5. ConfirmDeleteProfile and the raw DeleteProfile both reach backend.delete_profile through the same handler, so a confirmed delete and a direct delete behave identically
  #
  # ========================================
  Background: User Story
    As a TUI user managing local-server openai profiles
    I want to have my profile create/edit/delete actions routed through the transport to the PROV-108 backend and see the list refresh
    So that my profile changes persist and the provider-settings view repaints with the new state without a restart

  Scenario: Saving a new openai profile writes and refreshes the list
    Given the provider settings view is open with a MockBackend
    When the user dispatches SaveProfile for openai profile "work-vllm"
    Then backend.save_profile is awaited exactly once
    And the captured save carries provider "openai" and profile "work-vllm"
    And the inline status reports the profile was saved
    And a follow-up backend.list_provider_credentials refresh is dispatched

  Scenario: Editing an existing profile dispatches SaveProfile and repaints the row
    Given the provider settings view is open with a MockBackend
    When the user dispatches SaveProfile for openai profile "home" with baseUrl "http://localhost:9999"
    Then backend.save_profile is awaited exactly once
    And the captured save definition carries baseUrl "http://localhost:9999"
    And a follow-up backend.list_provider_credentials refresh is dispatched

  Scenario: Confirming a profile deletion removes it and refreshes the list
    Given the provider settings view is open with a MockBackend
    When the user dispatches ConfirmDeleteProfile for openai profile "work-vllm"
    Then backend.delete_profile is awaited exactly once
    And the captured delete carries provider "openai" and profile "work-vllm"
    And the inline status reports the profile was deleted
    And a follow-up backend.list_provider_credentials refresh is dispatched

  Scenario: A failed profile save surfaces an inline error without leaking
    Given the provider settings view is open with a MockBackend that fails save_profile with "write failed"
    When the user dispatches SaveProfile for openai profile "work-vllm"
    Then the inline status surfaces the failure with a "✗" marker
    And no panic occurs
    And no RPC method name leaks into the agent scrollback

  Scenario: ConfirmDeleteProfile and DeleteProfile route through the same handler
    Given the provider settings view is open with a MockBackend
    When the user dispatches DeleteProfile for openai profile "a"
    And the user dispatches ConfirmDeleteProfile for openai profile "b"
    Then backend.delete_profile is awaited exactly twice
    And both deletes target provider "openai"
