# BOARD-022 — Design & RPC Rationale

## 1. Visual design (parity with the AgentView `@` file popup)

The dialog reuses the canonical `dialog_theme` renderer
(`components/dialog_theme.rs`, RPC-027) exactly as
`views/agent/file_search_popup.rs` does: cyan rounded border, bold cyan
inner title, `▸ ` selection marker, inverse cyan/black highlight on the
selected row, dim centered footer.

```
┌──────────────────────────────────────────┐
│ Search Work Units [id]                   │
│                                          │
│ ▸ AUTH-001                               │
│   AUTH-002                               │
│   AUTH-003                               │
│                                          │
│  ↑↓ Navigate │ Tab Mode │ Enter Select   │
└──────────────────────────────────────────┘
```

- Title row encodes the active mode: `[id]`, `[title]`, `[desc]`.
- Rows show the work-unit **id** (short, stable, unique). Title/description
  matching still selects by id; showing the id keeps rows narrow and
  unambiguous (the details strip already shows the title of the focused
  card, so the user gets the full context on selection).
- Scroll windowing reuses `components/scroll_viewport.rs`
  (`ensure_visible`, `wrap_index`, `WheelVelocity`) — the same helpers the
  file popup and `SearchHistoryView` use.
- Empty state row: `(no work units match "xyz")` / `(board is empty)` —
  non-selectable, exactly like `file_search_popup_rows.rs`.
- The dialog is centered over the board via `dialog_rect` (shrink-to-
  content, clamped to area) — no layout change to the board itself.

## 2. Why the search is client-side (the RPC question)

fspec's RPC layer exists to move data the TUI does not already have:

| Data | In TUI? | RPC needed? |
|------|---------|-------------|
| Work units (id/title/desc/status) | **Yes** — `BoardStore` snapshot, live-updated via `list_work_units` + `work_units_rx` broadcast (RPC-006) | **No** |
| Filesystem paths for `@` popup | No | Yes — `search_files` (RPC-020) |
| Session history for Ctrl+R | No | Yes — `persistence_search_history` (RPC-025) |

Adding a `search_work_units` RPC would:
1. duplicate data the client already holds (extra round-trip per keystroke
   over the WebSocket transport for a pure string filter),
2. widen the `FspecService` / `FspecBackend` surface with a method whose
   body is a trivial filter over `list_work_units` results,
3. create a new cross-transport parity test burden (RPC-020 convention)
   for behaviour that is transport-independent by construction.

**Decision:** the dialog filters the `BoardStore` snapshot in-process.
Freshness is inherited from the existing RPC-006 broadcast — if the board
updates while the dialog is open, the next open re-seeds from the fresh
snapshot (the dialog is seeded at open time, matching how
`AttachmentPickerDialog` seeds from the selected unit).

**Pinned by a source-shape test** (`tests/source_shape_board_search.rs`):
the substring `search_work_units` must NOT appear in
`rust/rpc/src/lib.rs`, `rust/fspec-tui/src/transport/mod.rs`,
`rust/fspec-tui/src/transport/embedded.rs`,
`rust/fspec-tui/src/transport/websocket.rs`.

## 3. Key-binding collision audit

`/` is currently unbound on the board (verified: no `KeyCode::Char('/')`
arm in `views/board.rs`; the only `/` handlers live in
`provider_settings/list.rs` and `model_selector/*`, which are separate
`ViewMode`s). The App-level stage-4 shortcuts are `?`, `Esc` (board exit
confirm), `Ctrl+D` — no conflict. The handler is modifier-free with the
same `!CONTROL` guard pattern as the `d`/`a`/`.` arms so `Ctrl+/`
(terminal toggle) falls through untouched.

## 4. Precedents followed

- **Modal-on-compositor pattern** — `AttachmentPickerDialog` (RPC-374):
  `Component` + `Priority::Foreground` + `remove_callback()` +
  idempotent push guarded by `compositor.contains(id)`.
- **Popup key routing** — `FileSearchPopup::handle_key` (RPC-020):
  Up/Down/PageUp/PageDown/Home/End + wrap + `ensure_visible`; `Esc`
  dismiss; modifier-guarded.
- **Dispatch helper file** — `app/dispatch_viewer.rs` (RPC-373/374):
  keeps `app/dispatch.rs` under the 300-LoC ceiling.
- **Store mutation via Actions** — BoardView never mutates `BoardStore`;
  it emits `Action`s that `App::dispatch` applies (RPC-009 single-task
  pattern). Selection therefore reuses the existing
  `BoardStore` mutation surface + one new `select_work_unit` helper.

## 5. Open questions (resolved by default, flag if wrong)

1. **Row content** — id only (chosen). Alternative: `ID — title…`
   truncated. Id-only keeps the dialog narrow and matches the "find by
   id/title/description, jump there" intent; the details strip shows the
   title after selection.
2. **Mode indicator** — in the title row (`[id]`). Alternative: a
   dedicated row. Title row avoids growing the dialog height.
3. **Enter with zero matches** — no-op (dialog stays open). Alternative:
   close. No-op is friendlier (user can keep typing).
4. **Query preserved across Tab** — yes (rule 2). The same text is
   re-matched against the new field, which is the least-surprising
   behaviour for a mode toggle.
