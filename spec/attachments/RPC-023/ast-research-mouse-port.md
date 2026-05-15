# RPC-023 AST research — mouse-port landing surface

This file captures the AST analysis performed during the SPECIFYING
phase of RPC-023. It documents the existing code shape that the mouse
port plugs into so the implementation phase doesn't drift.

## 1 · Mouse capture is already wired but events are dropped

`AstGrep`:

```
pattern: EnableMouseCapture
hits:    codelet/fspec-tui/src/terminal.rs:19   (use)
         codelet/fspec-tui/src/terminal.rs:72   (execute!)
```

→ `TerminalGuard::init` already enables crossterm mouse capture; the
   matching `DisableMouseCapture` runs in `Drop` + the panic hook
   (terminal.rs:80-90). No other source file references it.

```
pattern: Event::Mouse($$$ARGS)
hits:    (none)
```

→ Confirms the gap RPC-023 closes. Nothing in the codebase currently
   matches mouse variants — every Event::Mouse delivered by crossterm
   is dropped because every `handle_event` impl matches Event::Key only.

```
pattern: Event::Key($$$ARGS)
hits:    src/app/events.rs:211     (synth_key helper)
         src/compositor.rs:198     (paste-stub synthesises Key events)
         src/compositor_tests.rs:117
```

→ The "drop site" is at:
  - `src/views/board.rs:96`  (BoardView::handle_event let-else)
  - `src/views/agent.rs`     (AgentView::handle_event let-else)
  - `src/components/disconnect_dialog.rs:110` (Event::Key match)
  - `src/components/help_dialog.rs:84`        (Event::Key match)

   The first two are addressed by this card (BoardView). The dialog
   files MUST stay Event::Key-only (Q5 decision) — enforced by a new
   source-shape test in `tests/source_shape_rpc023.rs`.

## 2 · `handle_event` impls in the workspace

```
pattern: fn handle_event($$$ARGS) -> EventResult { $$$BODY }
hits:    src/components/help_dialog.rs:83
         src/components/mod.rs:217       (trait default)
         src/components/disconnect_dialog.rs:109
         src/compositor_tests.rs:83
```

Plus the view-level impls (free fns, not on the Component trait):

- `src/views/board.rs:95   pub fn handle_event(&self, event, store)`
- `src/views/agent.rs      pub fn handle_event(&mut self, event)`
- `src/views/navigator.rs:68 pub fn handle_event(&mut self, event, store)`

The navigator already passes ANY `&Event` (Mouse included) to the
active view — no plumbing change there.

## 3 · The Action enum (BEFORE RPC-023)

`codelet/fspec-tui/src/components/mod.rs:86-192` — 33 variants.
RPC-023 appends three:

- `SetFocusedColumn(usize)`
- `SelectIndexInFocused(usize)`
- `ReEnableMouseTracking(String)`

## 4 · BoardStore mutation surface (RPC-016)

`codelet/fspec-tui/src/store/board_viewport.rs` already exposes:
- `move_selection(delta, viewport_height)`         — used by SelectNext/Prev
- `scroll_focused_column(delta, viewport_height)`  — used by PageUp/Down
- `select_first_in_focused()` / `select_last_in_focused()`

RPC-023 adds **one** new method: `select_index_in_focused(idx, viewport_height)`
that clamps and triggers the same `adjust_scroll_offset` helper used by
`move_selection`.

`BoardStore::set_focused_column(column: &str)` already exists at
`store/board.rs:128-132`. RPC-023 calls it via a new dispatch arm that
maps `Action::SetFocusedColumn(idx)` → `COLUMN_ORDER[idx]` → method.

## 5 · BoardView render → last-Rect persistence

`codelet/fspec-tui/src/views/board.rs:188-243` — the `render_with_store`
fn uses `Layout::default().constraints([…]).split(area)` to produce 11
vertical rows. `split[5]` is the column-header row; `split[7]` is the
content area. RPC-023 stores both Rects in `Cell<Option<Rect>>` fields
on `BoardView` and additionally slices `split[5]` and `split[7]`
horizontally by `column_width_at(idx, widths)` to build the
per-column rect arrays.

## 6 · `tokio::time::pause` availability

The crate already uses `tokio` workspace + `tokio-test` dev-dep — virtual
time is supported via `#[tokio::test(start_paused = true)]` or
`tokio::time::pause()`. The MouseTrackingToggle 5-second debounce timer
is testable through the same path used by RPC-011's reconnect backoff
tests (see `tests/auto_reconnect_slice2_rpc011.rs`).

## 7 · Source-shape policing pattern

`codelet/fspec-tui/tests/source_shape_rpc016.rs` is the template:
- Walks `src/` via `common::collect_rs_files`.
- Strips comments via `common::strip_rust_comments`.
- Asserts presence/absence of identifiers and substring patterns.
- Enforces the 300-LoC ceiling per file.

RPC-023 piggybacks on the same helpers via `tests/common/mod.rs` — no
new helper functions required.

## Summary

The mouse port plugs into a codebase that is already 90% ready:
- crossterm mouse capture is on,
- `Event::Mouse` arrives on the event stream,
- the Navigator forwards every event into BoardView::handle_event,
- RPC-016 already exposes the viewport-aware mutation surface
  (`move_selection`, `scroll_focused_column`) that wheel events delegate to,
- and `BoardStore::set_focused_column` is the existing click-target.

What's missing — the gap RPC-023 closes — is the **branch inside
BoardView::handle_event** that matches `Event::Mouse(MouseEvent { … })`,
plus the **hit-test helper** + **MouseTrackingToggle scaffolding** for
RPC-019.
