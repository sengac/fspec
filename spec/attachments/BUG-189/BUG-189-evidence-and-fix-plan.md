# BUG-189 — Git capture is O(repo size) (cold handles, double lookups, per-file hashing)

## What must be done

Make a single `GitState` capture cheap enough that it is fast even when it
*does* run (BUG-187 moves it off the async pool; BUG-188 stops it running
pointlessly — this card makes the run itself affordable). Four independent
sources of waste in `codelet-git`:

## Evidence (profile `/tmp/fspec-prof-0925` + code)

1. **Cold gix handle per call → repeated pack decode.**
   Every collector (`status.rs`, `change_type.rs`) calls
   `open_repo(dir)` → `gix::open(path)` (`rust/git/src/lib.rs:58`), a
   fresh handle with an **empty in-memory ODB cache**. So the same
   HEAD-tree objects get re-decoded from the 103 MB pack set on every
   capture. Profile: thread `140754` shows
   `gix_pack::decode_entry → resolve_delta → decompress_entry → zlib_rs
   inflate` = **21,629 / 41,726 samples** (≈52% of the whole window).

2. **Double `lookup_entry_by_path` per staged file.**
   `get_staged_files_with_change_type` (`change_type.rs:53`) calls
   `status::get_staged_files` (which already does
   `tree.lookup_entry_by_path` per index entry, `status.rs:44` to decide
   `is_staged`), then does a **second** `tree.lookup_entry_by_path` per
   staged path (`change_type.rs:71`) to re-derive "in HEAD?" for the
   change type. Same tree, decoded twice.

3. **Full-file sha1 over every indexed file.**
   `get_unstaged_files` (`status.rs:68`) does `std::fs::read` +
   `gix::objs::compute_hash(sha1)` on **every** indexed file to compare
   against the index entry — O(working-tree size), every capture.

4. **Whole-worktree walk per capture.**
   `get_untracked_files` (`status.rs:109`) does
   `walkdir::WalkDir::new(workdir)` over the entire worktree, every
   capture, just to find untracked files.

## Changes required

Prioritized (each is independently shippable):

1. **Share one `gix::Repository` per capture** — open the repo once in
   `capture_git_state` (`git_state.rs:180`) and pass the handle into the
   staged/unstaged/untracked collectors (add a `*_with_repo` internal
   variant that takes `&gix::Repository`; keep the existing
   `dir: impl AsRef<Path>` public fns as thin wrappers for other
   callers). This alone removes the cold-cache re-decode for the *three*
   collectors sharing one capture.
   - Stretch: cache the handle across ticks keyed by `.git` mtime, so
     steady-state captures reuse a warm ODB cache. (Keep this optional /
     behind the BUG-188 signature gate so it only matters when a capture
     actually runs.)

2. **Merge the two lookup passes.** `get_staged_files` should return the
   per-path verdict (in-HEAD / new) alongside the staged list, so
   `get_staged_files_with_change_type` reuses it instead of re-issuing
   `lookup_entry_by_path`. (Or compute change-type inline inside a single
   pass.)

3. **Stat-first unstaged check.** In `get_unstaged_files`, before
   reading + hashing a file, compare the file's `mtime`/`size` against
   the index entry's stat fields (git does exactly this). Only hash when
   the stat matches are unknown/changed. This turns the common case
   (file unchanged) into a cheap `stat` syscall.

4. **Gate the untracked walk.** Only run `get_untracked_files` when a
   worktree fs-event actually landed (the debouncer already distinguishes
   `.git` events from worktree events — extend the filter at
   `git_state.rs:370-388`) or on the very first capture; skip it on
   steady-state ticks where nothing touched the worktree. (This is the
   worktree-event half of the BUG-187/188 design.)

## Acceptance criteria

- A `GitState` capture on a repo with ≥5k tracked files and a multi-MB
  pack set completes in well under the poll interval (target: <500 ms in
  debug) for the steady-state "nothing changed" case.
- `get_staged_files_with_change_type` issues exactly **one**
  `lookup_entry_by_path` per staged path (assert via a counting gix
  trait wrapper or by measuring pack-decode calls / object lookups).
- `get_unstaged_files` performs no `fs::read` + hash for a file whose
  mtime+size match the index (assert by pointing the index at a file and
  verifying no read happens when the stat matches).
- Sharing one repo handle across the three collectors does not change
  any public result (existing `codelet-git` tests stay green).

## Tests to add

- `rust/git/tests/` (or existing status/change_type tests):
  - staged-with-change-type returns the same list as before, and a
    lookup-count assertion proves the double-pass is gone.
  - unstaged detection: unchanged file (stat matches) → not in the list
    and no full read; modified file → detected.
  - capture-with-shared-handle: a capture runs staged+unstaged+untracked
    against one `gix::Repository` and matches the per-call results.
- Reuse `rust/test-helpers/` repo fixtures.

## Out of scope

- Where the capture runs (blocking vs async) → **BUG-187**.
- Whether the capture runs at all (dedup) → **BUG-188**.
- TUI repaint cost → **BUG-190**.

## Related

- `rust/git/src/status.rs`, `rust/git/src/change_type.rs`,
  `rust/git/src/lib.rs` (`open_repo`), `rust/core/src/git_state.rs`
  (`capture_git_state`)
- BUG-182/187/188/190 (same performance epic)
