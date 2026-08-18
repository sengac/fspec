# AST research — TUI-107 checkpoints loading wiring

Analyzed via AST search over `rust/fspec-tui/src` (functions/structs relevant to the
CheckpointsView loading-dialog wiring).

## Key entities

| Entity | Location | Role |
|---|---|---|
| `CheckpointsView` (struct) | `views/checkpoints/mod.rs:68` | three-pane state; already holds `loading: LoadingDialog` + `load: LoadTracker` (TUI-106) |
| `CheckpointsView::new` | `views/checkpoints/mod.rs:118` | mounts `LoadingDialog::new("Loading checkpoints", "Loading checkpoint list…")` + `LoadTracker::new(...)` |
| `CheckpointsView::is_loading` | `views/checkpoints/mod.rs:155` | delegates to `load.is_loading()` |
| `CheckpointsView::sync_loading_label` | `views/checkpoints/mod.rs:162` | copies `load.active_label()` onto `loading.label` |
| `CheckpointsView::set_checkpoints` | `views/checkpoints/mod.rs:170` | folds list result; resets selection; does NOT yet mark the tracker flushed (dispatcher does) |
| `CheckpointsView::set_files` | `views/checkpoints/mod.rs:193` | stale-drop via `selection_matches`; no tracker interaction (dispatcher calls `complete_stage`) |
| `CheckpointsView::set_diff` | `views/checkpoints/mod.rs:206` | stale-drop via selection + file path match |
| `CheckpointsView::render` | `views/checkpoints/render.rs:34` | paints panes; `render_empty` when `checkpoints.is_empty()` (render.rs:129) — the fake empty state; NO loading-dialog paint branch yet |
| `CheckpointsView::handle_key` | `views/checkpoints/keys.rs:22` | guards: ctrl/alt → Ignored; restore dialog; delete dialog; ESC → Close. NO loading guard yet |
| `CheckpointsView::handle_mouse` | `views/checkpoints/keys.rs:61` | prepend-guard only for restore/delete dialogs (keys.rs:64) — must extend for loading |
| `LoadingDialog` | `components/loading_dialog.rs:35` | TUI-106 shared dialog value (title/label/progress) |
| `render_loading_dialog` | `components/loading_dialog.rs:83` | paints via `dialog_theme::render_dialog` (cyan accent, rounded border) |
| `LoadTracker` | `components/load_state.rs:30` | staged in-flight marker; `begin_stage`/`complete_stage`/`mark_list_flushed`/`active_label` |
| `current_frame_glyph` | `components/spinner.rs:48` | braille glyph from elapsed ms (80 ms cadence) |
| `App::handle_checkpoints_loaded` | `app/dispatch_checkpoints.rs:52` | `mark_list_flushed` + `begin_stage(files…)` + `sync_loading_label` |
| `App::handle_checkpoint_files_loaded` | `app/dispatch_checkpoints.rs:112` | `set_files` + `complete_stage(files…)` + `begin_stage(diff…)` |
| `App::handle_checkpoint_file_diff_loaded` | `app/dispatch_checkpoints.rs:186` | `set_diff` + `complete_stage(diff…)` |
| `Navigator::is_view_loading` | `views/navigator.rs:103` | feeds `tick_should_draw` (app/mod.rs:89) |
| `App::is_view_loading` | `app/state.rs:230` | App-level gate operand |

## Gaps (what TUI-107 must add)

1. `render.rs`: paint `render_loading_dialog` over the body when `is_loading()`;
   `render_empty` only after the list flushed and the list is empty.
2. `keys.rs`: `handle_loading_key` guard (ESC → Ignored, other keys → Consumed)
   placed AFTER the restore/delete guards (invariant: delete/restore take precedence);
   extend the mouse prepend-guard with `|| self.is_loading()`.
3. Elapsed-ms source for the spinner: view-owned `Instant` start (set in `new()`),
   `elapsed.as_millis() as u64` at paint time (no new timers in the run loop).
4. Stale-drop invariants preserved: `complete_stage` no-op on key mismatch;
   `set_files`/`set_diff` selection guards untouched.
