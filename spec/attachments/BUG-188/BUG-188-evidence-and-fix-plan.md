# BUG-188 — GitStateWatcher dedup never fires (volatile checkpoint timestamps)

## What must be done

Make the watcher's "only broadcast when the snapshot differs" guarantee
actually hold. Today the equality check compares the whole `GitState`,
including `checkpoints[].timestamp`, and every capture stamps all
checkpoints with a fresh `SystemTime::now()` — so two captures of an
*unchanged* repo are never equal, and **every 10 s tick (and every
debounced `.git` fs-event) re-broadcasts a "changed" frame**.

## Evidence

- `rust/core/src/git_state.rs:116-121` (`collect_checkpoints`):
  `timestamp: fallback_timestamp()` — the index sidecar is never read,
  so *every* checkpoint row gets "now" (millisecond precision) on every
  capture.
- `rust/core/src/git_state.rs:318-329` (`re_snapshot_and_publish`):
  `if *guard == snapshot { return; }` — full `PartialEq` on `GitState`,
  which includes the volatile timestamps (see
  `rust/rpc-types/src/lib.rs:98-107`).
- Consequence: the dedup branch is dead code in real repos. Every tick
  sends a frame; the TUI subscriber (`bootstrap_git_state.rs:36`) pushes
  `Action::GitStateChanged` onto the bus every 10 s;
  `handle_git_state_changed` (`dispatch_git_state.rs:35`) re-sends
  `CheckpointCountsLoaded` every tick and, when the Files/Checkpoints
  views are visible, attempts a re-fetch (BUG-184's signature drop at
  `dispatch_git_state.rs:121/142` is what keeps the re-*fetch* from
  running — but only because the TUI layer already knew to exclude the
  timestamp leg from its signature, `dispatch_git_state.rs:216-233`).

## Changes required

Pick one (option A is the smaller, option B is the cleaner):

**A — Sign at the watcher (recommended):**
1. In `re_snapshot_and_publish`, compare a *stable signature* of the new
   snapshot against the previously published one, instead of `==` on the
   full struct. The signature must mirror the TUI's existing one so both
   layers agree on "unchanged":
   - `git_branch`, `checkpoint_counts`,
   - `changed_files` in list order (`path:change_type:staged`),
   - `checkpoints` as `(work_unit_id, name, is_automatic)` in list order
     — **excluding `timestamp`** (same rule as
     `checkpoints_signature` in `dispatch_git_state.rs:220`).
2. Keep the full `GitState` stored/served by `snapshot()` unchanged
   (subscribers still get fresh "now" stamps; only the *broadcast
   decision* uses the stable signature).

**B — Stop stamping "now" in the watcher:**
1. `collect_checkpoints` emits a fixed epoch (e.g. the watcher's start
   time, or the checkpoint's last-modified ref time if cheaply available)
   so the timestamp leg is stable across captures; the existing `==`
   dedup then works.
2. Verify `CheckpointInfo.timestamp` consumers (TUI row rendering) don't
   rely on freshness — `checkpoints_signature` excludes it, and row
   rendering doesn't show it, so this is safe, but pin it in a test.

Option A additionally removes the need for the TUI-side signature drop on
the watcher's own frames (BUG-184's drop stays as a second line of
defense for transport-supplied frames).

## Acceptance criteria

- With a repo whose git state is unchanged, the watcher broadcasts at
  most one frame (the initial one); steady-state 10 s ticks produce zero
  broadcasts (assert on a `broadcast::Receiver`: `try_recv` stays
  empty after N ticks).
- A real change (staging a file, creating a checkpoint ref) produces a
  new frame within one tick (or one debounced fs-event).
- `snapshot()` still returns a full `GitState` with sane timestamps.

## Tests to add

- `rust/core/tests/git_state_watcher.rs`:
  - steady-state silence: `with_interval(repo, 250ms)`, advance past
    several ticks, assert no further frames after the initial broadcast.
  - change detection still works: stage/commit a change, assert a new
    frame arrives with the expected `changed_files` leg.
  - (option A) two captures of an unchanged repo produce equal
    signatures while their raw `GitState` values differ in
    `checkpoints[].timestamp` — pins the bug's root cause.

## Related

- `spec/features/git-state-watcher.feature` (dedup rule R1),
  `git-state-refresh-stability.feature`
- BUG-182 (introduced watcher + dedup), BUG-184 (TUI signature drop),
  BUG-187 (blocking execution), BUG-189 (capture cost)
