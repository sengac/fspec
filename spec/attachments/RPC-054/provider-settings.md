# RPC-054 — `/provider` ProviderSettingsView + provider-credentials RPC surface (REVISED)

**Parent:** RPC-030 · **Phase:** 7.1 · **Estimate:** 8 pts · **Depends on:** RPC-053

> **REVISION (2026-06-01):** The first pass of RPC-054 was structurally incorrect and is being re-specified.
> Key defects identified by the user:
>
> 1. **A non-existent `/providers` command was invented.** No `/providers` command exists in the TypeScript
>    Ink reference frontend (`src/tui/utils/slashCommands.ts` defines `provider` only). The Rust
>    `SlashCommandAction::Providers` variant, its registry entry, and the
>    `SlashCommandAction::Provider | SlashCommandAction::Providers` arm in `dispatch_rpc020.rs`
>    MUST be removed.
> 2. **`ProviderSettingsView` does not follow the established full-screen mode-view pattern.** It is implemented
>    as a small `Block` with a border, two side-by-side panes and an inline status string. The correct pattern
>    is the `ResumeSessionView` shape from RPC-026 — `Clear.render(area, buf)` first, a `Layout::default()
>    .constraints([Length(1), Length(1), Min(0), Length(1)])` split for title / separator / scrollable body /
>    footer hint, scroll-window math via `ensure_visible` + `wrap_index`, and identical key bindings
>    (↑/↓/PageUp/PageDown/Home/End wrap-around, Enter Select, D Delete with `ConfirmDialog`, Esc Dismiss).
> 3. **No `ConfirmDialog` is used for destructive 'd' (delete) action.** RPC-026 uses `ConfirmDialog`
>    for delete-session confirmation. RPC-054 currently fires the backend round-trip directly with no
>    confirmation step — that is wrong and inconsistent with the ResumeSessionView UX contract.
> 4. **Test-and-refresh keys (`t`, `r`) are bound on the list view itself.** That conflicts with the
>    full-screen mode-view contract where the body is a navigable list and side-effect actions are reached
>    via Enter on a dedicated detail/edit screen. The RPC-026 pattern is: list rows are read-only navigation,
>    Enter opens a detail/edit sub-view, destructive actions live behind `ConfirmDialog`.
> 5. **Help RPC-002 architecture compliance.** The view's `render` paints a bordered `Block` with title
>    instead of letting AgentView's mode-view dispatcher hand it the full area Rect. This violates the
>    full-screen render contract (`Clear.render(area, buf)` then layout into the supplied Rect) used by
>    `BoardView`, `ResumeSessionView`, `SearchHistoryView`, `BlocklistView`.

---

## Goal

Port the TypeScript `ProviderSettingsScreen` (`src/tui/components/ProviderSettingsScreen.tsx`) to Rust
**as a full-screen mode view that follows the exact pattern established by `ResumeSessionView`
(RPC-026)**. Add the supporting RPC surface (`list_provider_credentials`, `set_provider_credentials`,
`test_provider_connection`, `refresh_models_cache`) on `SessionManagerHandle`, `FspecService`, and
`FspecBackend`.

## TS reference (canonical command name)

`src/tui/utils/slashCommands.ts` — the TypeScript registry — defines the slash command exactly:

```typescript
{
  name: 'provider',
  description: 'Configure API providers',
  requiresSession: false,
},
```

**There is NO `/providers` command.** The Rust TUI must mirror this 1:1.

The TS `AgentView.tsx` dispatches `/provider` (singular) to `ProviderSettingsScreen` which composes
`useProviderSettingsState` + `useProviderSettingsInput` + `ProviderSettingsPanel`. The Rust port must
deliver the same singular-command UX surface.

## RPC-002 architectural compliance

The view follows the **full-screen mode-view contract** documented in:

- `spec/attachments/RPC-002/07-recommended-architecture.md` (Compositor + view dispatch)
- `spec/features/rpc026-resume-session-view.feature` (full-screen render pattern)
- `codelet/fspec-tui/src/views/agent/resume_session_view.rs` (canonical implementation)

Specifically:

1. View is owned by `Navigator` as `provider_settings: ProviderSettingsView`.
2. `Navigator::handle_event` routes `Event::Key(_)` to `provider_settings.handle_key(...)` when
   `active_view == ViewMode::ProviderSettings`.
3. View `render(area, buf)` paints into the **full** area Rect: first statement is
   `Clear.render(area, buf)`; then a `Layout::default().direction(Vertical)
   .constraints([Length(1), Length(1), Min(0), Length(1)])` split for **title / separator / body / footer**.
4. The body is a scrollable list with `ensure_visible(scroll_offset, selected_index, visible_rows, len)`
   and `wrap_index(selected, delta, len)` from `crate::components::scroll_viewport` — the exact same
   helpers `ResumeSessionView` uses.
5. Destructive actions (delete credentials) go through `ConfirmDialog` (mirrors RPC-026 delete-session).
6. Key bindings mirror `ResumeSessionView`:
   - `↑` / `↓` — wrap-around navigation with scroll-window adjustment
   - `PageUp` / `PageDown` — page-size jumps
   - `Home` / `End` — jump to extremes
   - `Enter` — open per-provider detail view (api-key edit form OR OAuth read-only notice)
   - `d` / `D` — open `ConfirmDialog` for credential deletion (configured providers only)
   - `t` / `T` — open per-provider detail view AND fire connection test from there (NOT on list view)
   - `r` / `R` — open per-provider detail view AND fire model refresh from there (NOT on list view)
   - `Esc` — Dismiss view (back to AgentView)
7. View paints title with count: `"Provider Settings (N configured)"`, identical to
   `ResumeSessionView`'s `"Resume Session (N available)"`.
8. Footer hint string: `"Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"` — same shape as
   `ResumeSessionView`'s footer.

## Architectural fixes from the first pass

| Defect                                                                                              | Fix                                                                                                                                                                                                                                                          |
| --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `SlashCommandAction::Providers` variant exists and is dispatched.                                   | DELETE the variant from `SlashCommandAction`, its `name()` arm, its `SLASH_COMMANDS` registry entry, and the `\| SlashCommandAction::Providers` arm in `dispatch_rpc020.rs`. Update `behaviour_parity_rpc065.rs` to remove `slash_providers_alias_*` test.    |
| `ProviderSettingsView::render` paints a bordered `Block`.                                           | Replace with `Clear.render(area, buf)` first, then the 4-constraint vertical layout. Reuse `render_title_with_count` and `render_footer_hint` from `mode_view_render.rs` so the title row + footer paint identically to `ResumeSessionView`.                 |
| Two side-by-side panes (list + status).                                                             | Replace with single scrollable list. Right-pane detail is shown only when the user presses `Enter` on a row — that opens a per-provider detail sub-view that overlays the list area (NOT replaces the whole screen).                                         |
| `t` / `r` / `d` keys bound on list view, fire immediate side effects.                               | Move `t` / `r` to detail sub-view (reached via Enter). Keep `d` on list view BUT route through `ConfirmDialog` before firing the backend round-trip.                                                                                                         |
| Scroll uses ad-hoc `selected_index += 1` clamp.                                                     | Replace with `ensure_visible` + `wrap_index` from `crate::components::scroll_viewport`, identical to `ResumeSessionView`.                                                                                                                                    |
| Status is a free-form `String` field rendered inline.                                               | Status moves to detail sub-view (where it makes sense — it's per-provider). The list view itself never shows free-form status text.                                                                                                                          |
| API-key edit form is reached via `Enter` on list, replaces the right pane.                          | API-key edit form is one variant of the detail sub-view, opened by `Enter` on an api_key-type row. It paints into the body Rect (the title+separator+footer chrome stay visible), mirroring how `ResumeSessionView` paints `ConfirmDialog` over its body.    |
| OAuth rows show inline notice text on the right pane.                                               | OAuth rows open a read-only detail sub-view explaining the limitation. The list view itself never has free-form status.                                                                                                                                      |

## Backend trait additions (UNCHANGED from first pass)

The RPC surface itself was correct — the defect was purely in the frontend view. These stay:

```rust
fn list_provider_credentials(&self) -> Vec<ProviderCredentialInfo>;
fn get_provider_credential(&self, provider_id: &str) -> Option<ProviderCredentialInfo>;
fn set_provider_credentials(&self, provider_id: &str, creds: ProviderCredentialInput) -> Result<(), String>;
fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String>;
fn test_provider_connection(&self, provider_id: &str) -> Result<TestConnectionResult, String>;
fn refresh_models_cache(&self, provider_id: &str) -> Result<Vec<ModelEntry>, String>;
```

`ProviderCredentialInfo`, `ProviderCredentialInput`, `TestConnectionResult` wire types in
`codelet/rpc-types/src/lib.rs` are also unchanged.

## Frontend — `ProviderSettingsView` (REVISED)

`codelet/fspec-tui/src/views/provider_settings/mod.rs`:

```rust
use codelet_rpc_types::{ProviderCredentialInfo, TestConnectionResult};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Clear, Widget};

use crate::components::scroll_viewport::{ensure_visible, wrap_index};
use crate::views::agent::confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};
use crate::views::agent::mode_view_render::{render_footer_hint, render_title_with_count};

const CHROME_ROWS: u16 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProviderSettingsMode {
    #[default]
    List,
    Detail {
        provider_id: String,
        sub: DetailSub,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailSub {
    /// Default detail view — read-only summary + footer hint
    /// "t: test  |  r: refresh models  |  Esc: back".
    Summary { last_status: Option<DetailStatus> },
    /// Inline API key edit form (api_key credential type only).
    EditApiKey { draft: String },
    /// OAuth notice — read-only.
    OAuthNotice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailStatus {
    Testing,
    TestOk { latency_ms: u32 },
    TestErr { error: String },
    RefreshingModels,
    ModelsRefreshed { count: u32 },
    SavingCredentials,
    CredentialsSaved,
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSettingsEvent {
    Consumed,
    Ignored,
    Close,
    Emit(crate::components::Action),
}

pub struct ProviderSettingsView {
    providers: Vec<ProviderCredentialInfo>,
    selected_index: usize,
    scroll_offset: usize,
    mode: ProviderSettingsMode,
    delete_confirm: Option<ConfirmDialog>,
}
```

Render contract:

```rust
impl ProviderSettingsView {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);
        let title_area = split[0];
        let body_area = split[2];
        let footer_area = split[3];

        let configured_count = self.providers.iter().filter(|p| p.configured).count();
        render_title_with_count(title_area, buf, "Provider Settings", configured_count);

        match &self.mode {
            ProviderSettingsMode::List => self.render_list(body_area, buf),
            ProviderSettingsMode::Detail { provider_id, sub } => {
                self.render_detail(body_area, buf, provider_id, sub);
            }
        }

        let hint = self.footer_hint();
        render_footer_hint(footer_area, buf, hint);

        if let Some(dialog) = self.delete_confirm.as_ref() {
            dialog.render(area, buf);
        }
    }
}
```

Footer hints by mode:

- `List`: `"Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"`
- `Detail::Summary`: `"t Test | r Refresh Models | Esc Back"`
- `Detail::EditApiKey`: `"Enter Save | Esc Cancel"`
- `Detail::OAuthNotice`: `"Esc Back"`

## Slash command wiring (CORRECTED)

`SlashCommandAction::Provider` is the ONLY enum variant. There is no `Providers` variant. The arm
in `dispatch_rpc020.rs` becomes:

```rust
SlashCommandAction::Provider => {
    // RPC-054: open the ProviderSettingsView. Singular `/provider` only —
    // no `/providers` alias (matches the TypeScript SLASH_COMMANDS registry
    // which defines exactly one entry: `name: 'provider'`).
    let _ = self.action_tx.send(Action::OpenProviderSettingsView);
}
```

`Navigator::apply_action`:

```rust
Action::OpenProviderSettingsView => {
    self.active_view = ViewMode::ProviderSettings;
}
Action::CloseProviderSettingsView => self.active_view = ViewMode::Agent,
```

`Navigator::handle_event` already forwards `Event::Key(_)` to `provider_settings.handle_key(...)` —
that wiring stays. Mouse events are not in scope for this card (consistent with
`ResumeSessionView` which is keyboard-only).

## Acceptance criteria (CORRECTED)

1. `SlashCommandAction::Providers` variant is DELETED. The `SLASH_COMMANDS` registry contains
   exactly one provider-related entry: `Provider`. `dispatch_rpc020.rs` has a single
   `SlashCommandAction::Provider =>` arm.
2. `ProviderSettingsView::render(area, buf)` first calls `Clear.render(area, buf)`, then splits the
   FULL area into `[Length(1), Length(1), Min(0), Length(1)]` for title / separator / body / footer.
   The view paints NO outer `Block` with borders.
3. View uses `render_title_with_count` and `render_footer_hint` from `mode_view_render.rs` —
   same helpers as `ResumeSessionView`.
4. Title text is `"Provider Settings (N configured)"` where N is the count of `configured: true`
   rows.
5. Scroll mechanics use `ensure_visible` + `wrap_index` from `crate::components::scroll_viewport`.
   `↑` / `↓` wrap around, `PageUp` / `PageDown` jump by `visible_rows`, `Home` / `End` jump to
   extremes — identical bindings to `ResumeSessionView`.
6. Pressing `Enter` on a row transitions `mode` from `List` to `Detail { provider_id, sub: Summary }`.
   For api_key rows, pressing `Enter` again from `Summary` opens `Detail::EditApiKey`.
   For oauth rows, pressing `Enter` transitions directly from `List` to `Detail::OAuthNotice`.
7. From `Detail::Summary` pressing `t` fires `Action::TestProviderConnection(provider_id)`. The
   status text in the body updates to `"Testing…"` then `"✓ ok (Xms)"` or `"✗ <error>"` when the
   round-trip completes.
8. From `Detail::Summary` pressing `r` fires `Action::RefreshProviderModels(provider_id)`. The
   status text updates to `"Refreshing models…"` then `"✓ models refreshed (N)"` or
   `"✗ <error>"`.
9. From `Detail::EditApiKey` pressing `Enter` with a non-empty draft fires
   `Action::SaveProviderCredentials { provider_id, api_key }`; an empty draft surfaces inline
   `"API key cannot be empty"` and stays in EditApiKey mode without emitting.
10. From any `Detail::*` mode, `Esc` returns to `List` mode (NOT to AgentView).
11. From `List` mode, pressing `d` / `D` on a row with `configured: true` opens a `ConfirmDialog`
    with title `"Delete credentials?"`, body `"Delete credentials for {provider_id}?"`, primary
    label `"Delete"`, cancel label `"Cancel"`. Pressing `d` / `D` on a `configured: false` row is
    a no-op (matches `ResumeSessionView`'s D-on-empty-selection behaviour).
12. Pressing `Enter` on the `ConfirmDialog`'s Primary fires
    `Action::DeleteProviderCredentials(provider_id)`. Pressing `Esc` (or Cancel) closes the dialog
    without emitting any action.
13. From `List` mode, `Esc` emits `ProviderSettingsEvent::Close` which Navigator translates into
    `Action::CloseProviderSettingsView`.
14. `Tab` is NOT bound (the TS frontend's tab-to-models keybind is out of scope for this card;
    track as follow-up).
15. The `behaviour_parity_rpc065.rs::slash_providers_alias_activates_provider_settings_view`
    test is DELETED (no `/providers` alias exists).
16. `behaviour_parity_rpc065.rs::slash_provider_activates_provider_settings_view` continues to
    pass against the corrected dispatch arm.
17. All scenarios in `rpc054-provider-settings-view.feature` are rewritten to match the new
    list/detail/confirm-dialog UX.
18. Source-shape feature asserts: `views/provider_settings/mod.rs` exists; it imports
    `Clear`, `ensure_visible`, `wrap_index`, `ConfirmDialog`, `render_title_with_count`,
    `render_footer_hint`; it does NOT import `Borders` or `Block`.
19. The view file stays under 300 LoC (RPC-002 rule [10]). If render+key handling logic exceeds
    300 LoC, split into `provider_settings/list.rs`, `provider_settings/detail.rs`,
    `provider_settings/mod.rs` (orchestrator) — same shape as
    `views/agent/{resume_session_view, search_history_view, ...}`.
20. Backend trait surface (`list_provider_credentials`, etc.) is UNCHANGED — only the view layer
    and slash-command registry change.

## Out of scope (deferred to follow-up RPC cards)

- Tab-to-models keybind (existing TS `Tab` → switch to ModelSelectorScreen).
- Custom provider creation (add new provider).
- Real credential persistence in `codelet-sessions::handle_impl::set_provider_credentials` / 
  `delete_provider_credentials` (currently no-op success; this stays).
- OAuth flow (PKCE, device flow) for codex / github-copilot.
- Profile sub-list inside provider rows (TS frontend has per-provider profiles for OpenAI).
- Mouse hit-testing.

## Risks

- Existing integration tests (`provider_settings_view_rpc054.rs`,
  `provider_settings_dispatch_rpc054.rs`, `source_shape_rpc054.rs`,
  `rpc054_cross_transport_parity.rs`) will need to be updated to drive the new mode-view shape.
  Plan: rewrite assertions to target `ViewMode::ProviderSettings` + the new `mode` enum and
  `delete_confirm` field; preserve cross-transport parity coverage verbatim (the backend RPC
  surface didn't change).
- Removing `SlashCommandAction::Providers` is a breaking change to the public enum — but the
  enum is internal to `codelet-fspec-tui` and never crosses the tarpc wire, so safe to remove.

## Source-shape sketch

```
codelet/fspec-tui/src/views/provider_settings/
  mod.rs              # orchestrator (< 200 LoC):
                      #   ProviderSettingsView struct, key dispatcher,
                      #   render() outer shell, ProviderSettingsMode enum
  list.rs             # render_list(area, buf, providers, selected, scroll)
  detail.rs           # render_detail(area, buf, provider, sub)
  status_text.rs      # DetailStatus → footer-coloured text mapping
```
