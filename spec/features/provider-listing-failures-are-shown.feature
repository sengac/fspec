@done
@model-selector
@RPC-451
@rpc
@tui
@rust
Feature: Provider listing failures are shown
  """
  SessionManager::list_providers (sessions handle_impl_5.rs) logged a
  failed codelet_providers::custom::list_providers_info() (e.g. a corrupt
  ~/.fspec/providers/*.json) and returned Vec::new(); the handle and the
  tarpc list_providers returned Vec, so the failure could not reach the
  client, and the TUI ignored a failed list (if let Ok(..)), so the model
  selector opened empty with no explanation.
  Now list_providers returns Result<Vec<ProviderInfo>, String> on the
  SessionManagerHandle, the FspecService wire (same position in the frozen
  order) and the TUI backends; the RPC refuses without a session manager
  (BUG-453); the selector and startup show a failure as an error.
  Tests: rust/fspec-tui/tests/rpc451_provider_listing_failures_are_shown.rs.
  """

  Background: User Story
    As a user opening the model selector
    I want to see why the provider list could not be read
    So that a broken provider config is reported instead of an unexplained empty selector

  Scenario: An invalid provider config makes the provider listing fail
    Given a providers directory containing an invalid "broken.json"
    When the session manager lists providers
    Then the listing fails naming the invalid provider config

  Scenario: A failed provider listing is shown when the model selector opens
    Given a backend whose provider listing fails with "providers/broken.json is not valid JSON"
    When the model selector opens
    Then an error dialog shows "providers/broken.json is not valid JSON"
