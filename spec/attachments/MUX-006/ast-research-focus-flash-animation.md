# AST research — MUX-006 (focus flash animation on pane selection)

Research notes for the 1-second purple background animation painted over the
entire currently-selected mux pane. Equivalent AST/ripgrep searches were run
directly (the `fspec research --tool=ast` command is not yet ported to the
Rust binary).

## 1. Mux architecture (target of the change)

- `rust/fspec-tui/src/views/multiplex/mod.rs` — `MultiplexLayout` mutation API
  (`enable_default`, `enable_with_config`, `disable`, `set_focus`,
  `set_pane_count`, `set_pane_list`, `set_orientation`, `finish_drag`,
  `begin_drag`, `update_drag`). `pub(super) focus: MuxFocus` (a `usize`) is
  the focused pane index.
- `rust/fspec-tui/src/views/multiplex/types.rs` — `MultiplexLayout` struct
  fields (focus, rendered_panes, body_area, …). NOT persisted — live state
  only. `MuxConfig` IS persisted (orientation, splits, panes, focused_pane,
  enabled) via `store/mux_state.rs` → `tui.mux` in the shared
  `fspec-config.json`.
- `rust/fspec-tui/src/views/multiplex/window.rs` — MUX-002 agent-window
  math; `shift_left`/`shift_right`/`focus_prev`/`focus_next`/
  `note_session_created` all mutate `focus` directly.
- `rust/fspec-tui/src/views/multiplex/render.rs` — `render_with_stores`
  (entry point, called by `Navigator::render_with_stores` in
  `views/navigator.rs:420-434`). Short-circuits when
  `!layout.config.enabled || area.height < 3 || area.width < 2`. Computes
  `body` (area minus footer row), pane rects, divider rects, then renders
  each pane into its rect, then `paint_dividers` + `paint_footer`.
  `MUX_FOOTER_BG: Color = Color::Rgb(74, 44, 112)` (line 22) is THE purple
  from the last commit (MUX-005) — reuse it for the flash.
- `rust/fspec-tui/src/views/multiplex/mouse.rs` — `classify_mouse` →
  `MouseDecision::Pane { index }` → `Navigator::handle_mux_event`
  (`views/navigator.rs:179-187`) calls `self.mux.set_focus(index)`.
- `rust/fspec-tui/src/views/multiplex/keys.rs` — `classify_key` →
  `KeyDecision::FocusPrev/FocusNext` → `Navigator::handle_mux_event`
  (`views/navigator.rs:212-234`) calls `self.mux.shift_left()` /
  `shift_right()` (the Navigator-level fallback; while mux is enabled the
  App-level handler below intercepts first).

### Focus-mutation sites (EXHAUSTIVE list, verified by grep)

| Site | File:line | Trigger |
|------|-----------|---------|
| `MultiplexLayout::set_focus` | multiplex/mod.rs:122 | click-to-focus (navigator.rs:180), `handle_mux_on` (dispatch_mux.rs:196), `BackToBoard` (navigator.rs:332), `MuxEnterWorkUnit` (navigator.rs:375) |
| `MultiplexLayout::shift_left` | window.rs:140 | Shift+Left (App-level events.rs:114 → dispatch_mux.rs:261; Navigator fallback navigator.rs:216) |
| `MultiplexLayout::shift_right` | window.rs:120 | Shift+Right (App-level dispatch_mux.rs:255; Navigator fallback navigator.rs:222) |
| `MultiplexLayout::note_session_created` | window.rs:157 | session created after new-agent prompt (dispatch_session_cycle.rs / dispatch_create_session_dialog.rs call sites) |
| `MultiplexLayout::set_pane_count` | mod.rs:131-142 | `/mux N` — re-homes focus to last pane |
| `MultiplexLayout::set_pane_list` | mod.rs:149-171 | `/mux board agent 40` — clamps focus |
| `MultiplexLayout::enable_default` | mod.rs:65-80 | fresh `/mux` entry (focus = 0) |
| `MultiplexLayout::enable_with_config` | mod.rs:90-104 | dialog commit / `/mux default` (focus = config.focused_pane) |
| `MultiplexLayout::disable` | mod.rs:107-120 | `/mux off` — focus reset to 0 (mux OFF; no flash needed) |
| `recompute_effective_panes` | window.rs:212-232 | window clamp — `focus = focus.min(n-1)` only when pane list shrinks |

**Design consequence:** the cleanest trigger point is a single
`MultiplexLayout::flash_focus(&mut self)` helper called from a private
`set_focus_internal(prev: MuxFocus)` (or by comparing old/new focus at the
public mutation sites). Simplest robust approach: a `prev_focus` field —
`render_with_stores` (or every focus mutation) compares `focus` vs
`flash_focus`; when they differ AND mux is enabled, (re)arm the flash with
`focus` as the flash pane index and reset the flash clock to 0.

Chosen design: `MultiplexLayout` owns the flash state
(`flash: Option<MuxFocus>` + `flash_clock_ms: u64`); the public mutators
`set_focus` / `shift_left` / `shift_right` / `note_session_created` /
`enable_default` / `enable_with_config` / `set_pane_count` / `set_pane_list`
call a private `bump_flash(next_focus)` that re-arms the 1s window when the
focus actually changes (old != new, mux enabled, pane index valid). The
render path (`render_with_stores`) advances `flash_clock_ms += 16` per
frame (render-driven clock, same pattern as
`AgentView::animation_clock_ms` in `views/agent.rs:126` +
`views/agent/animation.rs:32`) and clears the flash when
`flash_clock_ms >= FLASH_MS` (1000ms).

## 2. Run-loop draw gate (why a 5th `tick_should_draw` operand is needed)

- `app/mod.rs:96` — `pub fn tick_should_draw(should_render, is_busy,
  is_animating, is_view_loading) -> bool` (currently 4 bools; pure, has a
  5-case unit test module at `app/mod.rs:106-130`).
- `app/events.rs:297-332` — the 16ms `RENDER_TICK` arm:
  `let is_busy = self.is_session_busy(); let is_animating =
  self.is_input_animating(); let is_view_loading = self.is_view_loading();
  if super::tick_should_draw(...) { draw }`.
- Precedent: TUI-106 added the 4th operand exactly for this reason (see
  `spec/attachments/TUI-106/ast-research-TUI-106.md`: "Prefer adding a 4th
  boolean to `tick_should_draw` … over a per-view `tokio::time::interval`
  — the ~60fps render tick already exists"). MUX-006 adds the 5th:
  `is_mux_flash_active`.
- Chain: `MultiplexLayout::is_flash_active()` → `Navigator::is_mux_flash_active()`
  (match on `active_view == ViewMode::Mux`) → `App::is_mux_flash_active()`
  → 5th `tick_should_draw` operand.
- **External callers of `tick_should_draw` that MUST be updated (5 args):**
  - `rust/fspec-tui/src/app/events.rs:307` (the run loop)
  - `rust/fspec-tui/src/app/mod.rs:106-130` (5 inline tests)
  - `rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs`
    (lines ~75-100, ~499-535 — 4 call sites, all `false, false`-style
    trailing args)
  - `rust/fspec-tui/tests/tui106_loading_dialog.rs` (lines ~240-262 — 6 call
    sites)
- **No tokio timer in the view** (SSR: the run loop owns the clock; the view
  only reports state — TUI-106/107 architecture decision).

## 3. Render pipeline details

- `Navigator::render_with_stores` (`views/navigator.rs:386-436`) dispatches
  `ViewMode::Mux` → `mux_render::render_with_stores(&mut self.mux, area, buf,
  board_store, agent_store, &mut MuxRenderViews {...})`.
- Inside `mux_render::render_with_stores` (`render.rs:34-114`): panes render
  FIRST (board/agent/files/checkpoints into their rects), then
  `paint_dividers`, then `paint_footer`. The flash must be painted AFTER the
  pane content (over it) but BEFORE dividers/footer (or after — dividers sit
  in the 1-cell gaps and the footer row is outside pane rects, so painting
  after pane content and before dividers is correct; the flash never touches
  divider columns or the footer row).
- Pane rects come from `layout.pane_rects` (absolute coords, same buffer —
  no translation needed; views use absolute coordinates per the module doc).
- Flash rect for pane i = `rects[i]` — the ENTIRE pane area (header through
  input for agent panes). The flash paints only BACKGROUND of existing
  cells (style .bg(purple)), never symbols — content stays readable.

## 4. Existing animation patterns to mirror

- `views/agent/input_transition.rs` — state machine driven by an absolute
  `clock_ms` (pure `advance(clock_ms)`), `INK_FRAME_TIME_MS = 17`.
- `views/agent/animation.rs` — `tick_animation` bumps
  `animation_clock_ms += 16` per frame (render-driven, NOT wall-clock).
- `views/agent/spinner.rs` — `current_frame_glyph(elapsed_ms)` picks frame
  by `(elapsed / INTERVAL) % FRAMES`.
- `components/loading_dialog.rs` — braille spinner at 80ms cadence while
  `tick_should_draw` gate is open.
- Deterministic PRNG for rain columns: a simple `xorshift`-style u64 hash of
  `(column, frame)` — pure function, testable, no `rand` crate dependency
  (check Cargo.toml: fspec-tui does not depend on `rand`; `proptest` is
  dev-only).

## 5. Colors / theme

- `MUX_FOOTER_BG: Color = Color::Rgb(74, 44, 112)` (`render.rs:22`) — the
  "purple from the last commit" (MUX-005). Reuse the same constant for the
  flash background so the flash visually matches the footer bar.
- Cell write pattern (from MUX-005 tests): after panes paint, do
  `buf[(x, y)].set_style(Style::default().bg(MUX_FOOTER_BG))` on the
  selected cells — `set_symbol` must NOT be called (it would blank the
  content); `set_style` alone preserves the existing symbol. Verify in
  ratatui: `Cell::set_style` returns `&mut Cell` and only mutates fg/bg/
  modifiers (symbol untouched) — yes, ratatui's `Cell::set_style` keeps the
  symbol.

## 6. Test infrastructure

- Mux render tests use `ratatui::backend::TestBackend::new(120, 24)` +
  `Terminal::draw` + `nav.render_with_stores(frame.area(), frame.buffer_mut(),
  board, agent)` (see `tests/mux005_footer_styling.rs` helpers `fresh()`,
  `enable_default()`, `seed_agent_session()`, `render_buffer()`).
- Focus changes in tests: `nav.mux.set_focus(i)` directly (public), or
  `nav.handle_event(&Event::Key(KeyEvent::new(KeyCode::Left,
  KeyModifiers::SHIFT)), &board)` (Navigator-level fallback path — note the
  App-level handler is the real one, but the Navigator fallback exercises
  the same `shift_left`), or mouse click `nav.handle_event(&click(x, y),
  &board)` (click-to-focus path).
- Buffer style assertions: `buf[(x, y)].bg == MUX_FOOTER_PURPLE`
  (`Color::Rgb(74, 44, 112)`) — exact `Color` equality works (see
  mux005 test lines 104-112).
- New test file: `rust/fspec-tui/tests/mux006_focus_flash.rs`.

## 7. Risk / compatibility notes

- R10 from rust-mux-mode.feature: "existing single-view behavior is
  byte-for-byte unchanged when mux is off" — the flash must only run when
  `mux.config.enabled && active_view == ViewMode::Mux`; with mux disabled,
  `is_flash_active()` returns false and no purple cells are painted.
- 300-LoC file ceiling: `render.rs` is currently 172 lines → adding a
  `paint_focus_flash` fn (≤ ~50 lines) keeps it under 300. `mod.rs` is ~241
  lines → focus-bump helpers (≤ ~30 lines) stay under 300. If either
  overflows, split flash state into a new `flash.rs` module.
- `MultiplexLayout` has NO `Clone`/`Default`-derive beyond manual impl —
  new fields need updating in `new()` (types.rs:95-118) only.
- `MuxConfig` serde shape is UNCHANGED (flash state is live-only, not
  persisted) → no persistence migration concerns, no changes to
  `store/mux_state.rs` / `codelet-sessions::mux_config_persistence`.
- proptest: a property test over the flash painter (every cell painted is
  inside the focused pane rect; no cell outside is painted; deterministic
  for a given (clock, rect)) fits the workspace's proptest-for-parsing-
  and-serialization logic guidance — the pure `frame_pattern` function is
  the natural proptest target.
- `tick_should_draw` signature change (4→5 args) is public API of
  `codelet_fspec_tui::app` — the two integration-test files calling it with
  4 args must be updated in the same commit (verified list in §2).
