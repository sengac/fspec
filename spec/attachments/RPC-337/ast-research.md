# RPC-337 — AST Research (AstGrep)

Structural survey of the surfaces RPC-337 will touch, gathered with the
AstGrep tool (language: rust). Captures the **scaffold duplication** to
collapse and the **enum/dispatch surfaces** to extend.

## 1. Full-screen scaffold duplication

### `Clear.render(area, buf)` — the full-screen "overwrite underlying view" marker
Pattern: `Clear.render(area, buf)`
```
codelet/fspec-tui/src/views/provider_settings/mod.rs:260
codelet/fspec-tui/src/views/agent/search_history_view.rs:244
codelet/fspec-tui/src/views/agent/resume_session_view.rs:260
```
→ Exactly THREE existing full-screen mode-views. The new model_selector
view becomes the FOURTH consumer of the shared shell.

### Vertical `Layout::default()` splits (scaffold + others)
Pattern: `Layout::default().direction(Direction::Vertical).constraints([$$$C]).split($A)`
```
codelet/fspec-tui/src/views/board.rs:205
codelet/fspec-tui/src/views/blocklist/mod.rs:162
codelet/fspec-tui/src/views/provider_settings/mod.rs:261     <- scaffold (title/sep/body/footer)
codelet/fspec-tui/src/views/agent/search_history_view.rs:245 <- scaffold
codelet/fspec-tui/src/views/agent/resume_session_view.rs:261 <- scaffold
codelet/fspec-tui/src/views/agent.rs:248
```
→ The 3 scaffold sites use the identical 4-constraint
`[Length(1), Length(1), Min(0), Length(1)]` shape. board/blocklist/agent
use different layouts (not in scope for the shell extraction).

### Shared chrome helpers — current call sites
Pattern: `render_title_with_count($$$ARGS)`
```
codelet/fspec-tui/src/views/provider_settings/mod.rs:272
codelet/fspec-tui/src/views/agent/resume_session_view.rs:273
```
Pattern: `render_footer_hint($$$ARGS)`
```
codelet/fspec-tui/src/views/provider_settings/mod.rs:285
codelet/fspec-tui/src/views/agent/resume_session_view.rs:281
```
(defined in `views/agent/mode_view_render.rs:18` and `:32`,
`pub(crate)`). `search_history_view` does its own title/footer paint —
auditing it during the refit will normalise all three onto the shell.

## 2. Outcome enums (the mode-view event contract to mirror)

Pattern: `pub enum $NAME { $$$V }` (filtered to relevant)
```
codelet/fspec-tui/src/views/navigator.rs:32            ViewMode              <- ADD ModelSelector variant
codelet/fspec-tui/src/views/provider_settings/mod.rs:58 ProviderSettingsEvent <- template for ModelSelectorEvent; has SwitchToModels (Tab) to wire
codelet/fspec-tui/src/views/agent/resume_session_view.rs:39 ResumeSessionViewOutcome
codelet/fspec-tui/src/views/agent/search_history_view.rs:37  SearchHistoryViewOutcome
codelet/fspec-tui/src/views/agent/slash_commands.rs:21       SlashCommandAction    <- has Model variant (name "model")
codelet/fspec-tui/src/views/blocklist/mod.rs:41             BlocklistEvent
codelet/fspec-tui/src/views/agent/confirm_dialog.rs:22      ConfirmDialogOutcome  <- reused as overlay
```
`ProviderSettingsEvent` variants: `Consumed | Ignored | Emit(Action) |
Close | SwitchToModels`. The new `ModelSelectorEvent` should mirror
`Consumed | Ignored | Emit(Action) | Close`.

## 3. Reusable row/navigation builders (no rewrite needed)

Pattern: `pub fn $NAME($$$ARGS) -> $RET { $$$BODY }` in
`components/model_selector_dialog_rows.rs`
```
:38  build_rows(providers: &[ProviderInfo]) -> Vec<ModelSelectorRow>
:89  build_dialog_rows(...)                  <- dialog-specific; new view needs a view variant
:164 move_up_skipping_headers(...)
:184 move_down_skipping_headers(...)
:206 page_step_selectable(...)
:234 first_selectable(rows) -> usize
:239 last_selectable(rows) -> usize
```
→ All currently `pub(super)` to `components::`. To reuse from
`views/model_selector/` they must be re-scoped (move the module, or
promote to `pub(crate)` + relocate). `build_dialog_rows` is tied to the
`DialogRow`/`render_dialog` modal renderer — the new view needs its own
row→Line builder (full-width, with scrollbar), not the popup variant.

## 4. Slash-command + dispatch surfaces to change

- `views/agent/slash_commands.rs:51` — `SlashCommandAction::Model => "model"`
- `views/agent/slash_commands.rs:99` — `SLASH_COMMANDS` entry `action: SlashCommandAction::Model`
- `app/dispatch_rpc020.rs:53` — `SlashCommandAction::Model =>` arm (currently opens the modal)
- `app/dispatch_rpc022.rs:30` — `handle_open_model_dialog()` pushes `ModelSelectorDialog` onto the Compositor (to be replaced with `Action::OpenModelSelectorView`)
- `app/dispatch_rpc022.rs:216` — `Action::OpenModelDialog` match arm in `try_dispatch_rpc022`
- `components/mod.rs:374` — `Action::OpenModelDialog` (retire); add `OpenModelSelectorView` / `CloseModelSelectorView`
- `components/mod.rs:384` — `Action::ListProvidersLoaded(Vec<ProviderInfo>)` (keep — folds list into the view)
- `components/mod.rs:389` — `Action::ModelSelected(SessionId, String, String)` (keep — selection commit)

## 5. Navigator routing functions to extend

`views/navigator.rs`:
- `ViewMode` enum (`:32`) — add `ModelSelector`.
- struct `Navigator` (`:47`) — add `model_selector: ModelSelectorView` field.
- `handle_event` (`:84`) — add `ViewMode::ModelSelector` arm → new `handle_model_selector_event`.
- `apply_action` (`:155`) — add `OpenModelSelectorView` / `CloseModelSelectorView` arms.
- `render_with_stores` (`:177`) — add `ViewMode::ModelSelector => self.model_selector.render(area, buf)` arm.
- `handle_provider_settings_event` (`:100`) — `ProviderSettingsEvent::SwitchToModels` currently `EventResult::consumed()` no-op; rewire to send `Action::OpenModelSelectorView` (or a SwitchToModels action) so Tab flips views.

## 6. Wire types (unchanged)
`codelet/rpc-types/src/lib.rs`:
- `ModelEntry` (`:341`): `id, display_name, context_window, supports_reasoning, supports_vision, is_custom`
- `ProviderInfo` (`:362`): `key, display_name, models: Vec<ModelEntry>`
- `ProviderCredentialInfo` (`:402`): provider-settings only — not needed by model selector.
