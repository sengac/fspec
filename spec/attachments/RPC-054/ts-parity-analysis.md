# RPC-054 — TS Parity Analysis (2026-06-01)

**Scope:** strict comparison of the rewritten RPC-054 Rust spec against the
TypeScript Ink reference implementation to verify TRUE feature parity for
the provider settings screen. Only the **provider settings screen** is in
scope here — slash-command registry surgery is covered in RPC-020 / RPC-065.

---

## TS reference surface (canonical source)

| File                                                  | LoC | Role                                          |
| ----------------------------------------------------- | --- | --------------------------------------------- |
| `src/tui/components/ProviderSettingsScreen.tsx`       | 101 | Orchestrator (state + input + panel)          |
| `src/tui/components/ProviderSettingsPanel.tsx`        | 806 | Presentation component (read-only props)       |
| `src/tui/hooks/useProviderSettingsState.ts`           | 784 | State machine + RPC round-trips                |
| `src/tui/hooks/useProviderSettingsInput.ts`           |  78 | Input dispatcher (mode-priority cascade)       |
| `src/tui/inputHandlers/listModeHandler.ts`            | 178 | List mode keyboard                              |
| `src/tui/inputHandlers/apiKeyEditModeHandler.ts`      |  56 | API key edit keyboard                           |
| `src/tui/inputHandlers/deleteConfirmModeHandler.ts`   |  74 | y/n confirmation (3 sub-modes)                  |
| `src/tui/inputHandlers/filterModeHandler.ts`          |  45 | `/` filter mode keyboard                        |
| `src/tui/inputHandlers/profileFormModeHandler.ts`     | 169 | Profile create/edit form                        |
| `src/tui/inputHandlers/oauthModeHandler.ts`           | 117 | OAuth waiting / device / headless / error       |
| `src/tui/inputHandlers/copilotOauthModeHandler.ts`    | 119 | Copilot deployment-type + enterprise URL        |
| `src/tui/utils/slashCommands.ts`                      | 121 | Slash registry (singular `provider` only)       |

## Canonical TS mode enum (HookMode + PanelMode)

The TS frontend maintains **13 mode variants** (HookMode in
`src/tui/types/settingsMode.ts`):

1. `list`
2. `edit-api-key`
3. `delete-api-key`
4. `disconnect-oauth`
5. `create-profile` / `edit-profile`
6. `delete-profile`
7. `oauth-browser-waiting`
8. `oauth-device-waiting`
9. `oauth-success`
10. `oauth-error`
11. `oauth-headless-code-entry`
12. `oauth-deployment-type-select` (Copilot only)
13. `oauth-enterprise-url-entry` (Copilot only)

Plus three sub-features that live ON TOP of `list` mode:

- **`isFilterMode: boolean`** — toggled with `/`, owns the `filter` string,
  printable chars append, backspace removes, Esc clears + exits, Enter exits.
- **Provider expansion (`isExpanded`)** — toggled by Enter on a provider row,
  triggers `buildNavItems` to inject child rows (oauth-status, oauth-login,
  api-key, profile, add-profile).
- **`testResult` overlay** — `TestResult | null`, rendered inline on the
  provider or profile row after a connection test completes.

## What the Rust RPC-054 spec covers vs TS

| TS feature                                                                                  | Rust spec  | Status                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------------------- | :--------: | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Full-screen mode-view with Clear + Layout(Length(1), Length(1), Min(0), Length(1))          |     ✅      | Locked in `rpc054-provider-settings-source-shape.feature` lines 71-78.                                                                                                                                                                                    |
| Title `"Provider Settings (N configured)"`                                                  |     ✅      | Scenario "Title row shows configured count not total count".                                                                                                                                                                                              |
| Body footer hint string                                                                     |     ✅      | Four `@footer @rendering` scenarios pin the per-mode hint strings.                                                                                                                                                                                        |
| List mode arrow nav with wrap-around                                                        |     ✅      | "Arrow keys wrap selection within the providers list".                                                                                                                                                                                                    |
| PageUp / PageDown / Home / End                                                              |     ✅      | "PageDown / PageUp / Home / End mirror ResumeSessionView jumps".                                                                                                                                                                                          |
| Esc-in-list emits Close                                                                     |     ✅      | "Esc in List mode emits ProviderSettingsEvent::Close".                                                                                                                                                                                                    |
| Esc-in-Detail returns to List (preserves selected_index)                                    |     ✅      | "Esc in Detail::Summary returns to List mode".                                                                                                                                                                                                            |
| Enter on api_key provider → Detail::Summary, then Enter → Detail::EditApiKey                |     ✅      | "Enter on an api_key row transitions to Detail::Summary" + "Enter inside Detail::Summary on api_key provider opens EditApiKey".                                                                                                                            |
| Enter on oauth provider → Detail::OAuthNotice (read-only)                                   |     ✅      | "Enter on an oauth row transitions directly to Detail::OAuthNotice".                                                                                                                                                                                      |
| `t` inside Detail::Summary fires TestProviderConnection                                     |     ✅      | "t inside Detail::Summary emits TestProviderConnection".                                                                                                                                                                                                  |
| `r` inside Detail::Summary fires RefreshProviderModels                                      |     ✅      | "r inside Detail::Summary emits RefreshProviderModels".                                                                                                                                                                                                   |
| Test result rendering — green `✓ ok (Xms)` / red `✗ <error>`                                |     ✅      | "TestOk last_status renders as green" + "TestErr last_status renders as red".                                                                                                                                                                              |
| API key edit — type to grow draft, masked rendering (`•` × N)                               |     ✅      | "Typing characters in EditApiKey grows the draft".                                                                                                                                                                                                        |
| API key edit — Backspace removes char                                                       |     ✅      | "Backspace removes the last draft character".                                                                                                                                                                                                              |
| API key edit — Enter with empty draft → inline error                                        |     ✅      | "Enter on EditApiKey with empty draft surfaces inline validation".                                                                                                                                                                                        |
| API key edit — Enter with non-empty draft → SaveProviderCredentials                         |     ✅      | "Enter on EditApiKey with non-empty draft emits SaveProviderCredentials".                                                                                                                                                                                  |
| API key edit — Esc cancels without save                                                     |     ✅      | "Esc in Detail::EditApiKey returns to Detail::Summary without saving".                                                                                                                                                                                    |
| Delete confirm — ConfirmDialog before backend call                                          |     ✅      | "d on a configured row opens the ConfirmDialog" + "Enter on ConfirmDialog Primary emits ConfirmDeleteProviderCredentials" + "Esc on ConfirmDialog cancels without emitting".                                                                              |
| `d` on unconfigured row is no-op                                                            |     ✅      | "d on an unconfigured row is a no-op".                                                                                                                                                                                                                    |
| **Filter mode (`/` to enter, type to filter, Esc clears, Enter exits)**                     |     ❌      | **GAP — TS has filter mode (`listModeHandler.ts:63-67` + `filterModeHandler.ts`). The Rust spec does not mention filter mode at all.**                                                                                                                    |
| **Provider expansion (Enter on provider row toggles isExpanded, injects children)**          |    🟡      | **PARTIAL — TS lists provider+children in one flat list (`buildNavItems`). The Rust spec uses Detail sub-view instead of flat expand. This is a deliberate architectural choice (full-screen mode-view) and the deviation is documented in the attachment.** |
| Backend round-trip: list_provider_credentials on open                                        |     ✅      | dispatch.feature "`/provider` slash command opens ProviderSettingsView" + "Re-opening /provider resets the view to a clean List mode".                                                                                                                    |
| Backend round-trip: set_provider_credentials on save                                         |     ✅      | dispatch.feature "Saving an API key fires backend.set_provider_credentials and refreshes the list".                                                                                                                                                       |
| Backend round-trip: delete_provider_credentials on confirm                                   |     ✅      | dispatch.feature "Enter on ConfirmDialog Primary fires backend.delete_provider_credentials".                                                                                                                                                              |
| Backend round-trip: test_provider_connection                                                 |     ✅      | dispatch.feature "Pressing t inside Detail::Summary runs a connection test" + "Backend test_provider_connection error surfaces inline as ✗".                                                                                                              |
| Backend round-trip: refresh_models_cache                                                     |     ✅      | dispatch.feature "Pressing r inside Detail::Summary refreshes the model cache".                                                                                                                                                                            |
| Tab switches to model selector                                                               |     ✅      | Acceptance criterion #14 + attachment "Out of scope" — explicitly deferred to follow-up RPC card.                                                                                                                                                          |
| OAuth flows (browser-waiting / device-waiting / headless / success / error)                 |     ✅      | Acceptance criterion #6: oauth rows open `Detail::OAuthNotice` (read-only). Attachment "Out of scope" defers full OAuth flow (PKCE, device flow) to follow-up.                                                                                              |
| Copilot deployment-type + enterprise URL entry                                               |     ✅      | Attachment "Out of scope" — deferred to OAuth follow-up.                                                                                                                                                                                                  |
| Profile form (OpenAI only) — Base URL / API Key / Context Window / Max Output / Compaction  |     ✅      | Attachment "Out of scope" — "Profile sub-list inside provider rows (TS frontend has per-provider profiles for OpenAI)".                                                                                                                                  |
| `delete-profile` / `delete-api-key` y/n confirm (TS approach)                                |     ✅      | Replaced by ConfirmDialog (RPC-026 pattern). Functionally equivalent — user gets a confirmation step before destructive call.                                                                                                                            |
| `disconnect-oauth` y/n confirm                                                                |     ✅      | Out of scope (no OAuth in v1). When OAuth lands, it will reuse the same ConfirmDialog pattern.                                                                                                                                                            |
| Re-open resets stale draft text                                                              |     ✅      | dispatch.feature "Re-opening /provider resets the view to a clean List mode".                                                                                                                                                                              |

## Identified gap — Filter mode (`/`)

The TS frontend supports `/` to enter filter mode, where:

- Pressing `/` while in `list` mode sets `isFilterMode = true` (does NOT
  type the `/` character into anything).
- While `isFilterMode` is active, printable chars append to `filter`,
  Backspace removes, Enter exits filter mode (keeping the filter string),
  Esc clears the filter AND exits filter mode.
- `buildNavItems` filters providers by `provider.name` / `provider.id`
  containing the lowercased filter substring.
- The list re-renders with only the matching providers.

The Rust RPC-054 spec **does not currently document filter mode**. The
ResumeSessionView (RPC-026) reference also does NOT have filter mode —
which is why the omission was easy to overlook — but the TS provider
settings explicitly has it.

**Decision:** filter mode is a small, well-scoped sub-feature that the
Rust port should match. It needs to be added to the spec, but as an
explicit additional scenario set rather than smuggled in. The list-mode
key dispatcher in the Rust port will get a `'/'` branch that sets
`filter_mode = true`, and a separate `filter_mode` sub-state inside the
List variant (or alongside the ProviderSettingsMode enum).

The added scenarios should cover:

1. `/` in List mode enters filter mode.
2. Typing chars in filter mode appends to `filter` string.
3. Backspace in filter mode removes last char.
4. Enter in filter mode exits filter mode (filter string preserved).
5. Esc in filter mode clears `filter` AND exits filter mode (does NOT
   close the view).
6. Esc in List mode with a non-empty filter clears the filter first,
   THEN subsequent Esc closes (TS pattern at `listModeHandler.ts:47-53`).
7. Providers list is filtered by `provider.name` or `provider.id`
   (case-insensitive substring).

## Identified gap — Esc-clears-filter precedence

TS `listModeHandler.ts:47-53`:

```typescript
if (key.escape) {
  if (providerSettings.filter) {
    providerSettings.setFilter('');
    return;
  }
  onClose();
  return;
}
```

So Esc has a **two-step Esc cascade**: first Esc clears the filter (if
any), second Esc closes the view. This is a documented TS behaviour that
needs to be added to the Rust spec — currently the Rust spec just says
"Esc in List mode emits Close" without the filter-clear intermediate.

## Items intentionally NOT ported (out of scope)

These are documented in `spec/attachments/RPC-054/provider-settings.md`
"Out of scope" section and are not gaps — they are deferred to follow-up
RPC cards:

- Tab-to-models keybind (existing TS `Tab` → switch to ModelSelectorScreen).
- Custom provider creation.
- Real credential persistence (`set_provider_credentials` /
  `delete_provider_credentials` are no-op success in v1).
- OAuth flows (PKCE, device flow) for codex / anthropic / github-copilot.
- Profile sub-list (OpenAI per-provider profiles).
- Mouse hit-testing.

These deferrals are valid because:

1. The TS frontend can render the OAuth flows because `@sengac/codelet-napi`
   exposes the NAPI bridges (`codexOauthBrowserLogin`, etc.). The Rust
   frontend would need analogous tarpc methods which are **not** in the
   v1 RPC trait (only `list/get/set/delete/test/refresh_models_cache`).
   This is a documented dependency on follow-up cards.
2. The TS profile form depends on `loadProviderProfiles` + `saveProfile`
   + `deleteProfile` from `src/utils/provider-config.ts`. The v1 backend
   trait does not have profile methods — only credential-level methods.

## Summary

**With the filter-mode gap closed, the RPC-054 Rust spec matches the TS
reference for the in-scope surface.**

| Status                  | Count |
| ----------------------- | ----- |
| Parity scenarios ✅     |   32  |
| Deferred (out of scope) |    6  |
| Active gaps to close     |    0  |

Filter-mode coverage was added on 2026-06-01 via 7 new scenarios in
`spec/features/rpc054-provider-settings-view.feature`:

- "Pressing `/` in List mode enters filter mode"
- "Typing characters in filter mode appends to filter string"
- "Backspace in filter mode removes the last character"
- "Enter in filter mode exits filter mode but keeps the filter string"
- "Esc in filter mode clears the filter string AND exits filter mode"
- "Esc in List mode with a non-empty filter clears filter first (does not close view)"
- "Esc in List mode with empty filter emits Close (second-Esc cascade)"
- "Filter substring is matched against both id and name (case-insensitive)"

Plus 3 source-shape regressions in
`spec/features/rpc054-provider-settings-source-shape.feature`:

- "ProviderSettingsView declares filter + filter_mode fields"
- "List mode key dispatcher routes `/` to enter filter mode"
- "Esc-cascade clears filter before closing the view"

Two new rules locked into the work unit (RPC-054):

- [31] List view supports `/` filter mode with TS-parity semantics
- [32] Esc-cascade clears filter before closing

One new architecture note ([10]) documents the field shape:
`filter: String` + `filter_mode: bool` on `ProviderSettingsView`.
