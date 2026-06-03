@done
@critical
@RPC-054
@rust
@tui
@agent-view
@provider-settings
@view-isolation
Feature: ProviderSettingsView — full-screen mode-view keyboard handling

  """
  Isolated view-layer behaviour of ProviderSettingsView
  (codelet/fspec-tui/src/views/provider_settings/mod.rs).
  Drives the synchronous handle_key surface only — no App, no backend.
  The view follows the canonical full-screen mode-view pattern from
  RPC-026's ResumeSessionView: Clear.render(area, buf) first, then a
  Layout::default().direction(Vertical).constraints([Length(1) /* title */,
  Length(1) /* separator */, Min(0) /* body */, Length(1) /* footer */])
  split of the FULL area Rect.
  The view's mode field is ProviderSettingsMode { List, Detail { provider_id,
  sub: DetailSub { Summary { last_status }, EditApiKey { draft }, OAuthNotice } } }.
  Destructive 'd' on List opens a ConfirmDialog before any backend call fires.
  RPC + dispatch + transport plumbing lives in sibling feature files
  (rpc054-provider-settings-dispatch.feature, rpc054-provider-settings-cross-transport-parity.feature,
  rpc054-provider-settings-source-shape.feature).
  """

  Background: User Story
    As a developer working on the Rust ratatui TUI frontend
    I want the ProviderSettingsView to respond to keyboard input correctly in isolation
    So that Enter / Esc / t / r / d / typing produce the right mode transitions and emitted Actions before the dispatcher even sees them — and the view paints with the same Clear + Layout(Length(1), Length(1), Min(0), Length(1)) contract used by ResumeSessionView

  @list-mode
  @navigation
  Scenario: Open view with no providers shows the centred placeholder
    Given the ProviderSettingsView is in List mode with an empty providers list
    When the view is rendered into a 80x24 area
    Then the body row shows the centred placeholder "(no providers configured)"
    And pressing Enter is a no-op
    And pressing "d" is a no-op
    And pressing Esc emits ProviderSettingsEvent::Close

  @list-mode
  @navigation
  @list-mode
  @scrolling
  Scenario: ↓ scrolls the window when the selection moves past the visible rows
    Given the ProviderSettingsView is in List mode with 40 providers
    And the render area body height is 18 rows
    When the user presses ↓ twenty times
    Then selected_index equals 20
    And scroll_offset has advanced so row 20 falls inside the visible window
    And the rendered list shows the row at index 20 (verified via ensure_visible math)

  @list-mode
  @scrolling
  @list-mode
  @enter-detail
  Scenario: Enter on an api_key row transitions to Detail::Summary
    Given the ProviderSettingsView is in List mode with "anthropic" focused
    And the anthropic row's credential_type is "api_key"
    When the user presses Enter
    Then the view's mode is Detail { provider_id: "anthropic", sub: Summary { last_status: None } }
    And the footer hint reads "r: refresh models · Esc: back" (RPC-154 dropped `t: test ·` for TS parity)

  @list-mode
  @enter-oauth
  Scenario: Enter on an oauth row transitions directly to Detail::OAuthNotice
    Given the ProviderSettingsView is in List mode with "codex" focused
    And the codex row's credential_type is "oauth"
    When the user presses Enter
    Then the view's mode is Detail { provider_id: "codex", sub: OAuthNotice }
    And the body shows the read-only text "codex uses OAuth which is not yet supported in the Rust frontend"
    And the footer hint reads "Esc Back"

  @detail
  @summary
  @rpc-154
  Scenario: t inside Detail::Summary is silently ignored (RPC-154 TS parity)
    Given the ProviderSettingsView is in Detail { provider_id: "openai", sub: Summary { last_status: None } }
    When the user presses "t"
    Then the emitted ProviderSettingsEvent is Consumed
    And no Action::TestProviderConnection is emitted
    And view.mode remains Detail::Summary with last_status: None preserved

  @detail
  @summary
  Scenario: r inside Detail::Summary emits RefreshProviderModels
    Given the ProviderSettingsView is in Detail { provider_id: "openai", sub: Summary { last_status: None } }
    When the user presses "r"
    Then the emitted ProviderSettingsEvent is Emit(Action::RefreshProviderModels("openai"))
    And the last_status is updated to RefreshingModels
    And the body shows "Refreshing models…"

  @detail
  @summary
  Scenario: TestOk last_status renders as green "✓ ok (Xms)"
    Given the ProviderSettingsView is in Detail::Summary for "openai" with last_status = TestOk { latency_ms: 42 }
    When the view is rendered
    Then the body contains "✓ openai ok (42ms)" in green

  @detail
  @summary
  Scenario: TestErr last_status renders as red "✗ <error>"
    Given the ProviderSettingsView is in Detail::Summary for "openai" with last_status = TestErr { error: "unreachable: dns resolution failed" }
    When the view is rendered
    Then the body contains "✗ unreachable: dns resolution failed" in red

  @detail
  @enter-edit
  Scenario: Enter inside Detail::Summary on api_key provider opens EditApiKey
    Given the ProviderSettingsView is in Detail::Summary for "anthropic" (credential_type api_key)
    When the user presses Enter
    Then the view's mode is Detail { provider_id: "anthropic", sub: EditApiKey { draft: "" } }
    And the body shows "Key: " followed by an empty masked input
    And the footer hint reads "Enter Save | Esc Cancel"

  @detail
  @edit
  Scenario: Typing characters in EditApiKey grows the draft
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    When the user types "sk-test-1"
    Then the draft equals "sk-test-1"
    And the rendered Key line shows 9 masked characters ("•" × 9)

  @detail
  @edit
  Scenario: Backspace removes the last draft character
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "abc"
    When the user presses Backspace
    Then the draft equals "ab"

  @detail
  @edit
  @save
  Scenario: Enter on EditApiKey with non-empty draft emits SaveProviderCredentials
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-test-1"
    When the user presses Enter
    Then the emitted ProviderSettingsEvent is Emit(Action::SaveProviderCredentials { provider_id: "anthropic", api_key: "sk-test-1" })

  @list-mode
  @delete
  @confirm-dialog
  Scenario: d on a configured row opens the ConfirmDialog
    Given the ProviderSettingsView is in List mode with "anthropic" focused
    And the anthropic row's configured field is true
    When the user presses "d"
    Then delete_confirm is Some(ConfirmDialog) with title "Delete credentials?", body "Delete credentials for anthropic?", primary "Delete", cancel "Cancel"
    And NO ProviderSettingsEvent::Emit is dispatched
    And backend.delete_provider_credentials is NEVER called

  @list-mode
  @delete
  @confirm-dialog
  Scenario: d on an unconfigured row is a no-op
    Given the ProviderSettingsView is in List mode with "anthropic" focused
    And the anthropic row's configured field is false
    When the user presses "d"
    Then delete_confirm is None
    And no ProviderSettingsEvent::Emit is dispatched

  @list-mode
  @delete
  @confirm-dialog
  Scenario: Enter on ConfirmDialog Primary emits ConfirmDeleteProviderCredentials
    Given the ProviderSettingsView's delete_confirm dialog is open for "anthropic" with Primary focused
    When the user presses Enter
    Then the emitted ProviderSettingsEvent is Emit(Action::ConfirmDeleteProviderCredentials("anthropic"))
    And delete_confirm is None
    And the view returns to List mode

  @list-mode
  @delete
  @confirm-dialog
  Scenario: Esc on ConfirmDialog cancels without emitting
    Given the ProviderSettingsView's delete_confirm dialog is open for "anthropic"
    When the user presses Esc
    Then delete_confirm is None
    And no ProviderSettingsEvent::Emit is dispatched
    And the view returns to List mode

  @esc-hierarchy
  Scenario: Esc in List mode emits ProviderSettingsEvent::Close
    Given the ProviderSettingsView is in List mode
    And no ConfirmDialog is open
    When the user presses Esc
    Then the emitted ProviderSettingsEvent is Close

  @esc-hierarchy
  Scenario: Esc in Detail::Summary returns to List mode
    Given the ProviderSettingsView is in Detail::Summary for "openai" with selected_index = 5
    When the user presses Esc
    Then the view's mode is List
    And selected_index is still 5 (preserved)
    And no ProviderSettingsEvent::Close is emitted

  @esc-hierarchy
  Scenario: Esc in Detail::OAuthNotice returns to List mode
    Given the ProviderSettingsView is in Detail::OAuthNotice for "codex"
    When the user presses Esc
    Then the view's mode is List

  @title
  @rendering
  @footer
  @rendering
  Scenario: Footer hint in List mode
    Given the ProviderSettingsView is in List mode
    When the view is rendered
    Then the footer row contains "Enter Select"
    And the footer row contains "↑↓ Navigate"
    And the footer row contains "D Delete"
    And the footer row contains "Esc Cancel"

  @footer
  @rendering
  Scenario: Footer hint in Detail::Summary mode
    Given the ProviderSettingsView is in Detail::Summary for "openai"
    When the view is rendered
    Then the footer row contains "t Test"
    And the footer row contains "r Refresh Models"
    And the footer row contains "Esc Back"

  @footer
  @rendering
  Scenario: Footer hint in Detail::EditApiKey mode
    Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic"
    When the view is rendered
    Then the footer row contains "Enter Save"
    And the footer row contains "Esc Cancel"

  @footer
  @rendering
  Scenario: Footer hint in Detail::OAuthNotice mode
    Given the ProviderSettingsView is in Detail::OAuthNotice for "codex"
    When the view is rendered
    Then the footer row contains "Esc Back"

  @list-mode
  @filter
  Scenario: Pressing "/" in List mode enters filter mode
    Given the ProviderSettingsView is in List mode with providers ["anthropic", "openai", "codex"]
    And filter_mode is false
    And filter is ""
    When the user presses "/"
    Then filter_mode is true
    And filter is still ""
    And no "/" character was inserted into any draft

  @list-mode
  @filter
  Scenario: Typing characters in filter mode appends to filter string
    Given the ProviderSettingsView is in List mode with filter_mode = true and filter = ""
    When the user types "an"
    Then filter equals "an"
    And the body row above the list shows "Filter: an"
    And the provider list shows only providers whose id or name contains "an" (case-insensitive)

  @list-mode
  @filter
  Scenario: Backspace in filter mode removes the last character
    Given the ProviderSettingsView is in List mode with filter_mode = true and filter = "ant"
    When the user presses Backspace
    Then filter equals "an"

  @list-mode
  @filter
  Scenario: Enter in filter mode exits filter mode but keeps the filter string
    Given the ProviderSettingsView is in List mode with filter_mode = true and filter = "anth"
    When the user presses Enter
    Then filter_mode is false
    And filter equals "anth" (preserved)
    And the visible providers are still filtered by "anth"

  @list-mode
  @filter
  Scenario: Esc in filter mode clears the filter string AND exits filter mode
    Given the ProviderSettingsView is in List mode with filter_mode = true and filter = "xyz"
    When the user presses Esc
    Then filter_mode is false
    And filter equals ""
    And the provider list is fully restored
    And no ProviderSettingsEvent::Close is emitted (Esc does NOT close the view in this case)

  @esc-hierarchy
  @filter
  Scenario: Esc in List mode with a non-empty filter clears filter first (does not close view)
    Given the ProviderSettingsView is in List mode with filter_mode = false and filter = "ant"
    When the user presses Esc
    Then filter equals ""
    And no ProviderSettingsEvent::Close is emitted
    And the view's mode is still List

  @esc-hierarchy
  @filter
  Scenario: Esc in List mode with empty filter emits Close (second-Esc cascade)
    Given the ProviderSettingsView is in List mode with filter_mode = false and filter = ""
    When the user presses Esc
    Then the emitted ProviderSettingsEvent is Close

  @list-mode
  @filter
  Scenario: Filter substring is matched against both id and name (case-insensitive)
    Given the ProviderSettingsView is in List mode with providers [{id: "github-copilot", name: "GitHub Copilot"}, {id: "anthropic", name: "Anthropic"}]
    And filter_mode = false and filter = "COPILOT"
    Then the visible providers list contains "github-copilot"
    And does NOT contain "anthropic"
