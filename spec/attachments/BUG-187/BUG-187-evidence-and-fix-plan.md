# BUG-187 — GitStateWatcher pins a tokio worker (blocking git snapshot on async pool)

## What must be done

Move the expensive `GitState` capture off the async runtime so a slow
capture can never pin a `tokio-rt-worker` thread and starve everything
else (LLM stream dispatch, RPCs, the TUI's own tasks).

## Evidence (profile `/tmp/fspec-prof-0925`, 60s sample, 2026-09-19)

- Thread `140754` (tokio-rt-worker) spent **41,726 / 41,726 samples —
  100% of the whole capture window** inside a single
  `capture_git_state` call.
- Hot chain (counts from `sample.txt`):

```
codelet_core::git_state::GitStateWatcher::with_interval::closure   git_state.rs:269 (poll task)
 └ re_snapshot_and_publish                                          git_state.rs:318
    └ capture_git_state                                             git_state.rs:180
       └ collect_changed_files                                      git_state.rs:134
          └ codelet_git::change_type::get_staged_files_with_change_type   35,688
             └ codelet_git::status::get_staged_files                 35,638
                └ gix Tree::lookup_entry_by_path                    35,636
                   └ gix_odb cache miss → gix_pack decode_entry
                      └ resolve_delta → decompress_entry → zlib inflate  21,629
```

- Process was at ~120–128% CPU the whole window (`meta.txt`), i.e. one core
  pinned in pure inflate/decode work on a *single* 10-second tick's
  capture. `interval.set_missed_tick_behavior(MissedTickBehavior::Skip)`
  (`git_state.rs:266`) then fires the next tick immediately after the long
  capture returns → back-to-back captures → the worker stays pinned
  indefinitely.
- The same synchronous call also runs on the debouncer callback path
  (`git_state.rs:389`, `re_snapshot_and_publish` from the notify
  callback closure).

## Why it hangs

1. `capture_git_state` is pure synchronous git work (gix tree lookups,
   pack decoding/inflate, file reads, worktree walks) — none of it yields.
2. It is invoked **directly inside an async task** (the poll task spawned
   at `git_state.rs:264-271`), not via `tokio::task::spawn_blocking`.
3. A fresh `gix::open` handle per call means cold ODB caches, so every
   `lookup_entry_by_path` re-decodes objects from the 103 MB of packs
   (delta chains included). On the profiled repo one capture took longer
   than the entire 60-second sample window.

## Changes required

1. **Poll task** (`rust/core/src/git_state.rs`, the `with_interval`
   spawn block, lines ~264-271): wrap the capture in
   `tokio::task::spawn_blocking(move || re_snapshot_and_publish(...)).await`
   (or `tokio::task::spawn_blocking` per tick; the task itself stays on the
   async pool, the work moves to the blocking pool).
   - Guard against overlap: track a `bool`/flag (e.g.
   `std::sync::atomic::AtomicBool`) so a tick that fires while a capture
   is in flight is skipped instead of queuing (prevents unbounded backlog
   on the blocking pool).
2. **Debouncer callback** (`build_debouncer`, `git_state.rs:389`): the
   notify callback is sync-only — it must spawn the blocking work from a
   captured runtime handle (`tokio::runtime::Handle`) with the same
   in-flight skip guard, instead of calling `re_snapshot_and_publish`
   inline.
3. Keep `re_snapshot_and_publish` itself unchanged in behavior (dedup +
   broadcast stay where they are); only its *execution context* changes.

## Acceptance criteria (for the feature file)

- A `GitState` capture longer than the poll interval does NOT block any
  `tokio-rt-worker` thread: during a long capture, other async tasks on
  the runtime still make progress (test: instrument or use a repo where
  one capture takes >2× the interval and assert a concurrent
  `tokio::time`-driven task fires on schedule).
- Concurrent ticks are dropped, not queued: with a capture longer than
  the interval, at most one capture runs at a time.
- The fs-event (debouncer) path also runs the capture on the blocking
  pool (same guarantee as the poll path).

## Tests to add

- `rust/core/tests/git_state_watcher.rs`: add a test with a repo whose
  first capture is artificially slow (or use a `with_interval` of e.g.
  10 ms over a repo with a large pack) and a concurrent
  `tokio::time::interval` task asserting ticks still fire while a
  capture is in flight — proves the capture is not on the async pool.
- Reuse existing helpers from `rust/test-helpers/` for repo fixtures.

## Out of scope (separate cards)

- Making the capture cheap / dedup real → **BUG-188**, **BUG-189**.
- TUI busy-redraw cost → **BUG-190**.

## Related

- `spec/features/git-state-watcher.feature`, `git-state-refresh-stability.feature`
- BUG-182 (introduced the watcher), BUG-184 (TUI-side signature drop)
