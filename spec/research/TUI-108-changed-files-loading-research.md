# TUI-108 Research — Changed Files view (`f`): staged animated loading dialog via the shared base

Scope note: shared machinery (lifted spinner, `LoadingDialog` on `dialog_theme`, `LoadTracker`, redraw gate) is designed in **TUI-106**. This doc is the ChangedFilesView-specific wiring. It's the simpler of the two consumers: one list, two stages instead of three.

## 1. Current loading path (verified in code)

1. Board `f` → `Action::OpenChangedFilesView` (`views/board.rs:233`).
2. Navigator → `ViewMode::ChangedFiles` (`views/navigator.rs:149,194-196`).
3. `App::handle_open_changed_files_view` (`app/dispatch_changed_files.rs:26-29`): `self.navigator.changed_files = ChangedFilesView::new()` + `spawn_changed_files()`.
4. `spawn_changed_files` (33-51): `tokio::spawn` of `backend.changed_files()`; `Err` → `tracing::warn!` + `ChangedFilesLoaded(Vec::new())` (silent empty). Done → `Action::ChangedFilesLoaded(files)`.
5. `handle_changed_files_loaded` (55-60): `changed_files.set_files(files)` (hard-resets selection/scroll/diff, `views/changed_files/mod.rs:106-113`) then — if a file is selected — `spawn_file_diff(path)` → `Action::FileDiffLoaded { path, diff }` → `handle_file_diff_loaded` (89-91) → `set_diff(path, diff)` keyed by `diff_path` for stale-drop (`mod.rs:117-124`).
6. Render (`views/changed_files/render.rs:30-94`): `if files.is_empty() → render_empty` → **"No changed files"** (`EMPTY_MESSAGE`, line 25) — painted while BOTH stage reads are in flight.

Also note: stage 2 fires again on *every* file selection (`move_selection`, `mod.rs:225-248` emits `Action::LoadFileDiff(path)` when the selection changes) — so the "diff loading" state the user perceives tonight is a *blank right pane* after each arrow key in a large tree. The dialog fixes both the first-load blank and the selection-churn blank.

## 2. What changes

### 2.1 View state (`views/changed_files/mod.rs`)

- Add `loading: Option<LoadingDialog>` + `tracker: LoadTracker` (TUI-106 types).
- Fold the existing `diff_path: Option<String>` stale-drop field into the tracker's key (`"diff:{path}"`). `set_diff` keeps its `selected_file().path == path` guard (independent of the tracker — tracker only tracks *in-flight*, not *rendered* content).
- Mount in `ChangedFilesView::new()` → `loading = Some(LoadingDialog::new("Loading changed files"))`, tracker stage "list" in-flight. (Same mount-on-new decision as TUI-107: the view is only constructed in `handle_open_changed_files_view`.)
- Dismiss + stage flow:
  - `set_files` → `tracker.mark_list_flushed()`; `loading = None` if the LIST stage just flushed AND no stage-2 is in flight; re-arm a DIFF-stage marker immediately after (the App cascade emits `LoadFileDiff` synchronously right after `set_files` in `handle_changed_files_loaded`, but to keep the view self-contained: **when `set_files` yields a selected file, start stage `diff:{path}` for the selected path** — the view knows what the dispatcher will next spawn, and if step-2 somehow doesn't follow the dialog would mislead… no — better: the view marks `files` stage complete, and the *diff* stage is armed by `move_selection` (already emitting `Action::LoadFileDiff`) + a new helper `begin_diff_load(path: &str)` called by the dispatcher in `handle_changed_files_loaded` and from `move_selection`'s own emit. One-line dispatcher addition, keeps view logic symmetric: every place `LoadFileDiff` is emitted also calls `begin_diff_load`.

Render decision table (mirror of TUI-107's two-stage row):

| tracker state | files vec | paint |
|---|---|---|
| "list" in-flight | (empty) | LoadingDialog over body — "Loading changed files…" |
| "diff:{p}" in-flight | ≥1 file | panes + LoadingDialog over body — "Loading diff for {path}…" |
| "diff:{p}" in-flight | empty + list flushed | **"No changed files"** (empty state wins — there is no diff to load; dialog would be a lie) |
| flushed | empty | "No changed files" |
| flushed | non-empty | panes |

Separating the "empty after flush, no stage" cell keeps the contract honest.

### 2.2 Keys (`mod.rs::handle_key`, 173-196)

Add at the top (after the Ctrl/Alt fall-through, before `Esc`): `if self.loading().is_some() { return self.handle_loading_key(key); }` → ESC `Ignored`, everything else `Consumed`. Mouse: prepend-guard in `handle_mouse` (in `mouse.rs` — check exact entry in implementation). Same precedence as TUI-107 (no dialogs exist in this view today, so loading is the only modal here).

Question: distinct loading state for Clean vs. dirty — **resolved above**: stage "list" in-flight always shows the dialog; after the list flushes empty, no dialog → "No changed files".

### 2.3 Dispatcher touch points (`app/dispatch_changed_files.rs`)

- `handle_changed_files_loaded`: after `set_files`, if `selected_path().is_some()` call `changed_files.begin_diff_load(path)` before spawning (pair with the existing `spawn_file_diff`).
- No changes to `spawn_file_diff` / `handle_file_diff_loaded` internals (stale-drop unchanged; `set_diff` still keyed).
- `Action::LoadFileDiff` emitted from the view's `move_selection` already lands back at the dispatcher via the normal bus → the dispatcher calls `(self.navigator.changed_files)` to begin the diff stage? *No* — `LoadFileDiff` targets the runtime, not the view: the emit path is view→bus→`handle_load_file_diff`, which can call `self.navigator.changed_files.begin_diff_load(path)`. So: **every** `LoadFileDiff` fold begins the stage; `FileDiffLoaded` flushes it (`set_diff` calls `tracker.complete_stage(&format!("diff:{path}"))` when matched). Symmetric and testable.

## 3. Redraw while the dialog is up (TUI-106 gate)

`ChangedFilesView::is_loading() -> bool`; `Navigator::is_view_loading` already matches ViewMode arms (add the ChangedFiles arm for TUI-107's Checkpoints in the same commit). No new timers (SSR: the run loop owns the clock; the view only reports state).

## 4. Invariants that must survive (regression list)

1. Stale-drop: `set_diff` for `path ≠ selected_file().path` is a no-op (`mod.rs:117-124`) — no change.
2. `set_files` hard-reset (selection 0, scroll 0, diff clear) — no change.
3. Silent error degradation (`Err` → empty + warn, `dispatch_changed_files.rs:44-48`) — preserved; visible error dialogs out of scope (same as TUI-107).
4. `move_selection` still emits `LoadFileDiff` exactly on selection *change* (`clamped != selected_index`, `mod.rs:232-247`) — no double-emit.
5. Public surface `selected_file/selected_path/is_empty/focused_pane/...` unchanged (dispatch + tests depend on it).
6. Wheel/scrollbar hit-testing rect caching (`mod.rs:67-75`, render.rs:88-92) — the dialog overlay must not corrupt the `last_*_rect` cache (dialog paints over panes *after* they, like the restore modal pattern).

## 5. Test plan (ACDD → Gherkin → Rust)

Feature file: `spec/features/changed-files-view-loading-indicator.feature`. Scenarios:

1. Given the Changed Files view is opened, When `changed_files` has not yet returned, Then the body shows an animated "Loading changed files…" dialog and NOT "No changed files".
2. And the task returns zero files, Then "No changed files" appears without a dialog.
3. Given the list is loaded, When a file is selected, Then the dialog shows "Loading diff for {path}…" until the diff folds in.
4. Given a stale `FileDiffLoaded` for a non-selected path arrives, Then it does not clear the in-flight stage for the selected path.
5. While the dialog is active, ESC stays put; after flush, ESC emits `CloseChangedFilesView`.
6. Dialog renders via the canonical `dialog_theme` (rounded border, cyan accent, animated glyph across the 0→80 ms window — spinner-frame test lives in TUI-106).
7. Selection churn: press Down twice in quick succession → both `LoadFileDiff`s get tracked; the last result flushes, the earlier stale one is dropped (regression of the existing stale-drop + a new-stage marker in the middle).

Rust locations: `views/changed_files/tests.rs` (existing file), render harness at `views/navigator.rs:234-248`; spinner-frame asserts on TUI-106's `components/loading_dialog.rs` tests.

## 6. Effort

~120-180 LoC (view + dispatcher + gate arm) + ~250 LoC test. Depends on TUI-106. Estimation: **3** (fewer stages than TUI-107, same-shape arc, much smaller file on top).

## 7. Files read

See TUI-106 doc §10. Anchors: `views/changed_files/mod.rs` (state, set_*, handle_key, move_selection), `render.rs:25,30-94`, `mouse.rs` (event entry), `app/dispatch_changed_files.rs:26-116` (spawns + folds).
