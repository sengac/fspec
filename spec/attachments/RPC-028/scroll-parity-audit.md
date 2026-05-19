# RPC-028 — Scroll, Mouse-Wheel & Wrap-Around Parity Audit

**Scope:** every selectable / scrollable view in `codelet/fspec-tui/src/` that is NOT `BoardView`.
**Reference implementations:** BoardView (RPC-016, RPC-023) and the TypeScript Ink source in `src/tui/components/`.
**Architecture authorities:** `spec/attachments/RPC-002/07-recommended-architecture.md`, `08-virtuallist-port-spec.md`, `09-dialog-and-input-priority-port-spec.md`, `10-multilineinput-and-mouse-port-spec.md`, and `rust-tui-parity-master-plan-2026-05-13.md`.

> The original problem report: after RPC-027 the SlashCommandPopup does not scroll when the user navigates past the visible region. The same defect (and missing mouse-wheel support, and missing wrap-around) exists in several sibling views. This document inventories the gaps and the exact fixes for each.

---

## 1. Executive summary

| View | Wrap-around | Viewport scroll | Mouse wheel | PgUp/PgDn/Home/End | Status |
|------|-------------|-----------------|-------------|--------------------|--------|
| BoardView (reference) | ✅ rem_euclid | ✅ per-column + ↑/↓ indicators | ✅ U/D/L/R | ✅ all four | OK |
| HelpDialog | n/a (static) | ❌ none | ❌ none | ❌ none | OK (short, fixed) |
| DisconnectDialog | n/a (static) | ❌ none | ❌ none | ❌ none | OK (short, fixed) |
| ThinkingLevelDialog | ✅ wraps | ❌ none (4 fixed rows) | ❌ none | ❌ none | OK (fixed list of 4) |
| ModelSelectorDialog | ✅ wraps (skips header rows) | ❌ **renders FULL list — no row windowing** | ❌ none | ❌ none | **BUG** |
| ConfirmDialog | ✅ cycles buttons | n/a | ❌ none | n/a | OK |
| **SlashCommandPopup** | ✅ wraps | ❌ **`.take(10)` hard cap, no scroll_offset — past index 9 is invisible** | ❌ none | ❌ none | **BUG (the reported defect)** |
| **FileSearchPopup** | ✅ wraps | ❌ **`.take(10)` hard cap, no scroll_offset** | ❌ none | ❌ none | **BUG** |
| ResumeSessionView | ❌ does wrap but the scroll-offset jump is buggy | ✅ has `scroll_offset` | ❌ none | ❌ none | **BUG (mouse + Pg keys)** |
| SearchHistoryView | ❌ does wrap but no PgUp/PgDn/Home/End | ✅ has `scroll_offset` | ❌ none | ❌ none | **BUG (mouse + Pg keys)** |

**Two distinct defect classes:**

1. **Viewport defect** — the SlashCommandPopup and FileSearchPopup paint at most 10 rows and have no `scroll_offset`. When the user moves selection to index ≥ 10 the selected row is off-screen. **This is exactly the bug the user reported.** ModelSelectorDialog also lacks row windowing; today it relies on the dialog body height growing to fit every row, which clips when the model list is taller than the terminal.
2. **Mouse-wheel defect** — only BoardView routes `Event::Mouse`. Every other selectable view ignores `MouseEventKind::ScrollUp`/`ScrollDown`. TS parity is partial: the TS source only mouse-scrolls the Resume picker, but the architecture master plan (RPC-002 §3.7 + RPC-023) extends mouse-wheel to *every* selectable list in the Rust port.

---

## 2. Architectural pattern (the contract every fix must satisfy)

These rules are extracted verbatim from `spec/attachments/RPC-002/`:

### 2.1 Viewport math (`08-virtuallist-port-spec.md`)

* Visible window is `items[scroll_offset .. scroll_offset + visible_height]`.
* `visible_height` is **constraint-based**, never dynamically measured. It is derived from the area `Rect` at render time.
* `ensure_selection_visible(visible_height)` runs **before** every render and on every selection mutation.
* PageUp / PageDown jump by exactly `visible_height`. Home = `navigate_to(0)`. End = `navigate_to(total - 1)`.
* A `last_area: Rect` (or, when only the body height matters, `last_viewport_height: Cell<u16>`) is captured at every render so subsequent key/mouse handlers can produce correctly-sized scroll steps.

### 2.2 Mouse routing (`07-recommended-architecture.md` + `10-multilineinput-and-mouse-port-spec.md`)

* `Event::Key` and `Event::Mouse` both flow through `compositor.handle_event(...)` — there is no separate mouse pipeline.
* Layers are walked **highest-priority first**, last-registered-first; the first `Consumed` wins.
* Components remember their last-rendered `Rect` and hit-test mouse events against it before handling.
* `MouseEventKind::ScrollUp`/`ScrollDown` map to `Action::SelectPrev`/`SelectNext` (or popup-local equivalents).
* `Down(Left)` triggers `MouseTrackingToggle::temporarily_disable_mouse()` (TUI-078) — a 5-second tokio timer re-enables capture. **Every** view that calls `crossterm::execute!(EnableMouseCapture, …)` must wire this toggle.
* Mouse-wheel acceleration (≥5 events within 150 ms → velocity cap 5; gap ≥150 ms → reset to 1) is mandatory for VirtualList; it is recommended for popups.

### 2.3 Wrap-around (`08-virtuallist-port-spec.md` + BoardView)

* VirtualList exposes `enable_wrap_around: bool`. When set, navigation wraps.
* BoardView is the canonical example: `proposed.rem_euclid(len_i)` produces a non-negative index regardless of `delta` sign or magnitude.
* `move_selection(delta, viewport_height)` ALWAYS calls `adjust_scroll_offset` after the wrap so the new selection is visible.

### 2.4 Component impl contract

Every selectable view must therefore expose:

```rust
pub fn render(&self, area: Rect, buf: &mut Buffer) { /* writes self.last_area */ }
pub fn handle_event(&mut self, event: &Event) -> EventResult { /* key + mouse */ }
fn move_by(&mut self, delta: i32, viewport_height: usize) { /* wrap via rem_euclid */ }
fn ensure_visible(&mut self, viewport_height: usize) { /* scroll_offset adjust */ }
```

(The exact names follow `BoardStore::move_selection` + `board_viewport::adjust_scroll_offset`.)

---

## 3. View-by-view audit + fix plan

### 3.1 SlashCommandPopup — `views/agent/slash_command_popup.rs`

**Current state**
- `selected_index: usize` wraps via `move_up`/`move_down`.
- `build_rows()` iterates `self.matches.iter().take(10)` — *every match past index 9 is invisible*.
- No `scroll_offset` field. No `last_area`. No mouse handling.

**Required fixes**
1. Add `scroll_offset: usize` and `last_visible_rows: Cell<usize>`.
2. Replace `iter().take(10)` with `iter().skip(scroll_offset).take(visible_rows)`. `visible_rows` is derived from the dialog body height at render time (the dialog frame already supplies it via `dialog_rect`).
3. Introduce `adjust_scroll(visible_rows)` mirroring `ResumeSessionView::adjust_scroll` (and `board_viewport::adjust_scroll_offset`).
4. Replace the bare `move_up`/`move_down` with `move_by(delta: i32, visible_rows: usize)` that uses `rem_euclid` for wrap-around AND calls `adjust_scroll`.
5. Add `handle_mouse(&mut self, ev: &MouseEvent, popup_rect: Rect)`:
   * `ScrollUp` → `move_by(-1, vr)` (Continued)
   * `ScrollDown` → `move_by(1, vr)` (Continued)
   * Outside `popup_rect` → `Ignored` (lets the click pass to the input)
6. Add PgUp / PgDn / Home / End:
   * PgUp → `move_by(-(visible_rows as i32), vr)`
   * PgDn → `move_by(visible_rows as i32, vr)`
   * Home → `selected_index = 0; scroll_offset = 0`
   * End → `selected_index = matches.len()-1; adjust_scroll(vr)`
7. Add `↑`/`↓` indicators inside the dialog body (top row when `scroll_offset > 0`, bottom row when `scroll_offset + visible_rows < matches.len()`) — same idiom as `viewport.rs:95-101`.
8. Snapshot regen: `slash_command_popup__centered_popup_80x24.snap` plus a new long-list snapshot.

### 3.2 FileSearchPopup — `views/agent/file_search_popup.rs`

**Current state** — identical defect to SlashCommandPopup:
- `iter().take(10)` hard cap.
- Empty-state literals `(type to search files)` and `(no files match "<filter>")` should remain unscrollable single-row affordances.

**Required fixes** — same as 3.1 with these specifics:
1. `scroll_offset: usize` + `last_visible_rows: Cell<usize>`.
2. Empty-state row is rendered regardless of scroll offset (no windowing applied).
3. Match list uses `iter().skip(scroll_offset).take(visible_rows)`.
4. `move_by(delta, vr)` with `rem_euclid`.
5. Mouse-wheel handler hit-tests `popup_rect`.
6. PgUp/PgDn/Home/End identical to 3.1.
7. Indicators `↑`/`↓` painted when applicable.

### 3.3 ModelSelectorDialog — `components/model_selector_dialog.rs`

**Current state**
- Renders the full `self.rows` vec; relies on the dialog growing to fit every row. When the model list exceeds terminal height the dialog clips.
- `move_up`/`move_down` already wrap and skip non-selectable header rows.
- No mouse, no Pg keys.

**Required fixes**
1. Add `scroll_offset: usize` and `last_visible_rows: Cell<usize>`. Determine `visible_rows` from `dialog_rect.height - footer_h - title_h - padding`.
2. In the render path (currently lines 137-164) window the rows: `rows.iter().enumerate().skip(scroll_offset).take(visible_rows)`.
3. `adjust_scroll` must keep `selected_index` inside the window AND avoid stopping on a non-selectable header row at the viewport edges (use the existing skip-non-selectable invariant when moving).
4. PgUp/PgDn jump by `visible_rows`; Home/End jump to first/last **selectable** row.
5. Mouse-wheel routed via the same hit-test pattern.
6. `↑`/`↓` indicators inside the dialog body.
7. Snapshot regen for `model_selector_dialog__*` (RPC-027 already requires this).

### 3.4 ResumeSessionView — `views/agent/resume_session_view.rs`

**Current state**
- Has `scroll_offset` and `adjust_scroll(visible_rows)`.
- `move_up` wraps with a scroll-offset jump (line 119-123). `move_down` wraps to 0 (line 134-136). Wrap-around exists but the math is bespoke and diverges from `rem_euclid` + `adjust_scroll`.
- No mouse handling.
- No PgUp/PgDn/Home/End.
- The `d`/`D` confirm dialog overlay is unaffected by this card.

**Required fixes**
1. Replace `move_up`/`move_down` with `move_by(delta: i32, vr: usize)` that uses `rem_euclid`. Keep the post-wrap `adjust_scroll(vr)` call.
2. Add PgUp/PgDn (jump by `vr`), Home/End.
3. Add mouse handling. `handle_mouse(&mut self, ev, body_rect, vr)`:
   * `ScrollUp`/`ScrollDown` → `move_by(±1, vr)`
   * `Down(Left)` inside a row → `selected_index = scroll_offset + row_in_body; emit Selected`
   * The TS reference (`AgentView.tsx:4435-4458`) supplies the 1×–5× velocity acceleration. Use the same `last_scroll_time: Cell<Option<Instant>>` + `scroll_velocity: Cell<u32>` pair.
4. The dialog `MouseTrackingToggle` (TUI-078) must be active so left-click temporarily releases mouse capture for native text selection.

### 3.5 SearchHistoryView — `views/agent/search_history_view.rs`

**Current state**
- Same shape as ResumeSessionView (own `scroll_offset` + `adjust_scroll`).
- Bespoke wrap math, no mouse, no Pg keys.

**Required fixes** — identical to 3.4. Care must be taken because the view consumes printable chars to extend the filter; mouse events and Pg keys must not be swallowed by the filter logic.

### 3.6 ThinkingLevelDialog — `components/thinking_level_dialog.rs`

**Status:** OK for now. The dialog has exactly 4 fixed rows and wraps. **Add mouse-wheel handling for consistency** so users on a trackpad can navigate.
* `ScrollUp` → `move_up()`, `ScrollDown` → `move_down()`.

### 3.7 ConfirmDialog — `views/agent/confirm_dialog.rs`

**Status:** OK. Horizontal button row, cyclic focus, no list.
* Optional ergonomic addition: left-click on a button focuses it; left-click on the focused button emits its outcome. This is **not required for parity** but is listed here for completeness.

### 3.8 HelpDialog & DisconnectDialog — `components/help_dialog.rs`, `components/disconnect_dialog.rs`

**Status:** OK. Static text, short, never overflows a reasonable terminal. No changes required unless the body grows past ~10 rows in a future card.

---

## 4. Shared helper module (proposed)

Several views will need identical viewport math. To keep file LoC ceilings (300) and prevent drift, introduce a new module:

`codelet/fspec-tui/src/components/scroll_viewport.rs`

```rust
/// Wrap-around step over `total` with rem_euclid semantics.
pub fn wrap_index(current: i32, delta: i32, total: usize) -> usize { ... }

/// Adjust `scroll_offset` so `selected` lies inside the window
/// `[scroll_offset, scroll_offset + visible_rows)`. Mirrors
/// `board_viewport::adjust_scroll_offset` but without the BoardView
/// two-pass arrow-correction (popups draw indicators inside the body,
/// not as separate gutter rows).
pub fn ensure_visible(
    scroll_offset: &mut usize,
    selected: usize,
    visible_rows: usize,
    total: usize,
);

/// SGR-wheel velocity acceleration, shared between views that opt in.
pub struct WheelVelocity { last: Cell<Option<Instant>>, vel: Cell<u32> }
impl WheelVelocity {
    pub fn step(&self, dir: Direction) -> i32 { ... }
}
```

This module is tested in isolation; the popups consume it instead of duplicating math.

---

## 5. Implementation order (smallest blast radius first)

1. `scroll_viewport.rs` + unit tests (wrap_index, ensure_visible, WheelVelocity).
2. SlashCommandPopup (the reported defect) — adds `scroll_offset`, `move_by`, mouse, Pg keys.
3. FileSearchPopup — identical changes, copy-paste-adapt.
4. ModelSelectorDialog — windowing + Pg keys + mouse. Snapshot regen.
5. ResumeSessionView — `move_by` + mouse + Pg keys.
6. SearchHistoryView — `move_by` + mouse + Pg keys.
7. ThinkingLevelDialog — mouse-wheel only.
8. (Optional) ConfirmDialog click-to-focus.
9. Regenerate every insta snapshot touched.

Each step ships with feature scenarios under `spec/features/rpc028-scroll-parity-*.feature` covering: wrap-around at both ends, scroll-offset advances past 10 rows, ↑/↓ indicators appear, PgUp/PgDn jumps by visible_rows, Home/End jump to ends, mouse-wheel up/down moves selection, mouse-wheel events outside the popup rect are ignored.

---

## 6. Acceptance criteria for the card

* `SlashCommandPopup` with > 10 matches: pressing Down 11 times scrolls the viewport and the 11th match is visible AND selected.
* The same popup wraps from the last match back to the first (and vice-versa for Up).
* Mouse wheel inside any popup/picker rect moves the selection up/down by 1 row.
* Mouse wheel outside the popup rect is ignored (no spurious scroll of the underlying view).
* PgUp/PgDn jumps by exactly the popup's visible-rows count.
* Home selects the first row (and scrolls); End selects the last row (and scrolls).
* The dialog body shows a `↑` glyph on its top content row when `scroll_offset > 0` and `↓` on the bottom content row when more rows lie below.
* No view file exceeds 300 LoC after the refactor.
* All insta snapshots updated; PR description enumerates each.

---

## 7. References (verbatim citations)

* **BoardView per-column scroll** — `codelet/fspec-tui/src/store/board_viewport.rs:33-47, 102-174`; `codelet/fspec-tui/src/views/board/viewport.rs:31-122`.
* **BoardView mouse** — `codelet/fspec-tui/src/views/board/mouse.rs:40-74`.
* **BoardView last_viewport_height** — `codelet/fspec-tui/src/views/board.rs:58-95, 134-151, 248-256`.
* **TS Ink wrap-around** — `src/tui/hooks/useSlashCommandInput.ts:156-174`, `src/tui/hooks/useFileSearchInput.ts:167-185`, `src/tui/components/ThinkingLevelDialog.tsx:98-110`.
* **TS Ink selection-centered scroll** — `src/tui/components/SlashCommandPalette.tsx:56-68`, `src/tui/components/FileSearchPopup.tsx:46-58`.
* **TS Ink mouse-wheel + acceleration (resume picker)** — `src/tui/components/AgentView.tsx:4435-4458, 4555-4568`.
* **VirtualList spec** — `spec/attachments/RPC-002/08-virtuallist-port-spec.md` lines 102-122 (visible_items), 246-275 (acceleration), 302-355 (keyboard), 357-401 (mouse + TUI-078), 403-444 (ensure_selection_visible), 558-559 (test plan).
* **Mouse subsystem** — `spec/attachments/RPC-002/10-multilineinput-and-mouse-port-spec.md` lines 276-310 (crossterm replacement), 318-365 (MouseTrackingToggle), 367-392 (hit-test helper).
* **Compositor dispatch** — `spec/attachments/RPC-002/07-recommended-architecture.md` lines 101-115, 184-193.
