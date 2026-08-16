# TUI-107 Research — Checkpoints view (`c`): staged animated loading dialog via the shared base

Scope note: the shared machinery (lifted spinner, `LoadingDialog` built on `dialog_theme`, `LoadTracker`, redraw-gate extension) is designed in **TUI-106**. This document covers what is CheckpointsView-specific: wiring, the cascade stages, stale-drop invariants, and the render/keys integration. Read TUI-106's doc first — the two cards are one feature split along the "shared base" vs. "consumer" seam.

## 1. Current loading path (verified in code)

1. Board `c` → `Action::OpenCheckpointsView` (`views/board.rs:238`).
2. Navigator flips to `ViewMode::Checkpoints` (`views/navigator.rs:154/197-199`).
3. `App::handle_open_checkpoints_view` (`app/dispatch_checkpoints.rs:25-28`): resets `self.navigator.checkpoints = CheckpointsView::new()` and calls `spawn_list_checkpoints()`.
4. `spawn_list_checkpoints` (same file, 30-47): `tokio::spawn` of `backend.list_checkpoints()`; on `Err` → `tracing::warn!` + `Vec::new()`; on done → `Action::CheckpointsLoaded(list)`.
5. `handle_checkpoints_loaded` (51-57): `checkpoints.set_checkpoints(list)` — which **hard-resets selection and clears files+diff** (`views/checkpoints/mod.rs:140-145`) — then, if a checkpoint is selected, `spawn_checkpoint_files(wu, name)` (stage 2), which on done emits `Action::CheckpointFilesLoaded` → `handle_checkpoint_files_loaded` (91-111) → `set_files(...)` + cascade `spawn_checkpoint_file_diff` (stage 3) → `Action::CheckpointFileDiffLoaded` → `set_diff(...)`.
6. Render (`views/checkpoints/render.rs:34-126`): if `checkpoints.is_empty()` → `render_empty` paints **"No checkpoints available"** (`EMPTY_MESSAGE`, line 29). That message is what the user sees *during* all three in-flight stages of the first load.

### Why it hurts when there are many checkpoints

`backend.list_checkpoints()` is one batched tarpc call, but server-side `collect_checkpoints` (`rpc/src/checkpoints.rs:40-65`) enumerates **every** ghost checkpoint ref across **all** work units (`list_all_ghost_checkpoints`) plus one JSON sidecar read per work unit, then sorts and truncates to `MAX_CHECKPOINTS = 200`. With hundreds of checkpoints this RPC can lag several seconds — during the whole window the view shouts "No checkpoints available". Stages 2/3 (`checkpoint_diff_files`, `checkpoint_file_diff`) are also separate RPCs and can each lag on large trees.

## 2. What changes

### 2.1 View state (`views/checkpoints/mod.rs`)

- Add `loading: Option<components::loading_dialog::LoadingDialog>` (TUI-106 type).
- Replace the purpose of `files_key: Option<(String,String)>` / `diff_key: Option<(String,String,String)>` with the TUI-106 `LoadTracker` (single marker, keys `"files:{wu}:{name}"` / `"diff:{wu}:{name}:{path}"`). **The keys keep the same content** — the stale-drop guards in `set_files`/`set_diff` (mod.rs:163-189) become `tracker.complete_stage(key)` calls guarded by the same `selection_matches` checks. `files_key`/`diff_key` may be removed once the tracker is in place (they exist only to discriminate stale async results — the tracker does that in one field).
- Mount: `CheckpointsView::new()` → `loading = Some(LoadingDialog::new("Loading checkpoints"))`, tracker starts at stage-1 "list" in-flight. (Two options exist: mount-on-new vs. mount-on-App-open. **Mount on `new()`** — the view is only constructed in `handle_open_checkpoints_view`, so lifecycle is identical, and the test story "fresh view renders the dialog" needs no App plumbing.)
- Dismiss: `set_checkpoints` calls `tracker.mark_list_flushed()` and drops `loading` **only if the list is non-empty**; on an empty list it keeps the flag cleared but the view then renders the true empty message. (i.e. `loading = None` unconditionally at end of `set_checkpoints`; the *discrimination* loading-vs-empty happens purely via the render branch + whether stage-1 ever flushed.)

Concretely the render decision table:

| tracker state | checkpoints vec | paint |
|---|---|---|
| stage "list" in-flight | (empty) | **LoadingDialog over body** — "Loading checkpoint list…" spinner |
| stage "files:{a}:{b}" in-flight | ≥1 checkpoint | panes + **LoadingDialog over Files pane area** — "Loading files for {label}…" |
| stage "diff:{a}:{b}:{p}" in-flight | ≥1 cp, ≥1 file | panes + LoadingDialog over Diff area — "Loading diff for {path}…" |
| flushed | empty | "No checkpoints available" (unchanged `render_empty`) |
| flushed | non-empty | panes, no dialog |

Design question resolved: dialog covers the **whole body** (simpler, matches how the restore modal covers all panes today at render.rs:118-120) rather than one pane — a per-pane overlay would collide with the `mem::take` render pattern and complicates hit-testing; the whole-body overlay has zero hit-test cost because while loading the user *cannot* meaningfully interact (nothing to select until stage 2 exists, and keys are captured).

### 2.2 Keys (`views/checkpoints/keys.rs`)

- Add the `if self.loading().is_some() { return self.handle_loading_key(key); }` guard alongside the existing `dialog().is_some()` / `delete_dialog().is_some()` guards (keys.rs:29-36).
- `handle_loading_key`: **ESC → `CheckpointsEvent::Ignored`** (view stays open; mirrors `StatusDialog` rule [7] — an ESC during a ~2s load is 99% a reflex/accident and closing-with-no-data forces a re-open + re-load). All other keys → `Consumed` (swallow). Mouse: extend the existing prepend-guard (keys.rs:62-66) with `|| self.loading().is_some()`.

### 2.3 Labels (staged progress)

Per-stage, from the tracker's active key (text is view-owned, TUI-106 provides only the spinner + frame):
- `"Loading checkpoint list…"`
- `"Loading files for {checkpoint_label(cp)}…"` — uses the existing `checkpoint_label` helper (mod.rs export, auto/manual formatting).
- `"Loading diff for {path}…"` — path run through `sanitize_for_terminal` (TUI-104/105 precedent, checkpoints/render.rs:257).

This *is* the "progress" for the many-checkpoints case at the protocol-free level: the user always sees which of up to 3 loads is pending, each with its own target. Per-item `(idx/through total)` percentages are deliberately **out of scope** (see TUI-109) — the dialog leaves the counter row's slot shaped so TUI-109 can feed it without touching this view again.

## 3. Redraw while the dialog is up (institutional wiring)

Uses the TUI-106 gate: `CheckpointsView::is_loading(&self) -> bool` = `self.loading_dialog().is_some()` (or tracker.active != flushed-list). Navigator/App read it through `App::is_view_loading`; `tick_should_draw` stays 4 params (TUI-106 owns the change). No new timers in the view (SSR: the run loop owns the clock; the view only reports state).

## 4. Invariants that must survive (regression list)

1. Stale-drop: `set_files` for `(wu,name) ≠ selected` is a pure no-op — same today (mod.rs:164-166), same after refactor via tracker key match.
2. Stale-drop: `set_diff` for `(wu,name,path) ≠ (selected cp, selected file)` no-op (mod.rs:177-182).
3. Errors degrade silently to empty + warn (dispatch_checkpoints.rs:38-43,74-79,133-138) — preserved; *visible* error surfacing remains out of scope (could be a follow-on: `dialog_theme` has a `Red`/Error precedent in StatusDialog).
4. `CheckpointsView::new()` hard-resets everything (`set_checkpoints` → selection 0, scroll 0, files/diff cleared) — unchanged.
5. Restore (RPC-365) / Delete (RPC-366) dialogs take precedence over the loading dialog at render + key-capture time (delete/restore only reachable once list loaded, but keep the guard order: delete/restore first, then loading — matches current ordering where those guards sit first in keys.rs).
6. `selected_checkpoint()`/`first_file_path()`/`is_empty()` public surface unchanged (dispatch + tests depend on them).

## 5. Test plan (ACDD → Gherkin → Rust)

Feature file: `spec/features/checkpoints-view-loading-indicator.feature` (capability name — describes WHAT, not the task). Scenarios (mirror the four-cell render table + invariants):

1. Given the Checkpoints view is opened, When `list_checkpoints` has not yet returned, Then the body shows the animated loading dialog with "Loading checkpoint list…" and NOT "No checkpoints available".
2. And the task completes with zero checkpoints, Then the view shows "No checkpoints available" and no dialog.
3. Given the list is loaded, When a checkpoint is selected, Then the dialog shows "Loading files for {label}…".
4. And the files are loaded, Then the dialog shows "Loading diff for {path}…" until the diff folds in.
5. Given a stale `CheckpointFilesLoaded` for a non-selected checkpoint arrives, Then it does not clear the current stage's loading flag (counter part).
6. While the loading dialog is active pressing ESC, Then the view stays open (dialog ignored); after flush, ESC closes the view (existing CloseCheckpointsView).
7. The loading dialog renders through the canonical dialog theme (rounded double-corner border, cyan accent, title "Loading checkpoints", spinner glyph advances 0 ms → 80 ms glyph).

Rust-side locations: `views/checkpoints/tests.rs` (table-row table + esc) + `components/loading_dialog.rs` tests from TUI-106 (glyph/frame assertion). Render-assert harness: existing pattern at `views/navigator.rs:234-248` (TestBackend 120×24 → collect buffer text).

## 6. Effort

~150-200 LoC across mod.rs/keys.rs/render.rs + ~200-300 LoC tests. Depends on TUI-106 (which delivers the types + gate). Estimation: **5** (clear patterns, multiple files, no protocol change).

## 7. Files read

See TUI-106 doc §10 (same list). View-specific anchors checked: `views/checkpoints/mod.rs` (state + set_* + keys of load), `mod.rs:label → checkpoint_row`, `render.rs:58-61,29,118-126`, `keys.rs:29-66`, `app/dispatch_checkpoints.rs:25-160`.
