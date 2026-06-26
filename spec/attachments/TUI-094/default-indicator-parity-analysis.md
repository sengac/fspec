# TUI-094 — Render the "(default)" indicator in the Rust /thinking dialog (TS parity)

## Problem statement

The TypeScript `/thinking` dialog marks which thinking level is the **persisted
default** by appending ` (default)` to that level's row. The Rust ratatui port
omits this entirely: the dialog has no awareness of the persisted default level,
so the user cannot see which level is currently the default while the dialog is
open. This is a read-side display gap (the `D` keybinding that *sets* the default
was already ported in RPC-027; only the *indicator* is missing).

Goal: achieve parity with the TS reference — the dialog row whose level equals the
persisted default renders a ` (default)` marker.

## TypeScript reference (target behavior)

`src/tui/components/ThinkingLevelDialog.tsx`:

- Dialog props (lines 51-62):
  ```tsx
  export interface ThinkingLevelDialogProps {
    /** Current base thinking level (used as initial selection) */
    currentLevel: JsThinkingLevel;
    /** Default thinking level for new sessions (null if not set) */
    defaultLevel: JsThinkingLevel | null;
    ...
  }
  ```
  Note `defaultLevel` is **separate** from `currentLevel` and is nullable.

- Per-row computation (line 129):
  ```tsx
  const isDefault = defaultLevel !== null && index === defaultLevel;
  ```

- Marker rendered into the description text (lines 140-144):
  ```tsx
  <Text dimColor={!isSelected}>
    {' - '}
    {option.description}
    {isDefault ? ' (default)' : ''}
  </Text>
  ```

So with default = High the High row renders:
`High - ~32K tokens, deep reasoning (default)`.

The `(default)` text rides on the **description** span (which is dimmed when the
row is not selected, via `dimColor={!isSelected}`).

## Rust port — current state (the gap)

`codelet/fspec-tui/src/components/thinking_level_dialog.rs`:

- Struct (lines 35-41) carries only `session_id`, `selected_index`,
  `action_tx`, `pending_action`. **No `default` field.**
- Constructor (line 46): `ThinkingLevelDialog::new(session_id, current_level)` —
  only the *current* level is passed in; the default is never threaded in.
- `render` (lines 170-186) builds each row with
  `label_description_row(label, desc, i == self.selected_index)` — knows only
  about selection, never default. No `(default)` text anywhere.

`codelet/fspec-tui/src/components/dialog_theme_rows.rs`:
- `label_description_row(label, description, selected)` has no notion of a
  default marker. The description span is dimmed when not selected (matches the
  TS `dimColor={!isSelected}`).

`codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:22-38`
(`handle_open_thinking_dialog`):
- Constructs the dialog passing only `current`. Does not load or pass the
  persisted default.

### Available data source (already exists from TUI-093)
`codelet_sessions::default_thinking_level_persistence::load_default_thinking_level_opt()`
returns `Option<ThinkingLevel>` — a direct analogue of the TS
`defaultLevel: JsThinkingLevel | null` (None = not set / invalid).

## Parity gap table

| Aspect | TypeScript | Rust (current) | Parity? |
|---|---|---|---|
| Dialog receives default level | `defaultLevel` prop (nullable) | not passed | NO |
| Struct field for default | (prop) | none | NO |
| Per-row isDefault check | `index === defaultLevel` | none | NO |
| `(default)` marker rendered | yes, appended to description | none | NO |
| Selection highlight | yes | yes (`selected_index`) | YES |
| `D` set-default keybinding | yes | yes (RPC-027) | YES |

## Proposed implementation (small, isolated)

1. **Dialog struct + constructor** (`thinking_level_dialog.rs`):
   - Add a field for the default, e.g. `default_index: Option<usize>` (the index
     in `LEVELS` matching the persisted default), OR store
     `default_level: Option<ThinkingLevel>` and compute the index at render time.
   - Add a builder `with_default_level(self, default: Option<ThinkingLevel>)`
     (mirrors the optional, nullable TS prop). Keep `new(session_id,
     current_level)` signature stable so existing call sites/tests compile, and
     thread the default via the builder (consistent with the existing
     `with_action_tx` pattern).
2. **Row rendering** (`thinking_level_dialog.rs` render + `dialog_theme_rows.rs`):
   - Append ` (default)` to the matching row's description, riding on the
     description span (so it is dimmed when the row is not selected — matching TS
     `dimColor={!isSelected}`). Prefer extending the row builder with an
     `is_default: bool` parameter OR adding a sibling builder
     `label_description_default_row(...)` so the existing callers
     (`ModelSelectorDialog`) are unaffected. Do NOT regress the existing
     `label_description_row` signature used elsewhere unless all callers are
     updated.
3. **Wiring** (`dispatch_model_thinking_dialogs.rs handle_open_thinking_dialog`):
   - Load the persisted default via
     `load_default_thinking_level_opt()` and pass it into the dialog via the new
     builder when constructing it.

## Acceptance behavior (parity)

- When a default level IS persisted, opening `/thinking` shows ` (default)` on
  exactly the row whose level equals the persisted default.
- When NO default is persisted (None), NO row shows ` (default)` (parity with TS
  `defaultLevel === null`).
- The `(default)` marker and the selection highlight (`▸`) are independent: the
  default row shows `(default)` whether or not it is the currently-highlighted
  row; the highlighted row shows `▸` whether or not it is the default.
- The `(default)` text is part of the (dimmable) description, dimmed when its row
  is not selected — matching the TS `dimColor={!isSelected}` styling.
- Existing behavior unchanged: selection highlight, `D` keybinding, Enter select,
  Esc close, arrow/mouse navigation.

## Non-goals / invariants
- Do NOT change storage location/encoding (`tui.defaultThinkingLevel`, 0-3).
- Do NOT change the `D` set-default flow or the TUI-093 restore/apply logic.
- Do NOT regress the insta snapshot test contract without intentionally updating
  the snapshot to reflect the new `(default)` marker (the existing snapshot uses
  default = Off-with-no-marker context; review whether the snapshot scenario sets
  a persisted default — if it does not, the snapshot output should be unchanged).
- Keep files under the 300-LoC ceiling (RPC-027 rule).

## Key files
- `codelet/fspec-tui/src/components/thinking_level_dialog.rs`
- `codelet/fspec-tui/src/components/dialog_theme_rows.rs`
- `codelet/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs`
- Data source: `codelet/sessions/src/default_thinking_level_persistence.rs`
  (`load_default_thinking_level_opt`)
- Reference: `src/tui/components/ThinkingLevelDialog.tsx` (lines 51-62, 129,
  140-144)
