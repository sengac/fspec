# AST research — RPC-411 inline HITL prompt consumers and templates

AstGrep queries run against `codelet/fspec-tui/src` during discovery.

## 1. Construction sites of the old modal (to delete)

Pattern: `HitlDialog::new($$$ARGS)` (rust)

```
src/components/hitl_dialog.rs:407,426,441,448  — unit tests inside the deleted file
src/app/dispatch_pause_hitl.rs:158             — handle_open_hitl_dialog (compositor push)
```

Pattern: `Action::OpenHitlDialog { $$$FIELDS }` (construction)

```
src/app/dispatch_pause_hitl.rs:96 — handle_pause_chunk HITL arm (action_tx.send)
```

Other `HITL_DIALOG_ID` references (Grep): `src/lib.rs:54` re-export,
`src/components/mod.rs:29` mod decl, `src/app/dispatch_pause_hitl.rs:43,119,155,235`
(imports, `handle_pause_cleared` pop, idempotent-push guard, `handle_hitl_submitted` pop).
All must be removed/rewired to the per-session HITL slot.

## 2. RPC-406 template (per-session pause slot)

Pattern: `pub fn set_pause_state(&mut self, $$$ARGS) { $$$BODY }`

```
src/store/agent_view/pause_state.rs:40 — the slot mutator to mirror for hitl_state.rs
```

Store fields: `store/agent_view.rs:110-111` (`pause_state_by_session`,
`triple_pause_selection_by_session`) — new `hitl_prompt_by_session` sits beside them.

## 3. Key routing seam

Pattern: `fn handle_pause_prompt_key(&mut self, $KEY: &KeyEvent) -> Option<EventResult>`
did not match with full body wildcard binding across files; located via Grep at
`src/views/agent/pause_keys.rs:38` — consulted from `views/agent/dispatch.rs:85`
right after the KeyEventKind::Press filter. The new `handle_hitl_prompt_key`
(views/agent/hitl_keys.rs) must be consulted BEFORE it (HITL wins over pause).

## 4. Render seam

`views/agent/input_area.rs:81-87` — `paint_input_area` consults
`store.pause_state_for(sid)` before `paint_input_or_spinner` and caches
`last_pause`. The HITL slot check goes ABOVE the pause check (TS
InputTransition.tsx:385-388 priority); freeform/Other mode falls through to the
SHARED `MultiLineInput::render_with_prompt` with placeholder swapped.

`views/agent.rs:238` — `input_area_height` must account for the HITL prompt's
row count via the RPC-405 auto-grow seam.

## 5. Wire shapes (RPC-410, already landed)

`codelet/rpc-types/src/lib.rs:1089-1146` — `HitlOption`, `HitlQuestion{id,header,
question,options}`, `HitlRequest{questions}`, `HitlAnswer{id,selected,other}`,
`HitlResponse{cancelled,answers}`. No rpc-types changes needed for RPC-411.

## 6. Tests pinning old behavior (to rewrite)

- `tests/pause_hitl_rpc053.rs` — modal mount/hotkey/Tab/free-text/Esc-no-send tests.
- `tests/agent_input_paste_routing_rpc403.rs:283-337` — paste swallowed by modal.
- `tests/inline_pause_prompt_rpc406.rs:844-847` — "HITL wins on tie" asserts
  compositor layer HITL_DIALOG_ID; must assert the HITL slot instead.
- `spec/features/pause-and-hitl-dialogs.feature` — HITL-modal scenarios rewritten.
