# TUI-109 Research — Per-item streaming progress for checkpoint enumeration

Depends on: **TUI-106** (LoadingDialog counter slot + redraw gate), **TUI-107** (CheckpointsView stage mounting). Extends the feature files of both.

## 1. The gap

TUI-106/107/108 make each *stage* visible ("Loading checkpoint list…" / "Loading files for X…" / "Loading diff for Y…"). But with many checkpoints, stage 1 — `backend.list_checkpoints()` — is a single batched tarpc call (`rust/fspec-tui/src/transport/embedded.rs:128-131` → `rust/rpc/src/checkpoints.rs:collect_checkpoints`, capped at `MAX_CHECKPOINTS = 200`) where the user sees a bare spinner for the entire enumeration: `list_all_ghost_checkpoints` walks **every** ghost ref under all work units, `collect_checkpoints` then reads one `.git/fspec-checkpoints-index/<wu>.json` sidecar per work unit, sorts, truncates. There is no wire-level notion of *how far* it is.

The user's original ask ("progress of the loading of that view **when there are many checkpoints**") only resolves at this level.

## 2. Precedents already in the codebase

- **Broadcast channels on the transport**: the TUI transport trait (`rust/fspec-tui/src/transport/mod.rs`) already exposes `work_units_rx()`, `chunks_rx()`, `logs_rx()` — `tokio::sync::broadcast::Receiver`s fed by server events. `checkpoint_counts`/`list_checkpoints` etc. are single-shot methods beside them. So a `checkpoints_progress_rx()` is a *pattern extension*, not a new channel kind (same for the websocket transport, `transport/websocket.rs`, which needs to forward the new event type).
- **Counter row shape**: `components/status_dialog.rs` (RPC-079) already renders `current item` + `"(idx/total)"` as two `DialogRow`s through the base `dialog_theme::render_dialog`. TUI-106's `LoadingDialog` includes `progress: Option<(usize, usize)>` + `set_progress` precisely so this card only feeds data, changing no view/render code.

## 3. Design

### 3.1 Wire type (new, `rust/rpc-types/src/lib.rs`)

```rust
pub struct CheckpointsProgress {
    pub loaded: usize, // countries enumerated so far (probably by count)
    pub total: usize, // countries total (0 until known — see 3.2)
    pub done: bool,
}
```

### 3.2 Server side

- `codelet_git::list_all_ghost_checkpoints` → add a parallel streaming variant: `list_all_ghost_checkpoints_stream(cwd, on_item: FnMut) -> Result<Vec<(String,String)>>` (callback-per-item; keeps the return value backward-compatible) — the ref walk is where the per-item ticks naturally are.
- `collect_checkpoints` gains `collect_checkpoints_stream(cwd, on_progress)` that drives the callback after each sidecar read (second tick source; total = sidecars discovered so far). The existing non-streaming `collect_checkpoints` remains for the CLI (`rust/fspec-core`'s `list_checkpoints` CLI command must be unaffected — two front doors, implementations are unified: the non-streaming variant calls the stream variant with a no-op callback — one source of truth, no logic duplication, uphold "Two Front Doors, One Source of Truth").
- The rpc `FspecServiceImpl::list_checkpoints` (`rust/rpc/src/lib.rs:1038`) emits progress events on the broadcast sender; the final Vec is still returned through the same RPC.

### 3.3 TUI side

- `transport/mod.rs`: trait method `checkpoints_progress_rx()`; `embedded.rs` exposes the broadcast rx; `websocket.rs` forwards the new progress message kind (degrade gracefully: on transports that don't emit events the dialog just never receives any → spinner-only, which is exactly TUI-107's behavior, so **no timeout logic needed** — the fallback is automatic and zero-cost).
- App: a subscriber task on `checkpoints_progress_rx()` (the work-units-rx subscriber pattern already exists in `app/` — follow it, `AppState` spawns the task), which on receipt emits a new action `CheckpointsProgress { loaded, total }` → App fold: if `navigator.active_view == ViewMode::Checkpoints` and the checkpoint view's tracker stage is "list", call `loading_dialog.set_progress(loaded, total)` (and keep the redraw gate true). Stale-drop: a `done=true` followed by a `CheckpointsLoaded` fold (which clears the dialog) — the bus order on the same runtime guarantees listener → fold; guard: at fold the dialog is dropped.
- Ordering subtlety (noted in the card description): progress events may arrive **after** `CheckpointsLoaded` (broadcast lag) — fold never reopens the dialog (only `LoadingDialog::set_progress` on a `Some` dialog matters; `load_flushed` leads to `loading = None`).

### 3.4 Display

Counter line in the dialog: `(47/180)` below the spinner line (or, following StatusDialog, an independent `DialogRow`). A progress-complete ring shows `(N/total)` before the list folds in — the already-written TUI-107 spec scenario (b) is extended to cover TUI-109.

### 3.5 What we explicitly do NOT touch

- No changes to the CLI's `list-checkpoints` output (rpc non-streaming path is preserved).
- No changes to stages 2/3 (file diff enumeration is per-selected-item, user-initiated — a per-interval spinner is sufficient; if it ever becomes slow, follow on by feeding a diff file list → same mechanism, different key allocation — LoadTracker key allocation is already open to this via Open-Closed).
- No changes to the auto-checkpoint 200 cap.

## 4. DRY/SOLID notes

- **Reuses**: broadcast-channel pattern (no new transport vehicle, one follow-on type + one forwarder), StatusDialog's counter line format, TUI-106's counter slot + gated redraw, LoadTracker's stale-drop for progress vs. list-order ripples.
- **Single Responsibility**: the git crate only *ticks* (callback), the rpc crate *shapes* (limits, sort, done flag), the TUI app *folds*, the view *paints*. No file exceeds the 300-line ceiling (the diff-side change is well under 50 lines per crate; the rpc-types addition is a 6-line struct).
- **Open/Closed**: a future "streaming changed_files" (if enumerating `git status` also becomes slow) adds its own progress follow-on type following the identical path — zero new infrastructure.

## 5. Testing (ACDD — scenarios added to the TUI-107 feature file + their own)

1. Given 150 checkpoints across 10 work units, when the view is opened, then the counter line advances visibly from (0/…) through intermediate values rendered by the 60fps tick up to (150/150) before the list shows up — test: mock the transport / embedded backend in a temp directory fixture (the `rust/rpc/tests/checkpoint_transport_rpc362.rs` pattern is the harness).
2. The counter reflects the "before limit": with 250 checkpoints, the (capped at 200) enumeration shows (200/250) — truncation does not hide progress.
3. Late `CheckpointsProgress{done}` arriving after `CheckpointsLoaded`: no additional hang, view is in the list presentation state; dialog is not re-painted (stale drop).
4. With the websocket transport that doesn't forward progress events: dialog = spinner + stage label only (thanks to TUI-107 — ask for no regression).
5. N-API/CLI regression: `fspec list-checkpoints` output is byte-identical to today (the no-op callback path is exercised by existing tests in `rust/git/tests` + `rust/fspec-core/tests/list_checkpoints.rs` without change).

## 6. Estimation refinements

Effort breakdown → 8: rpc-types + transport plumbing (2), git's stream side + callback variant (2, including sub-millisecond delay, 3 test fixtures), fold + fold on the TUI side + action + subscribe (2), scenarios + tests (occasional revert scenario (2)). Three crates + 2-3 transport implementations, but each slice is mechanically small.

## 7. Files examined for this document

- `rust/fspec-tui/src/transport/{mod,embedded,websocket}.rs` (rx of `work_units`, `chunks`, `logs`; `list_checkpoints` implementation)
- `rust/rpc/src/checkpoints.rs` (collect_checkpoints, MAX_CHECKPOINTS, sidecar reads)
- `rust/rpc/src/lib.rs:1038` (`FspecServiceImpl::list_checkpoints`)
- `rust/git/src/ghost_commit.rs:637` (`list_all_ghost_checkpoints`)
- `rust/rpc/tests/checkpoint_transport_rpc362.rs` (transport test pattern, cap-200 test)
- `rust/fspec-core/tests/list_checkpoints.rs` (CLI regression guard)
- `components/status_dialog.rs` (counter line shape)
