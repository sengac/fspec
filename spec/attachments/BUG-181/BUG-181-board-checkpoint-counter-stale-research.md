# BUG-181: Board header checkpoint counter goes stale after checkpoint add/remove

## Problem

The BoardView header row 0 paints a checkpoint status line:

- `Checkpoints: None` when both counts are 0, otherwise
- `Checkpoints: {manual} Manual, {auto} Auto`

This line is painted from `BoardStore::checkpoint_counts()`
(`rust/fspec-tui/src/views/board/checkpoint_status.rs` →
`header.rs:76`), a field that is only ever written by
`BoardStore::set_checkpoint_counts`, which is only called from the
`Action::CheckpointCountsLoaded` arm of `App::dispatch`
(`rust/fspec-tui/src/app/dispatch.rs:80-85`).

`CheckpointCountsLoaded` is produced in exactly **three** places:

1. **Bootstrap, once** — `App::bootstrap` (`app/bootstrap.rs:34-41`)
   calls `backend.checkpoint_counts()` and dispatches the result.
2. **After an in-TUI restore** — `Action::RefreshCheckpointCounts` →
   `App::spawn_refresh_checkpoint_counts`
   (`app/dispatch_checkpoint_restore.rs:90-102`), emitted by
   `CheckpointsView::on_restore_result` on success
   (`views/checkpoints/restore.rs:137`).
3. **After an in-TUI delete** — the same `RefreshCheckpointCounts`
   action, emitted by `CheckpointsView::on_delete_result` on success
   (`views/checkpoints/delete.rs:165`).

**There is no other producer.** Any checkpoint mutation that happens
*outside* those two in-TUI flows leaves the header painted with the
counts from bootstrap (or the last in-TUI restore/delete). The user
reports "the counter isn't updating when a checkpoint is added/removed
in the board view" — this is exactly that gap.

### Reproduction paths

| # | Trigger | What the header shows | What it should show |
|---|---------|----------------------|---------------------|
| 1 | Agent tool call `fspec checkpoint <WU> <name>` (manual add, in-process via `codelet_fspec_core::dispatch_command` at `agent_loop.rs:623`) | stale (bootstrap value) | +1 manual |
| 2 | Agent tool call `fspec update-work-unit-status <WU> <new>` (auto-checkpoint before transition, `update_work_unit_status.rs:298-302`; also cleanup on →done, `:366-369`) | stale | auto +1 / auto -N |
| 3 | Another terminal: `fspec checkpoint` / `fspec update-work-unit-status` / `fspec cleanup-checkpoints` / `fspec restore-checkpoint` | stale | updated |
| 4 | Direct git ops: `git update-ref refs/fspec-checkpoints/<WU>/<name> <sha>` / `git update-ref -d …` | stale | updated |
| 5 | In-TUI restore (Checkpoints view, `r`/`t`) | **updates** ✅ | — |
| 6 | In-TUI delete (Checkpoints view, `d`/`a`) | **updates** ✅ | — |

Rows 1-4 are the bug. The user's "added/removed" observation covers
paths 1-4; the "another state issue" phrasing points at the same
class as BUG-180 (stale UI state after an external mutation of the
single source of truth — work units there are kept live by
`WorkUnitsWatcher` file watching; checkpoints have NO equivalent).

---

## Data flow: how the counter reaches the screen

```
refs/fspec-checkpoints/<WU>/<name>   (git refs — the actual checkpoints)
            │  count_checkpoints(cwd)        (codelet_git::ghost_commit, git/src/ghost_commit.rs:52)
            ▼
FspecService::checkpoint_counts      (rpc/src/lib.rs:1063, gated on SharedFspecService::cwd)
            │  tarpc (embedded) / tarpc-over-WebSocket (remote)
            ▼
FspecBackend::checkpoint_counts      (fspec-tui/transport/{embedded,websocket}.rs)
            │
            ▼  (only when: bootstrap | RefreshCheckpointCounts)
App::dispatch → Action::CheckpointCountsLoaded → BoardStore::set_checkpoint_counts
            │
            ▼  (every frame)
BoardView::render_with_store → board/header.rs → checkpoint_status::render
```

The read side is a plain `&BoardStore` borrow; the header repaints
every frame, so **the render path is not the problem** — the *state
update* never reaches the store.

For contrast, `WorkUnitsWatcher` (core/src/work_units.rs:210) watches
`spec/work-units.json`, re-reads on a debounced fs event, and
broadcasts full snapshots on a `broadcast::Sender`
(`watcher_rx()`), which both the embedded transport
(`rpc-embedded/src/lib.rs:86`) and the WS server fan-out
(`rpc-server/src/server.rs:153-161` → `Envelope::WorkUnitsUpdate`)
deliver to the App's subscriber task (`app/bootstrap.rs:145-165` →
`Action::WorkUnitsLoaded`). That is why the board columns *do* update
after `update-work-unit-status` while the checkpoint counter does not.
Checkpoints have NO watcher: refs live under `.git/` (which nothing
watches) and the TS `checkpoint-changed` IPC push was dropped in the
Rust port.

---

## Root cause

1. **Primary — no push channel for checkpoint mutations.**
   The TS reference (TUI-016, spec
   `refactor-checkpoint-counts-to-be-command-triggered-instead-of-file-watching.feature`,
   @done) closed this exact gap by having the checkpoint commands
   (`checkpoint.ts`, `update-work-unit-status.ts`, `restore-checkpoint.ts`,
   `cleanup-checkpoints.ts`) send a `checkpoint-changed` IPC message to
   the running TUI after every mutation (Unix socket / Windows named
   pipe, cross-process), which triggered `loadCheckpointCounts()` in
   the zustand store. The Rust port dropped the push entirely:
   - `fspec-core/src/commands/checkpoint.rs:23-25` documents the IPC
     send as *"a documented NO-OP"* in the Rust standalone binary.
   - `update_work_unit_status.rs` (auto-checkpoint + done-cleanup),
     `cleanup_checkpoints.rs`, and `restore_checkpoint.rs` never notify
     anyone either.
   - The only Rust "push" today is the two in-TUI follow-ups
     (`RefreshCheckpointCounts`), which cover neither tool-driven
     mutations nor cross-terminal mutations.

2. **Secondary — the in-TUI counter refresh is also *asymmetric*.**
   The Checkpoints view re-lists itself on open (fresh
   `list_checkpoints`), and in-view delete/restore already update the
   rows locally. But the board header has NO equivalent lazy refetch:
   - opening/closing the Checkpoints view
     (`handle_open_checkpoints_view`, `app/dispatch_checkpoints.rs:26-29`)
     does **not** touch the counts;
   - `RefreshCheckpointCounts` is the ONLY writer besides bootstrap.
   So the header only ever moves when a restore/delete succeeds
   *while the view is open*. The fix should make the header
   self-refreshing independent of the view's follow-ups.

3. **Process boundary.** Agent tool calls run IN-PROCESS (the agent
   loop's fspec handler → `codelet_fspec_core::dispatch_command`,
   `agent_loop.rs:607-641`), but a `fspec ...` invocation from another
   terminal (or a raw `git update-ref`) is a separate process — an
   in-memory broadcast alone cannot cross that boundary. The TS fix
   used a local socket; the Rust-native equivalent is a
   **filesystem watch** on the checkpoint ref/index locations (the
   `WorkUnitsWatcher` pattern), which covers in-process, cross-process,
   AND raw-git mutations with a single mechanism.

4. **RPC-compliance gaps found along the way** (the "make sure it's RPC
   compliant" part of the report):

   a. **`FspecBackend` has no push surface for checkpoint changes.**
      The trait (`fspec-tui/src/transport/mod.rs`) exposes only
      request/response methods (`checkpoint_counts`, `list_checkpoints`,
      …). It has no `*_rx()` sibling for checkpoint *changes*, unlike
      `work_units_rx` / `chunks_rx` / `status_changes_rx` /
      `checkpoints_progress_rx`. A push-driven fix cannot be expressed
      transport-agnostically until a method such as
      `checkpoint_counts_changed_rx()` is added to the trait **with a
      closed-receiver default impl** (mirroring
      `checkpoints_progress_rx`, mod.rs:995-999), so test doubles and
      the WebSocket transport compile unchanged.

   b. **Cross-transport parity is untested for the push direction.**
      Existing cross-transport tests
      (`fspec-tui/tests/checkpoint_counts_rpc015.rs`) only prove
      request/response parity for `checkpoint_counts`. Any new push
      channel needs an RPC-015-style parity scenario: embedded
      delivers a frame end-to-end, WebSocket degrades (closed
      receiver, no crash), and the App folds the frame into
      `BoardStore` on both transports.

   c. **The existing `RefreshCheckpointCounts` flow has NO App-level
      integration test.** The view-level tests
      (`views/checkpoints/delete_tests.rs:151-187`,
      `restore_tests.rs:174-179`) assert the *view emits*
      `RefreshCheckpointCounts`; the dispatch tests
      (`tests/checkpoint_delete_dispatch_rpc366.rs`,
      `checkpoint_restore_dispatch_rpc365.rs`) assert the *transport
      call + result folding*. Nothing in between asserts that
      `RefreshCheckpointCounts` actually reaches
      `backend.checkpoint_counts()` and lands in
      `BoardStore.checkpoint_counts()`. `MockBackend` already has
      `set_checkpoint_counts` + `checkpoint_counts_calls`
      (`tests/common/mod.rs:284,1198-1206,2674`) — the test seam is
      ready; the test itself is missing.

   d. **Auto-checkpoint writes no index entry (minor, pre-existing).**
      `maybe_auto_checkpoint` (`update_work_unit_status.rs:1214-1249`)
      only runs `git stash create` + `git update-ref` and never appends
      to `.git/fspec-checkpoints-index/<WU>.json`. Consequence:
      `collect_checkpoints_stream` (`rpc/src/checkpoints.rs:65-68`)
      falls back to `fallback_timestamp()` (current wall-clock) for
      auto-checkpoints, which breaks newest-first ordering once the
      index is partially present. This does not affect the counts
      (counting is ref-based) but it affects the Checkpoints view's
      ordering — worth a line in the card.

---

## What needs fixing (proposed scope for BUG-181)

### A. Core fix — push the counts after every checkpoint mutation

Recommended design (Rust-native, single mechanism, reuses an
established pattern): **a checkpoint watcher** mirroring
`WorkUnitsWatcher`, owned by the shared service layer:

1. New `CheckpointsWatcher` (core crate, next to
   `WorkUnitsWatcher`): debounced `notify` watch over the checkpoint
   locations — `.git/refs/fspec-checkpoints/` (recursive),
   `.git/fspec-checkpoints-index/` (recursive), and `.git/packed-refs`
   — re-running `codelet_git::ghost_commit::count_checkpoints(cwd)` on
   each debounced event and publishing the fresh
   `CheckpointCounts` on a `broadcast::Sender` (+ `snapshot()` for
   backfill, exactly like `WorkUnitsWatcher`). Missing `.git` /
   non-repo cwd → zero counts, no error (graceful degradation,
   matching `count_checkpoints`' ENOENT tolerance).
2. `SharedFspecService` exposes `checkpoints_changed_rx()` /
   `checkpoints_counts_snapshot()` (mirror `watcher_rx()` /
   `watcher_snapshot()`, rpc/src/lib.rs:856-863).
3. Embedded transport forwards the receiver directly
   (`rpc-embedded/src/lib.rs`, like `work_units_rx` :86-88).
4. `FspecBackend` gains
   `fn checkpoint_counts_changed_rx() -> broadcast::Receiver<CheckpointCounts>`
   with a **closed-receiver default** (mod.rs:995-999 pattern) —
   RPC-compliance gap 3a. WebSocket transport returns the closed
   receiver for now (documented degradation; the remote TUI can still
   get fresh counts on Checkpoints-view open, see B2) — OR a new
   `Envelope` variant fans the frames out like
   `Envelope::WorkUnitsUpdate` (rpc-server/src/server.rs:147-161).
   Pick one in specifying; the feature file must pin it.
5. `App::spawn_subscriber_tasks` (app/bootstrap.rs) gains subscriber
   (g): `checkpoint_counts_changed_rx` →
   `Action::CheckpointCountsLoaded(counts)` — reusing the EXISTING
   action variant and `BoardStore::set_checkpoint_counts` (no new
   Action, no second writer path). `handle_reconnected`
   (dispatch_reconnect.rs:135) keeps working because it reuses
   `spawn_subscriber_tasks`.

This single mechanism covers ALL four reproduction paths:
in-process tool calls (ref files written on disk → watcher fires),
cross-terminal CLI (same), and raw `git update-ref` (same).
Debounce latency (~100ms, same as `WorkUnitsWatcher`) is
imperceptible for a header repaint.

**Interim / complementing mitigation (cheap, do either way):**
re-poll `backend.checkpoint_counts()` on `OpenCheckpointsView` /
`CloseCheckpointsView` (the App dispatcher already owns these arms —
`try_dispatch_checkpoints`, dispatch_checkpoints.rs:26-29 +
`CloseCheckpointsView` :250) so that even on the degraded WebSocket
transport the header is correct when the user leaves the Checkpoints
view. Keep the existing `RefreshCheckpointCounts` in-view follow-ups
(RPC-365/366) unchanged — they remain correct and give immediate
feedback while the view is open.

### B. RPC-compliance fixes (required regardless)

- New `FspecBackend::checkpoint_counts_changed_rx()` default method
  (closed receiver) — gap 3a.
- Cross-transport parity test: embedded delivers a frame end-to-end
  into `BoardStore`; WebSocket returns a closed/empty receiver and
  the App degrades without panicking — gap 3b.
- App-level integration test for the EXISTING
  `RefreshCheckpointCounts` → `checkpoint_counts()` →
  `BoardStore.checkpoint_counts()` path (delete + restore) using
  `MockBackend` — gap 3c.
- Source-shape test (mirror `tests/source_shape_rpc015.rs`) asserting
  the trait method exists on `FspecBackend`, the service exposes the
  rx accessor, and `app/bootstrap.rs` subscribes.
- Document the WS degradation decision in the feature file docstring,
  mirroring TUI-109's documented `checkpoints_progress_rx`
  degradation.

### C. Nice-to-have (call out, likely separate card)

- Auto-checkpoint index-entry write in `maybe_auto_checkpoint` so
  Checkpoints-view ordering is reliable (gap 3d).
- (Optional) fan the checkpoint-changed frames over the WebSocket
  transport as a new `Envelope` variant for remote-TUI parity.

---

## Files touched (expected)

| Crate / file | Change |
|---|---|
| `rust/core/src/` (new module, e.g. `checkpoints_watcher.rs`) | `CheckpointsWatcher` — debounced fs watch → `count_checkpoints` → broadcast |
| `rust/rpc/src/lib.rs` | `SharedFspecService` accessors (`checkpoints_changed_rx`, snapshot) |
| `rust/rpc-embedded/src/lib.rs` | forward the receiver (like `work_units_rx`) |
| `rust/fspec-tui/src/transport/mod.rs` | `FspecBackend::checkpoint_counts_changed_rx` default |
| `rust/fspec-tui/src/transport/embedded.rs` | forward the service receiver |
| `rust/fspec-tui/src/transport/websocket.rs` | closed receiver (or envelope fan-out) |
| `rust/fspec-tui/src/app/bootstrap.rs` | subscriber task (g) + `handle_reconnected` reuse |
| `rust/fspec-tui/src/app/dispatch_checkpoints.rs` | re-poll counts on Open/CloseCheckpointsView (degraded-transport mitigation) |
| `rust/fspec-tui/tests/` | new integration tests + source-shape tests |
| `spec/features/` | new feature file (capability name, e.g. `live-checkpoint-counts-in-board-header.feature`) |

## Precedents to copy

- **File watcher → broadcast → App fold**: `WorkUnitsWatcher`
  (`core/src/work_units.rs:210-330`) + subscriber task (a)
  (`app/bootstrap.rs:145-165`) + `Envelope::WorkUnitsUpdate` WS fan-out
  (`rpc-server/src/server.rs:147-161`).
- **Push-broadcast + closed-receiver default + stale-drop**: TUI-109
  `checkpoints_progress_rx` (`transport/mod.rs:986-999`,
  `app/bootstrap.rs:252-276`, `views/checkpoints/mod.rs:81-94`).
- **In-TUI follow-up refresh**: RPC-365/366 (`RefreshCheckpointCounts`)
  — keep as-is, it already works.
- **Bug-card + research-attachment pattern**: BUG-180
  (`spec/attachments/BUG-180/bug-180-findings.md`), BUG-179
  (`spec/attachments/BUG-179/bug179-research.md`).

## Verification plan (when implementing)

1. Unit: `count_checkpoints` unchanged (already covered by
   `git/tests/count_checkpoints_rpc015.rs`).
2. Watcher unit tests (temp git repo): create ref → broadcast delivers
   incremented counts; delete ref → decremented; non-repo dir → zeros;
   packed-refs change → refreshed. (Reuse `rust/test-helpers` fixtures.)
3. Integration (fspec-tui):
   - embedded: `create_ghost_commit` in a temp repo via the real
     embedded transport → `App` folds →
     `BoardStore.checkpoint_counts()` increments.
   - delete: `delete_one` → count decrements.
   - WS: receiver closed → no panic, counts stay at bootstrap value.
   - existing flow: `DeleteCheckpoint` → `RefreshCheckpointCounts` →
     `BoardStore` (closes gap 3c).
4. Manual: launch TUI on a dirty repo, run an agent turn that calls
   `fspec checkpoint`, confirm the header flips from
   `Checkpoints: None` to `Checkpoints: 1 Manual` without reopening;
   repeat with `update-work-unit-status` (auto) and a second-terminal
   `fspec cleanup-checkpoints`.
5. `fspec validate` + `fspec validate-tags` + clippy on all touched
   crates; no unscoped `cargo test` (use
   `cargo test -p codelet-fspec-tui --test <name>` scoping).
