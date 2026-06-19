# RPC-345 — AST research: Tab to Provider Settings parity

AST/grep analysis confirming the surgical change surface for adding the
reciprocal Tab keybind (ModelSelector to ProviderSettings).

## 1. ModelSelectorEvent enum (target for new variant)
AstGrep rust 'pub enum ModelSelectorEvent { $$$VARIANTS }'
=> codelet/fspec-tui/src/views/model_selector/mod.rs:33

Current variants (mod.rs:33-38): Consumed | Ignored | Emit(Action) | Close.
No SwitchToProviders => MUST add it.

## 2. handle_key (target for new Tab arm)
AstGrep rust 'pub fn handle_key(&mut self, key: KeyEvent) -> ModelSelectorEvent { $$$BODY }'
=> mod.rs:289

- Filter mode is short-circuited at the top:
  `if self.filter_mode { return self.handle_filter_key(key); }` (mod.rs:290-292).
  => Tab while filtering is owned by handle_filter_key, whose `_ => Consumed`
     keeps the selector open (mod.rs:285). No navigation. Matches rule #4.
- Non-filter match block (mod.rs:299-354) has no KeyCode::Tab arm; Tab falls
  into `_ => ModelSelectorEvent::Consumed` (mod.rs:353).
  => MUST add `KeyCode::Tab => ModelSelectorEvent::SwitchToProviders` before catch-all.
- Esc arm: `KeyCode::Esc => ModelSelectorEvent::Close` (mod.rs:300) — unaffected
  by the new arm. Matches rule #5 / example #4.

## 3. Forward-direction reference (mirror source)
- grep Tab provider_settings/list.rs => list.rs:62
  `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels` (filter_mode falls through, list.rs:54-62).
- grep SwitchToModels provider_settings/mod.rs => mod.rs:69 (enum variant).

## 4. Navigator translation (target for new arm)
handle_model_selector_event (navigator_events.rs:57-77) currently matches only
Consumed | Ignored | Close | Emit(action). The sibling
handle_provider_settings_event has the reference arm:
`ProviderSettingsEvent::SwitchToModels => send Action::OpenModelSelectorView; consumed()`
(navigator_events.rs:45-50).
=> MUST add
`ModelSelectorEvent::SwitchToProviders => send Action::OpenProviderSettingsView; consumed()`.

## 5. Reused target (no new wiring needed)
grep OpenProviderSettingsView|ViewMode::ProviderSettings navigator.rs:
- navigator.rs:111-112 `Action::OpenProviderSettingsView => self.active_view = ViewMode::ProviderSettings`
- navigator.rs:93 dispatches ProviderSettings events; ViewMode::ProviderSettings already exists.
- components/mod.rs:610 OpenProviderSettingsView action variant already defined.

## Conclusion
Three edits, all mirrors of the existing forward path: (a) enum variant,
(b) Tab arm in handle_key, (c) navigator translation arm. No new ViewMode,
Action, or apply_action wiring required.
</parameter>
</invoke>
