# RPC-344 — AST research: custom-model CRUD UI (a/e/d keybinds + form/confirm overlays)

Goal: wire the UI half on top of the RPC-347 backend write surface. Below are
the concrete Rust anchor points found via AstGrep / Read, plus the TS reference
they mirror.

## Existing Rust state (to be extended)

### `codelet/fspec-tui/src/views/model_selector/mod.rs`
- `pub fn handle_key(&mut self, key: KeyEvent) -> ModelSelectorEvent` — line 295.
  Today handles Esc/`/`/`r`/Tab/arrows/Home/End/Enter; `a`/`e`/`d` fall through to
  `_ => Consumed` (line 364). NEW: add `Char('a'|'e'|'d')` arms gated on the
  focused row + a custom-model mode check at the TOP (intercept before browse).
- `enum ModelSelectorEvent { Consumed, Ignored, Emit(Action), Close, SwitchToProviders }`
  (line 33) — the form/confirm save+delete paths emit `Emit(Action::AddCustomModel
  | EditCustomModel | DeleteCustomModel)`.
- `struct ModelSelectorView { … }` (line 47) — gains a custom-model mode field
  (browse / add / edit / delete-confirm) + a form-state struct (values +
  field_index). `handle_key` checks mode first and routes to form/confirm
  sub-handlers (mirror of TS handleDeleteConfirmInput + handleCustomModelFormInput
  running before normal/filter mode).
- `render(&mut self, area, buf)` (line 386) — when a form/confirm mode is active,
  paint the overlay instead of / over the browse body.
- `focused_provider_key()` (line 204) already exists; add focused-row helpers for
  is_custom / profile membership.

### `codelet/fspec-tui/src/components/model_selector_dialog_rows.rs`
- `pub(crate) struct ModelSelectorRow { label, badges, selectable, provider_key,
  model_id, is_profile, is_unreachable }` (line 25). MISSING the data the e/d guard
  and edit-prefill need. NEW fields: `is_custom: bool`, `profile_name:
  Option<String>` (on header rows), and the capability fields for prefill
  (`context_window: u32`, `supports_reasoning: bool`, `supports_vision: bool`).
  Two constructors set this struct and BOTH must be updated:
  - `build_rows` (dialog, line 48)
  - `build_view_rows` (full-screen, `views/model_selector/rows.rs`)

### Backend surface — already present (RPC-347), DO NOT rebuild
- `FspecBackend::add_custom_model / update_custom_model / delete_custom_model`
  (`transport/mod.rs:193/206/219`, default no-op + embedded/websocket overrides).
- `Action::AddCustomModel { provider_id, profile_name, definition }`,
  `EditCustomModel { …, original_model_id, definition }`,
  `DeleteCustomModel { provider_id, profile_name, model_id }`
  (`components/mod.rs:637/645/654`).
- Wire type `CustomModelDefinition { id, display_name, facade, context_window,
  max_output_tokens, compaction_threshold_type, compaction_threshold_value,
  reasoning, has_vision }` (`rpc-types/src/lib.rs:366`). NOTE the split
  compaction fields — the form's free-text "Compaction Trigger" must be parsed
  ("80%" → percentage/80, "200000" → tokens/200000).

### Dispatch — `codelet/fspec-tui/src/app/dispatch_model_selector.rs`
- `try_dispatch_model_selector` (catch-all). NEW arms: AddCustomModel /
  EditCustomModel / DeleteCustomModel spawn the matching `backend.*` call then a
  `spawn_list_providers_for_selector()` refresh (same pattern as
  `RefreshModelSelector`). Currently these three Actions are inert.

## TS reference (parity source)
- Keybind guards: `src/tui/components/ModelSelectorScreen.tsx:149-187`
  (a = profile header only; e/d = profile + selected custom model id).
- Form fields: `src/tui/constants/customModelForm.ts:38-96` (8 fields, order +
  types + required + options + placeholders).
- Mode union: `src/tui/types/customModelMode.ts:13-41`.
- Input handler: `src/tui/inputHandlers/customModelFormHandler.ts`
  (`handleDeleteConfirmInput` :20-40, `handleCustomModelFormInput` :47-165).
- Save/delete + id-required guard: `src/tui/hooks/useCustomModelFormState.ts`
  (`saveCustomModelForm` :96-143 requires `values.id`; `deleteCustomModelConfirmed`
  :149-162).
- Views: `CustomModelFormView.tsx` (title + profile + 8 rows + footer),
  `DeleteCustomModelConfirmView.tsx` (y/Enter confirm, n/Esc cancel).

## Offline test strategy
All RPC-344 behavior is pure view-state + key routing on `ModelSelectorView`
(no network, no fs): construct a view with a profile-section provider whose
models carry `is_custom`, drive `handle_key`, and assert mode transitions +
emitted `ModelSelectorEvent::Emit(Action::…)` payloads. Render-overlay scenarios
use `TestBackend` like the existing RPC-337/340 tests. The compaction-parse rule
is a pure function asserted directly.
