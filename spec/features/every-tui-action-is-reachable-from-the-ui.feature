@done
@tui
@rust
@code-quality
@TUI-132
Feature: Every TUI action is reachable from the UI
  """
  codelet-fspec-tui and codelet-tui. The cleanup half of the 2026-11-07 TUI audit (DA-9, ST-2, ST-4, PS-15; spec/attachments/TUI-132). Action variants with no production emitter are deleted from components/action_*.rs with their App::dispatch arms and every helper that only they reached; a live path that shared a helper keeps it. The never-shown DisconnectDialog, the App's DisconnectDialog key stage, Action::ManualReconnect, FspecBackend::request_manual_reconnect and the WebSocket supervisor's manual-reconnect Notify go; RPC-416's inline notice and auto-reconnect stay. mouse/toggle.rs, components/status_dialog.rs and try_dispatch_slash_debug go. rust/tui keeps only InputQueue. Provider Settings' footer_hint_for takes whether d can act. tests/tui132_every_action_variant_is_reachable.rs is a pure source scan; BUG-578 deleted the one exception (TestProviderConnection), so the scan has none. No RPC wire change.
  """

  Background: User Story
    As a fspec TUI maintainer
    I want to have every Action variant, dialog and helper in the TUI reachable from the UI and every footer hint match what its keys do
    So that no dead code or stub can mislead a reader or a user

  Scenario: Every Action variant has a production emitter and a production handler
    Given the Action variants declared in components/action_*.rs
    When the TUI source outside tests is scanned for each variant
    Then every variant is constructed somewhere outside tests
    And every variant is matched by a handler outside tests
    And there is no exception

  Scenario: The scaffolding nothing shows is gone
    Given the TUI source outside tests
    Then it declares no MouseTrackingToggle, no DisconnectDialog and no StatusDialog
    And no Action::ReEnableMouseTracking, Action::ManualReconnect or Action::RequestDeleteSession
    And no request_manual_reconnect and no try_dispatch_slash_debug

  Scenario: The resume view still deletes a session after its confirm
    Given the resume view lists the session "s-old"
    When the user presses d on "s-old"
    Then the delete confirm opens
    When the user confirms the delete
    Then persistence_delete_session is called once for "s-old"

  Scenario: The API key row offers delete only when the provider is configured
    Given Provider Settings lists the provider "anthropic" expanded with its API key row focused
    When "anthropic" is not configured
    Then the footer reads "Enter: edit · / filter · Tab: Switch to models · Esc: close"
    When "anthropic" is configured
    Then the footer reads "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"

  Scenario: A provider row advertises its arrow keys and delete
    Given Provider Settings lists the configured provider "anthropic" collapsed and focused
    Then the footer reads "Enter/→: expand · d: delete · / filter · Tab: Switch to models · Esc: close"
    When the user presses →
    Then the footer reads "Enter/←: collapse · d: delete · / filter · Tab: Switch to models · Esc: close"
    And for an unconfigured collapsed provider row the footer reads "Enter/→: expand · / filter · Tab: Switch to models · Esc: close"

  Scenario: The shared terminal crate exports only the input queue
    Given the codelet-tui crate
    Then its library exports InputQueue
    And it declares no StatusDisplay, create_event_stream, TuiEvent, restore_terminal or set_panic_hook
