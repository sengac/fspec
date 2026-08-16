# TUI-106 AST research — shared LoadingDialog base

Structured summary of the code AST touched by TUI-106 (companion to
TUI-106-shared-loading-dialog-research.md). Recorded 2026-08-16 from
ripgrep/ast reads of `rust/fspec-tui`.

## Lifted source: views/agent/spinner.rs (RPC-095)

- `DOTS_FRAMES: [&str; 10]` (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏)
- `DOTS_INTERVAL_MS: u64 = 80`
- `current_frame_glyph(elapsed_ms: u64) -> &'static str` — `(ms / 80) % 10`
- `paint_spinner_line(area, buf, frame_index, message, hint)` — DIM Paragraph

Callers (must keep compiling via re-export):
- views/agent/input_transition.rs:12 (`use super::spinner::{…}`), :229
- views/agent/transition_driver.rs:10 (`use super::spinner::current_frame_glyph`)
- views/agent.rs:57 `pub mod spinner;`

Inline unit tests: 6 (`frames_count_is_ten`, `interval_is_eighty_ms`,
`frame_picker_modulus_wraps`, `painter_writes_spinner_glyph_at_origin`,
`painter_applies_dim_modifier`, `painter_respects_area_origin`).

## Dialog base (untouched, reused): components/dialog_theme.rs (RPC-027)

- `Accent { Cyan, Yellow, Red }`, `FspecDialog { accent, title, rows, footer, min_width }`
- `render_dialog(area, buf, &dialog)` / `render_dialog_at(rect, …)`
- Precedent users: status_dialog.rs (Cyan "… Files" rows [current, "(idx/total)"]),
  checkpoint_restore_dialog.rs (Yellow, `min_width: 40`).

## Mode views (gaining `loading: Option<LoadingDialog>` + `load: LoadTracker` + `is_loading()`)

- views/checkpoints/mod.rs (310 LoC): stale-drop keys
  `files_key: Option<(String,String)>`, `diff_key: Option<(String,String,String)>`
  (set_files/set_diff drop mismatched results — behavior preserved, tracker
  `complete_stage(key)` is a no-op on mismatch).
- views/changed_files/mod.rs (286 LoC): `diff_path: Option<String>` stale-drop in
  `set_diff`.

## Cascade dispatch (feeds the tracker)

- app/dispatch_checkpoints.rs: stage 1 `list_checkpoints()` → `CheckpointsLoaded`;
  stage 2 `checkpoint_diff_files` → `CheckpointFilesLoaded`;
  stage 3 `checkpoint_file_diff` → `CheckpointFileDiffLoaded`.
- app/dispatch_changed_files.rs: stage 1 `changed_files()` → `ChangedFilesLoaded`;
  stage 2 `file_diff` → `FileDiffLoaded`.
- Spawn sites stay the ONLY tokio::spawn sites (action bus invariant).

## Redraw gate chain

- app/mod.rs:84 `tick_should_draw(should_render, is_busy, is_animating)` + test
  module (4 cases → extended with 5th operand).
- app/events.rs ~238-243: `_ = tick.tick()` arm calls `tick_should_draw(…)` on
  `self.should_render / self.is_session_busy() / self.is_input_animating()`.
- app/state.rs: `is_session_busy()` (210-216), `is_input_animating()` (222-224)
  delegate pattern → new `is_view_loading()` delegates to
  `self.navigator.is_view_loading()`.
- views/navigator.rs (299 LoC) — `is_view_loading` placed in
  views/navigator_events.rs (221 LoC, sibling `impl Navigator` block) to keep
  navigator.rs under the 300-LoC ceiling; matches on
  `self.active_view` (Checkpoints → self.checkpoints.is_loading(),
  ChangedFiles → self.changed_files.is_loading(), _ → false).

## Budget check (300-LoC ceiling)

| File | Now | +TUI-106 |
|---|---|---|
| views/navigator.rs | 299 | 0 (method → navigator_events.rs) |
| views/navigator_events.rs | 221 | +~14 (is_view_loading) = ~235 |
| app/state.rs | 314 | +~6 (is_view_loading) = ~320 (already over; minimal delta, flagged) |
| app/mod.rs | 108 | +~4 (gate signature/doc) + test cases |
| app/events.rs | 274 | +1 (gate call-site operand) |
| views/checkpoints/mod.rs | 310 | +~8 (fields + is_loading + tracker key builders) |
| views/changed_files/mod.rs | 286 | +~5 (fields + is_loading) |
| components/spinner.rs (new) | — | ~120 (moved byte-for-byte) |
| components/loading_dialog.rs (new) | — | ~180 (struct + render + tests) |
| components/load_state.rs (new) | — | ~160 (tracker + stage keys + tests) |

## Test anchors

- PROV-104 state-discriminator precedent: tests in
  views/model_selector/tests_loading_empty.rs (loading vs empty via
  `providers_loaded()` state, not pixels).
- StatusDialog rule [7] anchor: status_dialog.rs `handle_event` — ESC while
  Restoring returns `EventResult::ignored()` (components/status_dialog.rs:201-221).
- dialog buffer render pattern: checkpoint_restore_dialog.rs tests module
  (TestBackend::new(80,24) + row-joined text asserts).
