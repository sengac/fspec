@done
@RPC-054
@providers
@config-management
Feature: Provider Credential Persistence
  """
  Architecture: ProviderSettingsView ports the full-screen mode-view pattern from RPC-026's ResumeSessionView. Crate layout: rust/fspec-tui/src/views/provider_settings/{mod.rs (orchestrator: ProviderSettingsView struct, key dispatcher, render outer shell, ProviderSettingsMode enum), list.rs (render_list), detail.rs (render_detail), status_text.rs (DetailStatus → coloured text)}. Each file < 300 LoC per RPC-002 rule [10]
  Render shape: render(area, buf) first calls Clear.render(area, buf), then splits the FULL area with Layout::default().direction(Vertical).constraints([Length(1) /* title */, Length(1) /* separator */, Min(0) /* body */, Length(1) /* footer */]).split(area); title row paints via render_title_with_count('Provider Settings', configured_count); footer row paints via render_footer_hint(mode_aware_hint). Body content branches on ProviderSettingsMode (List vs Detail). ConfirmDialog renders LAST as an overlay on top of the body when delete_confirm is Some — same pattern as ResumeSessionView
  Slash command registry surgery: rust/fspec-tui/src/views/agent/slash_commands.rs MUST (a) remove the SlashCommandAction::Providers variant from the enum; (b) remove the corresponding name() match arm; (c) remove the SLASH_COMMANDS entry { action: Providers, description: 'Open provider settings' }; (d) leave the SlashCommandAction::Provider variant and { action: Provider, description: 'Configure API providers' } entry intact. Dispatch: rust/fspec-tui/src/app/dispatch_rpc020.rs changes the `SlashCommandAction::Provider | SlashCommandAction::Providers =>` arm to `SlashCommandAction::Provider =>` and updates the comment to drop the alias reference
  Action enum additions in components/mod.rs: existing variants (OpenProviderSettingsView, CloseProviderSettingsView, ProviderCredentialsLoaded, SaveProviderCredentials { provider_id, api_key }, TestProviderConnection(provider_id), ProviderTestComplete { provider_id, result }, RefreshProviderModels(provider_id), ProviderModelsRefreshed { provider_id, model_count }, DeleteProviderCredentials(provider_id), ProviderSettingsStatus(status)) all stay — they map to the same backend RPCs. New action: Action::ConfirmDeleteProviderCredentials(provider_id) — emitted when the user accepts the ConfirmDialog Primary, replaces the previous direct backend call from list view's 'd' handler
  Reuse contract: ConfirmDialog from rust/fspec-tui/src/views/agent/confirm_dialog.rs (the same component RPC-026 uses for delete-session) is reused verbatim — no new dialog component. Dialog id constant DELETE_PROVIDER_CREDS_DIALOG_ID for idempotent push. ConfirmDialogOutcome::Primary → emit Action::ConfirmDeleteProviderCredentials(provider_id); ConfirmDialogOutcome::Secondary | Cancel → close dialog with no action emitted
  Test surgery: (a) rust/fspec-tui/tests/provider_settings_view_rpc054.rs — rewrite assertions to target ProviderSettingsMode (List vs Detail) instead of the old ProviderSettingsMode { List, EditApiKey }; assert ConfirmDialog flow for 'd' key; (b) rust/fspec-tui/tests/provider_settings_dispatch_rpc054.rs — keep Action::Open/Close flow; (c) rust/fspec-tui/tests/source_shape_rpc054.rs — update import-list assertion to require Clear/ensure_visible/wrap_index/ConfirmDialog/render_title_with_count/render_footer_hint and FORBID Borders/Block; (d) tests/rpc054_cross_transport_parity.rs — UNCHANGED (backend surface didn't change); (e) tests/behaviour_parity_rpc065.rs — delete slash_providers_alias_activates_provider_settings_view; (f) tests/slash_debug_rpc055.rs — audit for any /providers references
  Filter mode lives as a sub-state on the List variant: ProviderSettingsView holds `filter: String` and `filter_mode: bool` fields. The list mode key dispatcher checks `filter_mode` first — when true, it routes keys to a filter sub-handler (printable→push, Backspace→pop, Enter→exit filter_mode keeping filter, Esc→clear filter + exit filter_mode). When `filter_mode` is false, list mode runs normally but Esc still has a precedence: non-empty filter → clear filter; empty filter → emit Close. This matches TS hooks/useProviderSettingsState.ts (isFilterMode, filter) + inputHandlers/filterModeHandler.ts + listModeHandler.ts:47-67.
  New write functions live in rust/sessions/src/credentials/ (store.rs or a new writer.rs kept <300 LoC): save_credential_with_dir/delete_credential_with_dir + fspec_user_dir-resolving conveniences. handle_impl.rs set/delete_provider_credentials delegate to them. Reuse existing CredentialsFile/ProviderCredential types (already camelCase) and credentials_reload().
  Tests run offline by pointing FSPEC_USER_DIR at a temp dir; assert JSON contents, in-place update, delete, empty-providers-on-last-delete, unix 0600/0700 perms, and get_stored_api_key_with_dir round-trip. No network, no $HOME mutation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The non-existent slash command /providers MUST be removed: delete SlashCommandAction::Providers variant, its name() arm, its SLASH_COMMANDS registry entry, and the | SlashCommandAction::Providers arm in dispatch_rpc020.rs — the TypeScript reference src/tui/utils/slashCommands.ts defines exactly one entry (name: 'provider'), so the Rust enum mirrors it 1:1 with no alias
  #   2. ProviderSettingsView::render(area, buf) follows the canonical full-screen mode-view pattern from RPC-026's ResumeSessionView: first statement is Clear.render(area, buf); body splits the FULL area Rect into Layout::default().direction(Vertical).constraints([Length(1), Length(1), Min(0), Length(1)]) for title / separator / body / footer; NO outer Block with borders, NO two-pane split
  #   3. The view reuses render_title_with_count and render_footer_hint from rust/fspec-tui/src/views/agent/mode_view_render.rs (the same helpers ResumeSessionView uses). Title text is 'Provider Settings (N configured)' where N is the count of rows with configured == true
  #   4. Scroll mechanics use ensure_visible(scroll_offset, selected_index, visible_rows, len) and wrap_index(selected, delta, len) from crate::components::scroll_viewport — identical helpers to ResumeSessionView. ↑/↓ wrap around; PageUp/PageDown jump by visible_rows; Home/End jump to extremes
  #   5. ProviderSettingsMode is an enum with two variants: List (default — scrollable provider list) and Detail { provider_id, sub }. DetailSub has three variants: Summary { last_status }, EditApiKey { draft }, OAuthNotice. The list view does NOT contain a status string; status text lives only in Detail::Summary
  #   6. From List mode, Enter on a row transitions mode to Detail { provider_id, sub: Summary }. For api_key rows, a second Enter inside Detail::Summary transitions to Detail::EditApiKey. For oauth rows (codex, github-copilot), the first Enter from List transitions directly to Detail::OAuthNotice (a read-only explanation that OAuth is deferred to the TS frontend)
  #   7. The t (test connection) and r (refresh models) keys are NOT bound on the List mode — they are bound only inside Detail::Summary. From Detail::Summary, t fires Action::TestProviderConnection(provider_id) and r fires Action::RefreshProviderModels(provider_id). Both update last_status which renders inside the body Rect
  #   8. The d / D (delete credentials) key on List mode opens a ConfirmDialog scoped to the view (mirrors ResumeSessionView's delete-session flow) — title 'Delete credentials?', body 'Delete credentials for {provider_id}?', primary label 'Delete', cancel label 'Cancel'. Backend.delete_provider_credentials is only called after the user presses Enter on the dialog's Primary. d / D on configured == false rows is a no-op
  #   9. Esc behaviour is hierarchical: Esc in ConfirmDialog closes the dialog (no backend call); Esc in any Detail::* sub-view returns to List mode (no backend call); Esc in List mode emits ProviderSettingsEvent::Close which Navigator translates to Action::CloseProviderSettingsView, returning to AgentView
  #   10. From Detail::EditApiKey, Enter with a non-empty draft fires Action::SaveProviderCredentials { provider_id, api_key } and transitions back to Detail::Summary with last_status = SavingCredentials → CredentialsSaved (or Error). Enter with an empty draft surfaces inline 'API key cannot be empty' and stays in EditApiKey mode without emitting
  #   11. Footer hints are mode-aware: List → 'Enter Select | ↑↓ Navigate | D Delete | Esc Cancel'; Detail::Summary → 't Test | r Refresh Models | Esc Back'; Detail::EditApiKey → 'Enter Save | Esc Cancel'; Detail::OAuthNotice → 'Esc Back'. These strings paint via render_footer_hint into the footer row
  #   12. The backend trait surface (list_provider_credentials, get_provider_credential, set_provider_credentials, delete_provider_credentials, test_provider_connection, refresh_models_cache) on SessionManagerHandle / FspecService / FspecBackend is UNCHANGED from the first pass — only the view layer and slash-command registry change. ProviderCredentialInfo / ProviderCredentialInput / TestConnectionResult wire types stay as-is
  #   13. Backend errors from list / get / set / delete / test / refresh continue to be silently logged via tracing — no panic, no scrollback notice. User-visible failures surface as DetailStatus::Error { message } inside the body Rect of Detail::Summary (e.g. '✗ unreachable: dns resolution failed')
  #   14. Source-shape regression test asserts: views/provider_settings/mod.rs exists; it imports Clear, ensure_visible, wrap_index, ConfirmDialog, render_title_with_count, render_footer_hint; it does NOT import Borders or Block (no bordered Block wrapping the view); SlashCommandAction::Providers variant does NOT exist; the SLASH_COMMANDS registry contains exactly one entry whose action is SlashCommandAction::Provider
  #   15. File-size discipline (RPC-002 rule [10]) is preserved: every new file under rust/fspec-tui/src/views/provider_settings/ stays under 300 LoC. If the orchestrator mod.rs grows past 300 LoC, split into provider_settings/{mod.rs, list.rs, detail.rs, status_text.rs} — same shape as views/agent/{resume_session_view, search_history_view, mode_view_render}
  #   16. Cross-transport parity (embedded vs WebSocket) tests stay UNCHANGED — they exercise list_provider_credentials, set_provider_credentials, test_provider_connection, refresh_models_cache via both backends against StubSessionManagerHandle. The view-layer rewrite must not regress this surface; the existing rpc054_cross_transport_parity.rs assertions about counter increments still apply
  #   17. List view supports `/` to enter filter mode; printable chars append to a filter string, Backspace removes, Enter exits filter mode (preserves filter), Esc clears filter+exits filter mode. Filter matches provider.id OR provider.name case-insensitively. Mirrors TS src/tui/inputHandlers/filterModeHandler.ts.
  #   18. Esc in List mode has a two-step cascade: if filter is non-empty, first Esc clears the filter and does NOT close the view; only the second Esc (with empty filter) emits ProviderSettingsEvent::Close. Mirrors TS listModeHandler.ts:47-53.
  #   19. set_provider_credentials with kind=api_key MUST persist the key to <FSPEC_USER_DIR>/credentials/credentials.json (read-modify-write), mirroring TS saveCredential — not a no-op
  #   20. Saving a credential writes ProviderCredential { apiKey, lastUpdated: ISO-8601 } under providers[providerId], preserving version and other providers' entries
  #   21. A missing or empty credentials.json is treated as { version: 1, providers: {} }; the credentials directory is created on demand
  #   22. On unix the credentials dir is chmod 0700 and the file is chmod 0600; chmod failures are swallowed (matches TS); on non-unix the write still succeeds
  #   23. delete_provider_credentials MUST remove providers[providerId] from credentials.json and write the file back, mirroring TS deleteCredential — not a no-op
  #   24. After any save or delete, credentials_reload() is invoked so the in-memory CredentialStore cache and process env vars reflect the change without restart (mirrors TS credentialsReload)
  #   25. The raw API key is never written to logs/tracing; only the provider id and credential kind may be logged
  #   26. Empty api_key still returns Err (existing validation kept); oauth and custom credential kinds remain non-persistent in this card (OAuth write path is a separate follow-up)
  #
  # EXAMPLES:
  #   1. User types /provider in the AgentView; Navigator flips active_view to ViewMode::ProviderSettings; the screen clears (Clear.render(area, buf)) and shows title row 'Provider Settings (3 configured)', separator row, scrollable provider list, footer 'Enter Select | ↑↓ Navigate | D Delete | Esc Cancel'
  #   2. User types /providers (plural) in the AgentView; the input is treated as ordinary text because no /providers entry exists in the slash command palette filter — the singular /provider remains the only match for the 'prov' prefix
  #   3. User opens the slash command palette by pressing /; the palette lists the slash commands and the only provider-related entry is /provider (singular) with description 'Configure API providers' — there is NO /providers entry anywhere in the palette
  #   4. User opens /provider with 20 providers (3 built-in configured, 5 unconfigured, 12 custom); the body shows the first ~18 rows depending on terminal height; pressing ↓ moves the highlight; reaching the last visible row + pressing ↓ scrolls the window so the next row appears at the bottom — identical to /resume behaviour
  #   5. User highlights the 'anthropic' row (an api_key-type configured provider) and presses Enter; the body repaints to show the per-provider summary: display name, provider_id, credential type, model count, and a status area; the footer hint changes to 't Test | r Refresh Models | Esc Back'
  #   6. User is inside the anthropic detail view (Summary); pressing Enter opens the inline API key edit form; the status area shows 'Key: ' followed by a masked input; the footer changes to 'Enter Save | Esc Cancel'; typing characters fills the masked input
  #   7. User types 'sk-test-123' into the edit form and presses Enter; the form closes; the view returns to Detail::Summary with the status line showing '✓ credentials saved'; pressing Esc returns to List mode and the anthropic row now shows the ✓ configured indicator
  #   8. User is inside the openai detail view (Summary); pressing 't' shows 'Testing…' in the status line; ~42ms later the line updates to '✓ ok (42ms)'; pressing 't' again re-runs the test and the latest latency replaces the old one
  #   9. User is inside the openai detail view; pressing 't' returns a backend error; the status line shows '✗ unreachable: dns resolution failed' in red; pressing Esc returns to List mode and the openai row's configured indicator is unchanged
  #   10. User is inside the openai detail view (Summary); pressing 'r' shows 'Refreshing models…'; on completion the line updates to '✓ models refreshed (8)'; pressing Esc back to List shows the row's model count updated from 4 to 8
  #   11. User is in List mode with the anthropic row (configured) focused; pressing 'd' opens a ConfirmDialog overlaid on the body — title 'Delete credentials?', body 'Delete credentials for anthropic?', primary 'Delete', cancel 'Cancel'; backend.delete_provider_credentials has NOT yet been called
  #   12. User is on the ConfirmDialog with Primary focused; pressing Enter fires backend.delete_provider_credentials('anthropic'); the dialog closes; the list refreshes; the anthropic row now shows '(not configured)' and 0 models
  #   13. User opens the ConfirmDialog and presses Esc; the dialog closes; backend.delete_provider_credentials is NEVER called; the anthropic row still shows ✓ configured
  #   14. User highlights an unconfigured row and presses 'd'; nothing happens — no ConfirmDialog opens, no backend call fires. This matches ResumeSessionView's D-on-empty-selection no-op behaviour
  #   15. User highlights the 'codex' row (oauth credential type) and presses Enter; the body transitions to Detail::OAuthNotice with read-only text 'codex uses OAuth which is not yet supported in the Rust frontend — use the legacy TS frontend or env vars'; footer shows 'Esc Back'; no edit form opens; pressing 't' or 'r' is a no-op
  #   16. User is inside Detail::EditApiKey with an empty draft; pressing Enter does NOT save — the body shows inline 'API key cannot be empty' beneath the masked input; the view stays in EditApiKey mode; backend.set_provider_credentials is NEVER called
  #   17. User is in List mode and presses Esc; Navigator returns active_view to ViewMode::Agent; the AgentView's prior input, scrollback, and focused session are intact (no state lost during the round-trip)
  #   18. User is inside Detail::Summary and presses Esc; the view returns to List mode (the focused row index is preserved); pressing Esc again from List mode returns to AgentView — confirming the Esc hierarchy: ConfirmDialog → Detail → List → AgentView
  #   19. User opens /provider against a workspace with 0 configured providers; the body shows the centered placeholder '(no providers configured)' (mirroring ResumeSessionView's '(no sessions to resume)'); pressing Enter is a no-op; pressing 'd' is a no-op; pressing Esc still dispatches Action::CloseProviderSettingsView
  #   20. Cross-transport parity: a test exercises set_provider_credentials + test_provider_connection through both the embedded transport and the WebSocket transport against StubSessionManagerHandle; both transports increment the same per-stub call counters identically — the view-layer rewrite did not regress the backend RPC surface
  #   21. User filters providers by typing "an" — list narrows to anthropic only, then Esc clears filter and Esc again closes the view (two-step cascade).
  #   22. Pressing "/" in filter-capable list mode enters filter mode (does NOT insert the slash anywhere) — matches TS listModeHandler.ts:63-67.
  #   23. Backspace inside the filter input removes the last typed character; Enter exits filter mode while keeping the filter applied to the visible provider list.
  #   24. Filter input "COPILOT" is matched case-insensitively against both provider.id and provider.name, so "github-copilot / GitHub Copilot" matches but "anthropic / Anthropic" does not.
  #   25. User saves api_key 'sk-test-123' for 'mistral' on a machine with no credentials.json yet; the file is created as {version:1,providers:{mistral:{apiKey:'sk-test-123',lastUpdated:<iso>}}} with 0600 perms and the dir is 0700
  #   26. credentials.json already has an 'openai' entry; saving a 'groq' key adds groq alongside openai without disturbing openai or the version field
  #   27. Saving 'mistral' again with a new key replaces the apiKey in place and bumps lastUpdated; no duplicate provider entry is created
  #   28. After saving 'mistral', get_stored_api_key_with_dir('mistral', dir) returns 'sk-test-123' — proving the reload contract refreshes the in-memory cache
  #   29. Deleting 'mistral' from a file containing mistral+groq removes only mistral; groq and version remain; get_stored_api_key for mistral then returns None
  #   30. Deleting the only provider leaves {version:1,providers:{}} on disk (file not removed)
  #   31. set_provider_credentials with kind=api_key and empty api_key returns Err and writes nothing to disk
  #   32. Deleting a provider that is not present in credentials.json is a successful no-op (file unchanged, no error)
  #
  # ASSUMPTIONS:
  #   1. OAuth credential persistence (codex/anthropic/github-copilot) and the openai profile write path remain follow-ups; this card ports only the api_key credentials.json write/delete path
  #
  # ========================================
  Background: User Story
    As a fspec TUI user configuring providers
    I want to save and delete an API key in the /provider screen
    So that the credential is persisted to credentials.json and immediately usable, exactly like the TypeScript frontend

  Scenario: Save an api_key on a machine with no credentials file yet
    Given FSPEC_USER_DIR points at an empty temp directory with no credentials.json
    When set_provider_credentials is called for "mistral" with an api_key input "sk-test-123"
    Then credentials.json is created at <FSPEC_USER_DIR>/credentials/credentials.json
    And the file contains version 1 and providers.mistral.apiKey equal to "sk-test-123"
    And providers.mistral.lastUpdated is a non-empty ISO-8601 timestamp
    And on unix the file mode is 0600 and the credentials directory mode is 0700

  Scenario: Saving a new provider preserves existing entries
    Given credentials.json already contains an "openai" provider entry
    When set_provider_credentials is called for "groq" with an api_key input "gk-123"
    Then providers.groq.apiKey equals "gk-123"
    And the existing providers.openai entry is unchanged
    And the version field is still 1

  Scenario: Saving the same provider again replaces the key in place
    Given credentials.json contains providers.mistral with apiKey "old-key"
    When set_provider_credentials is called for "mistral" with an api_key input "new-key"
    Then providers.mistral.apiKey equals "new-key"
    And providers.mistral.lastUpdated is refreshed
    And there is exactly one "mistral" entry under providers

  Scenario: A saved credential is immediately readable through a store on that directory
    Given FSPEC_USER_DIR points at an empty temp directory
    When set_provider_credentials is called for "mistral" with an api_key input "sk-test-123"
    Then a credential store reading that directory returns "sk-test-123" for "mistral"

  Scenario: Deleting one provider leaves the others intact
    Given credentials.json contains both "mistral" and "groq" provider entries
    When delete_provider_credentials is called for "mistral"
    Then providers.mistral is removed from credentials.json
    And the "groq" entry and the version field remain
    And get_stored_api_key_with_dir("mistral", <FSPEC_USER_DIR>) returns None

  Scenario: Deleting the last provider leaves an empty providers map
    Given credentials.json contains only a "mistral" provider entry
    When delete_provider_credentials is called for "mistral"
    Then credentials.json still exists on disk
    And it contains version 1 and an empty providers map

  Scenario: An empty api_key is rejected and nothing is written
    Given FSPEC_USER_DIR points at an empty temp directory with no credentials.json
    When set_provider_credentials is called for "mistral" with an empty api_key input
    Then the call returns an error
    And no credentials.json file is created

  Scenario: Deleting an absent provider is a successful no-op
    Given credentials.json contains only a "groq" provider entry
    When delete_provider_credentials is called for "mistral"
    Then the call succeeds
    And the "groq" entry is still present and unchanged
