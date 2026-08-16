# TUI-106 Research — Shared animated LoadingDialog base (canonical dialog_theme + lifted spinner + redraw-clock gate)

## 1. Goal

Provide the shared, DRY building blocks so the Checkpoints view (`c` key) and the Changed Files view (`f` key) can show an **animated loading dialog** while their lazy RPC loads are in flight — replacing today's behavior where a *loading* view paints the exact same text as a view that *loaded and legitimately has nothing* ("No checkpoints available" / "No changed files"). This is the same bug class PROV-104 already fixed for the ModelSelector.

Requirement from the user: the loading indicator must be **in a dialog**, and it should **extend the shared base dialog** the rest of the program uses.

## 2. Current state of the loading path (what we are fixing)

### 2.1 Entry points

- Board `f`: `rust/fspec-tui/src/views/board.rs:233` → `Action::OpenChangedFilesView`.
- Board `c`: `rust/fspec-tui/src/views/board.rs:238` → `Action::OpenCheckpointsView`.
- The Navigator flips `active_view` (`views/navigator.rs:149,154,194-199`); `handle_open_*_view` resets the owned view and spawns the first RPC.

### 2.2 Lazy-load dispatch (both views share the same shape)

`app/dispatch_changed_files.rs` (RPC-356) and `app/dispatch_checkpoints.rs` (RPC-364):

| View | Stage | Backend RPC (transport/mod.rs) | Result Action folded on the App |
|---|---|---|---|
| ChangedFiles | 1 | `backend.changed_files()` | `Action::ChangedFilesLoaded(files)` |
| ChangedFiles | 2 | `backend.file_diff(path)` (chained from stage 1) | `Action::FileDiffLoaded { path, diff }` |
| Checkpoints | 1 | `backend.list_checkpoints()` | `Action::CheckpointsLoaded(list)` |
| Checkpoints | 2 | `backend.checkpoint_diff_files(wu, name)` (chained) | `Action::CheckpointFilesLoaded { wu, name, files }` |
| Checkpoints | 3 | `backend.checkpoint_file_diff(wu, name, path)` (chained) | `Action::CheckpointFileDiffLoaded { wu, name, path, diff }` |

Each spawn is a `tokio::spawn` whose result is routed back over the action bus (`self.pending_tasks.push(handle)` + `action_tx.send(...)`). Errors degrade to `Vec::new()` / `None` + `tracing::warn!` — **failures are silent to the user today**.

### 2.3 The actual bug

Both views start with empty `Vec`s (`views/checkpoints/mod.rs:110-136`, `views/changed_files/mod.rs:85-102`). Render does:

```rust
if files.is_empty()       { render_empty(body, buf); return; }  // changed_files/render.rs:52,25
if checkpoints.is_empty() { render_empty(body, buf); return; }  // checkpoints/render.rs:58,29
```

→ "No changed files" / "No checkpoints available" while the RPC is in flight. **Loading ≡ empty.** There is no `loaded` flag, no spinner, no dialog.

### 2.4 No redraw clock for the mode views

Even if we painted an animated spinner it would freeze: `app/events.rs:238-243` only repaints when `tick_should_draw(should_render, is_busy, is_animating)` (`app/mod.rs:84-85`).

- `is_busy` = agent session Running/Compacting (`app/state.rs:210-216`).
- `is_animating` = agent input-row sweep (`app/state.rs:222-224`).

Nothing keeps the frame clock ticking for an idle board with a lazy mode-view open → no ticks → no redraw → a frozen spinner. **The redraw gate is part of this card** (it affects both downstream cards TUI-107/108).

## 3. The dialog stack: what "extend a shared base dialog" means here

### 3.1 Canonical visual base — `components/dialog_theme.rs` (RPC-027)

Single source of truth for the rounded / black / accent popup:

- `FspecDialog<'a> { accent, title, rows: Vec<DialogRow>, footer, min_width }` — the per-render input struct.
- `render_dialog(area, buf, &dialog)` and `render_dialog_at(rect, …)` (RPC-383): the ONE implementation of the visual contract — opaque black bg, rounded border in `accent.color()`, bold accent title, gap, body rows (inverse-highlight support), dim footer, with a compact-layout fallback for small terminals.
- `Accent` = `Cyan` (default) | `Yellow` | `Red`.

"Extending the shared base" here = build your `FspecDialog` (rows/footer/accent) and delegate the pixel paint to `render_dialog`. Every dialog in the crate already does this; e.g. `components/checkpoint_restore_dialog.rs:20-37` constructs `FspecDialog { accent: Accent::Yellow, … }` and calls `render_dialog`.

### 3.2 The two hosting patterns in the codebase

**A. Compositor host (`Priority::Critical` Component).** `components/status_dialog.rs` (RPC-079) — the model for long-running batch operations:
- State machine `StatusKind::{Restoring{ current, idx, total }, Complete, Error{ msg }}`.
- `Restoring` paints `FspecDialog { accent: Cyan, title "… Files", rows [current, "(idx/total)"] }` — i.e. it already shows a *current item + (idx/total) counter*, which is exactly the shape our loading row needs.
- Rule [7]: **ESC is IGNORED while Restoring** (`handle_event` returns `ignored()`); ESC dismisses only Complete/Error. Paste is swallowed (RPC-403).
- Auto-close via a spawned `sleep → Action::DismissDialog(id)` task.

**B. View-owned modal sub-state.** `CheckpointsView.restore_dialog: Option<RestoreDialog>` (`views/checkpoints/mod.rs:97` + `views/checkpoints/dialog.rs:45-95`): the view owns a `target + phase` struct; while `Some`, `handle_key`/`handle_mouse` route into the dialog (input capture, `views/checkpoints/keys.rs:30-36,62-66`) and `CheckpointsView::render` paints it **over the panes** (`views/checkpoints/render.rs:118-125`) via `render_restore_modal` → `dialog_theme::render_dialog`. Delete dialogs (RPC-366) reuse the exact same pattern.

### 3.3 Decision: **Pattern B (view-owned modal), built on the shared `dialog_theme` base**

Rationale (SOLID):

- Mode views are full-screen `ViewMode`s painted by the Navigator, not agent-layer overlays — the dialog belongs in that render path (Pattern B is already how this very view renders its restore/delete modals: consistency, zero new plumbing).
- Pattern B needs no Compositor registration: mount = set the `Option<LoadingDialog>: None → Some` on open; dismiss = set it `None` when the stage flushes.
- The shared base is still `dialog_theme::FspecDialog` + `render_dialog` — the same visual contract `StatusDialog` uses, so "extends the shared base dialog" is satisfied at the only level any dialog in this crate shares (the pixel/structure contract).

### 3.4 What is NEW

A loading-dedicated modal value, `components/loading_dialog.rs`:

```rust
pub struct LoadingDialog {              // view-owned; present while Some
    pub title: &'static str,            // e.g. "Loading checkpoints"
    pub label: String,                  // stable per-stage text: "Loading checkpoint list…"
    pub progress: Option<(usize, usize)>,// (idx, total) — set by TUI-109, absent until then
}
impl LoadingDialog {
    pub fn new(title: &'static str) -> Self;
    pub fn set_stage(&mut self, label: String);            // cascade moved to a new stage
    pub fn set_progress(&mut self, idx: usize, total: usize); // TUI-109 hook (no-op until fed)
    pub fn spinner_line(&self, elapsed_ms: u64) -> String;  // "{glyph} {label}…" glyph from components::spinner
    pub fn dismissable(&self) -> bool { false }            // ESC ignored while loading (StatusDialog rule [7])
}
// render helper:
pub fn render_loading_dialog(area, buf, &dialog, elapsed_ms: u64) { /* build FspecDialog{accent:Cyan, rows:[spinner line, optional "(idx/total)"] } → render_dialog */ }
```

Both mode views call `render_loading_dialog` from their own `render` (paint-over-the-panes), exactly as they already call `render_restore_modal`.

## 4. Spinner — lift is mandatory (avoid a wrong dependency direction)

`views/agent/spinner.rs` (RPC-095) owns the pure braille spinner:
- `DOTS_FRAMES: [&str; 10]` (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏), `DOTS_INTERVAL_MS = 80`.
- `current_frame_glyph(elapsed_ms) -> &'static str` — frame = `(ms / 80) % 10`.
- `paint_spinner_line(area, buf, frame_index, message, hint)` — DIM-styled painter.

It lives under `views/agent/` — importing `views::agent` from `views::checkpoints`/`views::changed_files` is a wrong dependency direction. **Deliverable: move it to `components/spinner.rs`** and re-export so the agent code (which uses `super::spinner::*` in `agent/input_transition.rs`, `agent/transition_driver.rs`) keeps compiling. Zero behavior change; its 7 unit tests move / re-export.

## 5. Redraw-clock gate (drives the animation)

Extend the existing chain — keep the single responsibility in the run loop:

1. Each mode view: `fn is_loading(&self) -> bool` (true while its `LoadingDialog` is `Some` / a stage is pending).
2. `Navigator::is_view_loading(&self)` → `match active_view`, ask the owned view (mirrors how `App::is_input_animating` delegates to `self.navigator.agent.is_input_animating()`, `app/state.rs:222`).
3. `App::is_view_loading(&self)` in `app/state.rs`.
4. `app/events.rs:243`: add the flag to `tick_should_draw(...)`.

Prefer adding a 4th boolean to `tick_should_draw` (its test block at `app/mod.rs:84-106` extends by adding cases) over a per-view `tokio::time::interval` — the ~60fps render tick already exists and a second animation clock per view would be duplication. The change is small: one param + the gate + ~1-2 tests.

Cost: while any mode view reports `is_loading == true`, the 16ms tick redraws — the same cost profile as a busy session's spinner today, and fully idle again once the last stage flushes.

## 6. Staged "progress" when there are many checkpoints (the LoadTracker)

Each *stage* of the cascade is an independent in-flight RPC. Progress is therefore shown as **which load is pending** (the spinner + stage label), not a per-item fraction (that requires the TUI-109 wire change):

```rust
// shared: components/load_state.rs  (or folded into loading_dialog.rs — see budget note)
pub struct LoadTracker {                 // one per view cascade
    list_loaded: bool,                   // stage 1 done
    stage_key: Option<String>,           // identity of the in-flight stage (for stale-drop)
}
impl LoadTracker {
    pub fn mark_list_flushed(&mut self) -> bool /* was_loading, to dismiss dialog */;
    pub fn begin_stage(&mut self, key: String);
    pub fn complete_stage(&mut self, key: &str);   // no-op when key ≠ current (stale-drop preserved)
    pub fn active_label(&self) -> Option<String>;  // drives spinner line + is_loading()
    pub fn is_loading(&self) -> bool;
}
```

- Checkpoints: `stage_key ∈ { "files:{wu}:{name}", "diff:{wu}:{name}:{path}" }` — the keys already exist as `files_key`/`diff_key` (`views/checkpoints/mod.rs:76-81`); the tracker folds those two fields into one marker.
- ChangedFiles: `stage_key = "diff:{path}"` — folds `diff_path` (`views/changed_files/mod.rs:61`).
- **Stale-drop invariance:** today `set_files`/`set_diff` drop results whose selection key mismatches (`views/checkpoints/mod.rs:163-189`, `views/changed_files/mod.rs:117-124`). `complete_stage(key)` is called only when the key matches the current selection, so this registry of behavior is preserved exactly.
- Labels come from the tracker's active stage (text owned by the view, sanitized with `crate::sanitize_for_terminal` per TUI-104/105): "Loading checkpoint list…" → "Loading files for {label}…" → "Loading diff for {path}…", and "Loading changed files…" → "Loading diff for {path}…".

**Honesty about "progress":** a repo with many checkpoints makes stage 1 long — `collect_checkpoints` (`rpc/src/checkpoints.rs:40-65`) enumerates *every* ghost checkpoint across all work units plus per-WU index sidecar reads, sorted and truncated at 200. The dialog surfaces that as *visibly long-running* (spinner + label). The `(idx/total)` counter slot is already shaped (StatusDialog renders it), so TUI-109 just starts feeding it.

## 7. DRY / SOLID summary

| Property | Where it is satisfied |
|---|---|
| **DRY** | One spinner (lifted, shared). One dialog visual contract via a single `LoadingDialog` + `render_loading_dialog`. One `LoadTracker` for both cascades. One redraw-gate operand. Views hold only flags + labels + mount/unmount. |
| **Single Responsibility** | `components/spinner.rs` = pure glyph/frames. `dialog_theme` = pixel contract (untouched). `LoadingDialog` = loading-modal state (input swallow + no-dismiss + label/counter). `LoadTracker` = staged in-flight marker. Mode view = its own cascade pairing (open→mount, result fold→update tracker, flush→dismiss). |
| **Open/Closed** | A third lazy mode-view reuses the same four parts; it only adds its labels + RPC actions. |
| **Dependency Inversion** | Views depend on `components::spinner` + `components::loading_dialog` (leaves), never on `views::agent`. App dispatch stays the only spawn-site (the action bus remains the single coordination channel — "two front doors, one source of truth" pattern preserved). |
| **ACDD** | Gherkin scenarios per view → Rust tests → code. Spinner lift is a pure refactor (existing tests carry the contract). |

## 8. File-size budget (300-LoC ceiling)

- `components/loading_dialog.rs` ≈ 150–220 LoC with its unit tests (buffer asserts + an insta snapshot, following the `status_dialog.rs` test block).
- `views/checkpoints/mod.rs` is already near the ceiling: add `loading_dialog: Option<LoadingDialog>` + `load: LoadTracker` + `is_loading()` (+~15 LoC net). `render.rs` gains a dialog branch (+~10 LoC). If the total overshoots, keep the tracker in `components/` as its own module — do NOT restructure the existing view module (that risks disturbing the RPC-364/365/366 shape tests).
- `views/changed_files/mod.rs` is comfortable: same three additions.
- `app/state.rs` + `app/events.rs` + `app/mod.rs` gate: <~15 LoC across the three.

## 9. Test plan (feeds the TUI-107/108 feature files + this card's own)

- Spinner lift: the 7 existing unit tests move; assert byte-identical glyphs + DIM/origin painter asserts.
- `tick_should_draw` with the new operand: extend the truth table (`app/mod.rs` test module) — add the case where only `is_view_loading` is true.
- `LoadingDialog` + `render_loading_dialog`: buffer-render on a TestBackend — cyan rounded border, title present, `⠋` at the spinner row's first column at `elapsed=0` and a different glyph at `elapsed=80ms`, `(3/10)` counter row only when progress is set, empty footer.
- "Loading ≠ empty": a fresh view (not-yet-loaded) render must contain the spinner glyph + label and must NOT contain "No checkpoints available" / "No changed files" (the model_selector PROV-104 discriminator, applied here).
- Staged flush semantics: `set_files`/`set_diff` with a stale key leave the tracker's active stage unchanged; `mark_list_flushed` on an empty list dismisses the dialog AND surfaces the real empty message.
- Input capture: during loading, ESC returns `Ignored` (view stays open), matching StatusDialog rule [7]; other keys are swallowed by the `if loading_dialog.is_some()` guard that sits alongside the existing `dialog().is_some()` guard in `keys.rs`.

## 10. Files read for this research

- `rust/fspec-tui/src/app/dispatch_checkpoints.rs` (RPC-364), `dispatch_changed_files.rs` (RPC-356), `dispatch_checkpoint_restore.rs` (RPC-365)
- `rust/fspec-tui/src/views/checkpoints/{mod,render,keys,dialog}.rs`
- `rust/fspec-tui/src/views/changed_files/{mod,render}.rs`
- `rust/fspec-tui/src/views/model_selector/{state,rows,rows_render,tests_loading_empty}.rs` — PROV-104 loading≠empty precedent
- `rust/fspec-tui/src/components/{dialog_theme,dialog_theme_rows,status_dialog,checkpoint_restore_dialog}.rs`
- `rust/fspec-tui/src/views/agent/spinner.rs` (RPC-095) + `input_transition.rs`, `transition_driver.rs` (its callers)
- `rust/fspec-tui/src/app/{events,mod,state}.rs` (RPC-008 render tick; RPC-093 busy/animating gates)
- `rust/fspec-tui/src/transport/{mod,embedded}.rs` (single-RPC backends; broadcast-channel precedent for TUI-109)
- `rust/rpc/src/checkpoints.rs` (RPC-362, `MAX_CHECKPOINTS = 200`, enumeration cost)
