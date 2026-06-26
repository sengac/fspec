# TUI-095 — Live-update `(default)` marker when `D` is pressed in the Rust `/thinking` dialog

## Summary

TUI-094 added the `(default)` marker to the Rust `ThinkingLevelDialog`, but it only
threads the persisted default **at dialog-open time** (`handle_open_thinking_dialog` →
`with_default_level(load_default_thinking_level_opt())`). When the user presses **`D`**
*inside the open dialog*, the marker does **not** move to the selected row — it stays on
the previously-persisted default until the dialog is closed and reopened.

This diverges from the TypeScript reference, where pressing `D` updates the marker
**immediately** (live).

## TS reference behaviour (the parity target)

`src/tui/components/ThinkingLevelDialog.tsx`:

```tsx
// D key — set current selection as default (dialog stays open)
if (input.toLowerCase() === 'd') {
  onSetDefault(selectedIndex as JsThinkingLevel);
  return true; // Consumed (dialog stays open)
}
...
const isDefault = defaultLevel !== null && index === defaultLevel;
```

`src/tui/components/AgentView.tsx` (parent, lines ~5662-5678):

```tsx
<ThinkingLevelDialog
  currentLevel={...}
  defaultLevel={defaultThinkingLevel}      // <-- parent state
  onSetDefault={async level => {
    await setDefaultThinkingLevel(level);   // <-- updates defaultThinkingLevel state
  }}
/>
```

When `D` is pressed:
1. `onSetDefault(selectedIndex)` runs → the `setDefaultThinkingLevel` hook updates the
   parent `defaultThinkingLevel` state.
2. React re-renders `ThinkingLevelDialog` with the **new** `defaultLevel` prop.
3. `isDefault = index === defaultLevel` now matches the newly-selected row → the
   `(default)` marker moves live, **while the dialog stays open**.

This is asserted by the TS test
`src/tui/components/__tests__/TUI-058-thinking-level-dialog-behavior.test.tsx`,
scenario **"Default indicator moves when D key is pressed"** (lines 207-258): after
pressing `d` on the High row and re-rendering with the updated `defaultLevel`, the High
row shows `(default)` and Medium no longer does.

## Rust root cause

`codelet/fspec-tui/src/components/thinking_level_dialog.rs`, the `D` key handler:

```rust
KeyCode::Char('d') | KeyCode::Char('D') => {
    let level = self.selected_level();
    let action = Action::SetThinkingLevelDefault(self.session_id.clone(), level);
    self.emit_action(action);
    return EventResult::consumed();   // <-- never updates self.default_index
}
```

The dialog owns its `default_index: Option<usize>` field and renders the marker from it
(`render()` → `Some(i) == self.default_index`). The `D` handler emits the persistence
action but **never updates `self.default_index`**, so the next `render()` still marks the
old row. Unlike the TS parent→prop→re-render loop, the Rust dialog is the source of
truth for what it draws *while open*, so it must update its own field.

`handle_set_thinking_level_default` (in `dispatch_model_thinking_dialogs.rs`) persists the
value and refreshes the session badge, but it does **not** (and should not need to) reach
back into the open dialog component — the dialog must self-update for a live marker.

## The fix

In the `D` key handler, update the dialog's own `default_index` to the currently selected
row **before/after** emitting the action, so the very next render reflects the new
default:

```rust
KeyCode::Char('d') | KeyCode::Char('D') => {
    let level = self.selected_level();
    self.default_index = Some(self.selected_index); // live marker move (TS parity)
    let action = Action::SetThinkingLevelDefault(self.session_id.clone(), level);
    self.emit_action(action);
    return EventResult::consumed();
}
```

### Why `Some(self.selected_index)` (not recompute from `level`)
`selected_index` is already the row the marker should land on, and `level ==
LEVELS[selected_index].0`, so they are equivalent. Using `selected_index` directly is the
simplest and avoids a redundant lookup.

## Behaviour that MUST be preserved (do not regress)

- `D` does **not** close the dialog (parity: "Consumed (dialog stays open)").
- `D` still emits `Action::SetThinkingLevelDefault(session_id, selected_level)` so the
  value is persisted via the backend (TUI-093 wiring) — the live marker update is *in
  addition to*, not a replacement for, the action.
- Enter/Esc/Up/Down/mouse-wheel behaviour unchanged.
- Open-time threading via `with_default_level(load_default_thinking_level_opt())`
  unchanged (TUI-094).
- The `(default)` text still rides the dimmable description span (DIM when row not
  selected). Note: after pressing `D`, the new default row **is** the selected row, so its
  `(default)` is rendered on the highlighted (non-dim) description — matching TS, where the
  selected row's description uses `dimColor={!isSelected}` = not dimmed.

## Acceptance criteria (Example Mapping)

**Rules**
1. Pressing `D` updates the dialog's `default_index` to the currently selected row so the
   `(default)` marker moves to that row on the next render (no reopen required).
2. Pressing `D` still emits `Action::SetThinkingLevelDefault(session_id, selected_level)`
   and keeps the dialog open (existing behaviour preserved).
3. After `D`, the previously-default row no longer shows `(default)`; only the newly
   selected row shows it.
4. Navigating with arrows after `D` moves only the selection highlight; the `(default)`
   marker stays on the row chosen by the last `D` press until `D` is pressed again.
5. `D` on a row that is already the default is idempotent — the marker stays on that row.

**Examples**
1. Default is Medium, user navigates to High and presses `D` → High row now reads
   `(default)`, Medium no longer does (single render, dialog still open).
2. No default set, user selects Low and presses `D` → Low row shows `(default)`.
3. Default is High and High is selected, user presses `D` → High still shows `(default)`
   (idempotent).
4. After pressing `D` on High, user presses Down to Off → Off is highlighted but High
   still carries `(default)`.

## Test strategy (Rust, Vitest-equivalent = Rust integration test)

Add scenarios to a new/extended integration test (e.g.
`codelet/fspec-tui/tests/tui095_live_default_marker.rs`) that:
- Builds a `ThinkingLevelDialog` with a known default, navigates, sends
  `KeyCode::Char('d')`, renders to an 80x24 `TestBackend`, and asserts the `(default)`
  marker is on the newly selected row and absent from the old default row — all without
  reopening.
- Asserts `D` still produces `Action::SetThinkingLevelDefault(..)` (via
  `take_pending_action`) and that `handle_event` returns `Consumed` (dialog stays open).

## Touch points

| File | Change |
|------|--------|
| `codelet/fspec-tui/src/components/thinking_level_dialog.rs` | Set `self.default_index = Some(self.selected_index)` in the `D` handler |
| `codelet/fspec-tui/tests/tui095_live_default_marker.rs` (new) | Live-marker integration tests with `@step` comments |
| `spec/features/<capability>.feature` (new, `@TUI-095`) | Generated scenarios |

No changes needed to `dispatch_model_thinking_dialogs.rs`, `dialog_theme_rows.rs`, or the
persistence layer.
