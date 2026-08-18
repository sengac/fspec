# AST research — TUI-108 changed-files loading wiring

Analyzed via AST search over `rust/fspec-tui/src` (functions/structs relevant to the
ChangedFilesView loading-dialog wiring).

## Key entities

| Entity | Location | Role |
|---|---|---|
| `ChangedFilesView` (struct) | `views/changed_files/mod.rs:55` | dual-pane state; already holds `loading: LoadingDialog` + `load: LoadTracker` (TUI-106) |
| `ChangedFilesView::new` | `views/changed_files/mod.rs:93` | mounts `LoadingDialog::new("Loading changed files", "Loading changed files…")` + `LoadTracker::new(...)` |
| `ChangedFilesView::is_loading` | `views/changed_files/mod.rs:120` | delegates to `load.is_loading()` |
| `ChangedFilesView::sync_loading_label` | `views/changed_files/mod.rs:127` | copies `load.active_label()` onto `loading.label` |
| `ChangedFilesView::set_files` | `views/changed_files/mod.rs:135` | folds list result; hard-resets selection/scroll/diff; does NOT yet mark the tracker flushed (dispatcher does) |
| `ChangedFilesView::set_diff` | `views/changed_files/mod.rs:146` | stale-drop via `selected_file().path == path` guard; no tracker interaction (dispatcher calls `complete_stage`) |
| `ChangedFilesView::move_selection` | `views/changed_files/mod.rs:254` | emits `LoadFileDiff` exactly on selection change (single-emit invariant) |
| `ChangedFilesView::handle_key` | `views/changed_files/mod.rs:202` | Ctrl/Alt → Ignored; ESC → Close. NO loading guard yet |
| `ChangedFilesView::handle_mouse` | `views/changed_files/mouse.rs:27` | no prepend-guard for loading yet |
| `ChangedFilesView::render` | `views/changed_files/render.rs:30` | paints panes; `render_empty` when `files.is_empty()` (render.rs:96) — the fake empty state; NO loading-dialog paint branch yet |
| `LoadingDialog` / `render_loading_dialog` | `components/loading_dialog.rs:35/83` | TUI-106 shared dialog (cyan accent, rounded border, braille spinner) |
| `LoadTracker` | `components/load_state.rs:30` | staged in-flight marker; `diff_stage_key_path` = `"diff:{path}"` |
| `App::handle_changed_files_loaded` | `app/dispatch_changed_files.rs:56` | `set_files` + `mark_list_flushed` + `begin_stage(diff…)` + `sync_loading_label` |
| `App::handle_load_file_diff` | `app/dispatch_changed_files.rs:75` | `begin_stage(diff…)` + `sync_loading_label` + spawn |
| `App::handle_file_diff_loaded` | `app/dispatch_changed_files.rs:105` | `set_diff` + `complete_stage(diff…)` + `sync_loading_label` |
| `Navigator::is_view_loading` | `views/navigator.rs:103` | feeds `tick_should_draw` (app/mod.rs:89) |

## Gaps (what TUI-108 must add)

1. `render.rs`: paint `render_loading_dialog` over the body while `is_loading()`
   AND the list is non-empty OR the list stage is in flight; the
   empty-after-flush cell wins: list flushed + empty files → `render_empty`
   even if a diff stage marker lingers (there is no diff to load).
2. `mod.rs::handle_key`: `handle_loading_key` guard (ESC → Ignored, other keys →
   Consumed) after the Ctrl/Alt fall-through.
3. `mouse.rs::handle_mouse`: prepend-guard `if self.is_loading() { return Consumed; }`.
4. Elapsed-ms source for the spinner: view-owned `Instant` start (set in `new()`),
   `elapsed.as_millis() as u64` at paint time (same shape as TUI-107).
5. `set_files` marks the list stage flushed on the tracker (idempotent with the
   dispatcher's call) so tests + direct view use behave consistently.

## Invariants preserved

- `set_diff` stale-drop guard unchanged (mod.rs:146-153).
- `set_files` hard-reset unchanged.
- `move_selection` single-emit on selection change unchanged.
- Public surface (`selected_file`/`selected_path`/`is_empty`/`focused_pane`) unchanged.
- Rect cache (`last_*_rect`) untouched by the dialog overlay (painted after panes,
  like the restore-modal pattern in the Checkpoints view).
