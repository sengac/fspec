# RPC-345 — Model selector missing Tab to return to Provider Settings

**Severity: LOW** — small, surgical parity fix. The ProviderSettings→Models
direction works; the reciprocal ModelSelector→ProviderSettings is unimplemented,
breaking the bidirectional Tab toggle.

## Summary

TS has symmetric Tab: both screens have a Tab keybind, and `AgentView` wires the
two callbacks to flip its mutually-exclusive flags. Rust implemented only the
forward stub (`ProviderSettingsEvent::SwitchToModels` → `OpenModelSelectorView`);
the model selector has no `Tab` arm and no `ModelSelectorEvent` variant to switch
back. Three small edits mirror the existing path.

---

## PART 1 — TS reference (bidirectional Tab works)

- ModelSelector → Settings: `ModelSelectorScreen.tsx:145` `if (key.tab) { onSwitchToSettings(); return; }`
  (prop `:36`, destructured `:45`). Filter mode handled earlier `:133-141`.
- Wiring in orchestrator `AgentView.tsx`:
  - `onSwitchToSettings` `:5031-5034` (`setShowModelSelector(false); setShowSettingsTab(true)`)
  - `onSwitchToModels` `:5050-5054` (reciprocal)
- `ModelSelectorView.tsx` is presentational only (no Tab logic).
- Tests confirm both: `ModelSelectorScreen.integration.test.tsx:356`,
  `ProviderSettingsScreen.integration.test.tsx:106`.

---

## PART 2 — Rust current state

| Concept | TS | Rust |
|---|---|---|
| Orchestrator owning view flag | `AgentView` flags | `Navigator.active_view: ViewMode` |
| Per-screen event | `onSwitchTo*` callbacks | `*Event::SwitchTo*` enums |
| Flag flip | `setShow*` | `Action::Open*View` → `apply_action` sets `ViewMode` |

### ProviderSettings → Models (implemented)
- `ProviderSettingsEvent::SwitchToModels` variant: `provider_settings/mod.rs:58-70` (`:69`)
- Tab emits it: `provider_settings/list.rs:62` `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels`
- Navigator handles: `navigator_events.rs:24-52` (`:45-50`) → sends
  `Action::OpenModelSelectorView`
- Applied: `navigator.rs:118-120` `self.active_view = ViewMode::ModelSelector`
  (`ViewMode::ModelSelector` `navigator.rs:39-43`)

### ModelSelector → Providers (MISSING — confirmed)
- `ModelSelectorEvent` enum `mod.rs:33-38`: only `Consumed | Ignored | Emit(Action) | Close`
  — **no `SwitchToProviders`**.
- `handle_key` `:225-289`: no `KeyCode::Tab` arm → Tab falls into catch-all
  `_ => ModelSelectorEvent::Consumed` (`:287`).
- `handle_model_selector_event` `navigator_events.rs:57-77`: only
  `Consumed | Ignored | Close | Emit(action)` — no SwitchToProviders arm.

---

## PART 3 — Proposed change (mirror SwitchToModels)

**(a) Add enum variant** — `model_selector/mod.rs` (enum `:33-38`):
```rust
pub enum ModelSelectorEvent {
    Consumed,
    Ignored,
    Emit(Action),
    Close,
    /// RPC-345: Tab keybind, pure UI navigation, no Action payload.
    /// Navigator translates it to ViewMode::ProviderSettings.
    /// TS analog: onSwitchToSettings() in ModelSelectorScreen.tsx:145.
    SwitchToProviders,
}
```

**(b) Add Tab arm in `handle_key`** — before the `_` catch-all at `mod.rs:287`:
```rust
KeyCode::Tab => ModelSelectorEvent::SwitchToProviders,
```
(mirror of `provider_settings/list.rs:62`)

**(c) Add Navigator translation arm** — `navigator_events.rs`, in
`handle_model_selector_event` after the `Emit` arm (~`:75`):
```rust
ModelSelectorEvent::SwitchToProviders => {
    if let Some(tx) = self.action_tx.as_ref() {
        let _ = tx.send(Action::OpenProviderSettingsView);
    }
    EventResult::consumed()
}
```
(mirror of the SwitchToModels arm `navigator_events.rs:45-50`)

**No new ViewMode or apply_action wiring needed:**
`Action::OpenProviderSettingsView` is already handled at `navigator.rs:111-113`
(`self.active_view = ViewMode::ProviderSettings`), and `ViewMode::ProviderSettings`
already exists (`navigator.rs:35`).

> Consider: only emit SwitchToProviders in NON-filter mode (mirror TS, where
> filter input is handled before the Tab arm). The Rust filter mode is handled in
> `handle_filter_key` (`mod.rs:196-223`); ensure Tab routing respects that so Tab
> while typing a filter does not navigate away unexpectedly.

### Reference summary
- TS: `ModelSelectorScreen.tsx:145` (+ `:36/:45`); `AgentView.tsx:5031-5034`, `:5050-5054`
- Rust forward stub: `provider_settings/mod.rs:69`, `provider_settings/list.rs:62`,
  `navigator_events.rs:45-50`, `navigator.rs:118-120`
- Rust missing back-dir: `model_selector/mod.rs:33-38`, `:225-288` (catch-all `:287`),
  `navigator_events.rs:57-77`
- Reused target: `navigator.rs:111-113` + `ViewMode::ProviderSettings` (`navigator.rs:35`)
