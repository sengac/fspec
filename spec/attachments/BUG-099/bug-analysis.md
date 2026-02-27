# BUG-099: Re-merge after conflict resolution enters infinite loop

## Status: SPECIFYING — Integration tests written, all failing as expected (pre-fix)

## Quick Resume Guide

**To resume this work unit after context clear:**

1. `fspec show-work-unit BUG-099` — read rules, examples, architecture notes
2. Read this file for full analysis and implementation plan
3. Read the feature file: `spec/features/merge-conflict-resolution-loop.feature`
4. Run integration tests (all 7 should FAIL pre-fix, 1 PASS):
   ```bash
   cd codelet && cargo test -p codelet-git --test conflict_resolution_loop_tests -- --nocapture
   ```
5. Implement the fix in `codelet/git/src/session_result.rs` and `codelet/git/src/tree_utils.rs`
6. All 8 tests should PASS after fix

**DO NOT modify `codelet/git/src/three_way_merge.rs`** — Rule [3].

---

## Summary

After BUG-098 correctly fixed conflict marker writing into worktree files, a second bug remains: **the re-merge loop**. When the LLM resolves conflict markers and `/merge-worktree` runs again, `apply_session_changes()` re-detects the same files as conflicting, overwrites the LLM's resolution with fresh conflict markers, and the cycle repeats until the user kills the session.

The root cause is that fspec has no equivalent of `git add` — no mechanism to mark a conflict as resolved. Every `/merge-worktree` call is stateless, re-running `detect_conflicts()` from scratch against the original base commit.

---

## Reproduction

**Debug Session:** `bb90f15f-8707-4680-b435-20b4f7fc0bf5`
**Log File:** `~/.fspec/debug/session-2026-02-26T23-51-12.jsonl`
**Duration:** 76 seconds (user killed session after 3 cycles)

### Timeline from Debug Log

| Sequence | Time | Event | Detail |
|----------|------|-------|--------|
| 2 | 23:51:23 | api.request | First conflict prompt: "Merge conflicts were detected in the following files:\n  - README.md" |
| 4 | 23:51:26 | tool.call (Read) | LLM reads worktree README.md — finds conflict markers |
| 5 | 23:51:27 | tool.result (Read) | Success — BUG-098 fix works, markers are in the file |
| 24 | 23:51:35 | tool.call (Edit) | LLM resolves: `old_string` = full marker block, `new_string` = "**Ma Spec-Driven, Multi-Agent Coding Factory**" |
| 25 | 23:51:35 | tool.result (Edit) | Success — markers removed from worktree file |
| 43 | 23:51:59 | api.request | **SECOND** conflict prompt — byte-identical to first (same files, same message) |
| 45 | 23:52:03 | tool.call (Read) | LLM reads README.md again — finds **NEW** markers (identical to first time) |
| 63 | 23:52:12 | tool.call (Edit) | LLM resolves identically — exact same `old_string`/`new_string` as seq 24 |
| 64 | 23:52:12 | tool.result (Edit) | Success — but will be overwritten again |
| 80 | 23:52:23 | api.request | **THIRD** conflict prompt fires |
| 82 | 23:52:29 | session.end | User kills session (exitReason: "user") |

### Exact Strings from Debug Log

**Edit old_string (seq 24 and 63 — identical):**
```
<<<<<<< session (your changes)
**Ma Spec-Driven, Multi-Agent Coding Factory**
=======
**Those Spec-Driven, Multi-Agent Coding Factory**
>>>>>>> main
```

**Edit new_string (seq 24 and 63 — identical):**
```
**Ma Spec-Driven, Multi-Agent Coding Factory**
```

The LLM resolves by keeping session's version. Main has "Those". Neither matches base "The". On re-merge, `detect_conflicts()` re-fires because both sides still differ from base.

---

## Root Cause Analysis

### The Re-merge Loop Step by Step

**Initial state:**
- Base commit: `README.md` contains `"The Spec-Driven"`
- Session worktree: LLM changed to `"Ma Spec-Driven"`
- Main worktree: Someone changed to `"Those Spec-Driven"`

**First `/merge-worktree`:**
1. `apply_session_changes()` calls `detect_conflicts(base, worktree, main)`
2. `detect_conflicts()`: worktree(`"Ma"`) ≠ base(`"The"`) → session_changed=true. main(`"Those"`) ≠ base(`"The"`) → main_changed=true. Both changed → CONFLICT.
3. `write_conflict_markers()`: `three_way_merge_text("The", "Ma", "Those")` → overlapping change → writes markers into worktree `README.md`
4. Returns `ConflictError { files: ["README.md"] }`
5. TypeScript handler tells LLM: "resolve the git conflict markers"

**LLM resolves:**
- Reads README.md, sees markers, picks `"Ma Spec-Driven"` (session version)
- Edits file to remove markers, saves

**Second `/merge-worktree`:**
1. `apply_session_changes()` re-reads ALL files fresh
2. `detect_conflicts(base, worktree, main)`:
   - worktree(`"Ma"`) ≠ base(`"The"`) → session_changed=true ← **STILL TRUE** (LLM's resolution ≠ base)
   - main(`"Those"`) ≠ base(`"The"`) → main_changed=true ← **STILL TRUE** (main hasn't changed)
   - Both changed → CONFLICT **AGAIN**
3. `write_conflict_markers()`: `three_way_merge_text("The", "Ma", "Those")` → **SAME** overlapping change → writes **SAME** markers, **overwriting LLM's resolution**
4. Returns `ConflictError` again
5. Same message to LLM → same resolution → loop forever

### Why detect_conflicts() Always Re-fires

The comparison is always against the **original base commit**, which never changes:

```rust
// codelet/git/src/session_result.rs:254-291
fn detect_conflicts(
    base_tree_files: &HashMap<String, Vec<u8>>,    // ← frozen at session creation
    worktree_files: &HashMap<String, Vec<u8>>,      // ← re-read each time (changes after resolution)
    main_files: &HashMap<String, Vec<u8>>,          // ← re-read each time (unchanged)
) -> Vec<String> {
    for (path, base_content) in base_tree_files {
        let session_changed = worktree_files.get(path)
            .map(|c| c != base_content)             // ← resolved content ≠ base → always true
            .unwrap_or(true);
        let main_changed = main_files.get(path)
            .map(|c| c != base_content)             // ← main hasn't changed → always true
            .unwrap_or(false);
        if session_changed && main_changed {        // ← always true after resolution
            conflicts.push(path.clone());
        }
    }
}
```

### The Analogous Git Problem (and Git's Solution)

In real git: `git merge` → conflict → user edits → `git add file` (marks resolved in index) → `git commit`. Step 3 is the key: `git add` sets stage 0, removing conflict entries. After that, git **never re-checks**.

**fspec has no equivalent of step 3.** No staging area, no conflict resolution tracking. The `/merge-worktree` command is completely stateless.

---

## Integration Test Results (Pre-Fix Baseline)

**Test file:** `codelet/git/tests/conflict_resolution_loop_tests.rs`
**Run with:** `cd codelet && cargo test -p codelet-git --test conflict_resolution_loop_tests -- --nocapture`

### Results (2026-02-27, pre-implementation)

| # | Test | Result | What it proves |
|---|------|--------|----------------|
| 1 | `test_bug_099_exact_reproduction_from_debug_session` | ❌ FAIL | Confirms infinite loop — `ConflictError` re-fires after LLM resolution. Uses exact strings from debug session bb90f15f. |
| 2 | `test_bug_099_markers_regenerated_overwrite_resolution` | ❌ FAIL | Confirms fresh `<<<<<<< session` markers overwrite LLM's clean resolved content. |
| 3 | `test_first_merge_creates_pending_conflicts_state_file` | ❌ FAIL | `.fspec-pending-conflicts` doesn't exist yet — needs implementation. |
| 4 | `test_resolved_file_accepted_on_remerge` | ❌ FAIL | Re-merge returns `ConflictError` instead of accepting resolution. Also verifies resolved content is copied to main. |
| 5 | `test_unresolved_file_returns_error_without_overwrite` | ❌ FAIL | **Critical discovery: DOUBLE-NESTED MARKERS.** See below. |
| 6 | `test_multi_file_partial_resolution` | ❌ FAIL | Both files re-flagged, no partial resolution logic exists. |
| 7 | `test_resolution_matching_main_succeeds` | ✅ PASS | Existing three-way merge already handles worktree==main (returns Clean). No code change needed for this case. |
| 8 | `test_pending_conflicts_excluded_from_worktree_collection` | ❌ FAIL | `.fspec-pending-conflicts` collected as `files_added` in diff. |

### Critical Discovery: Double-Nested Markers (Test 5)

When markers are still present and re-merge runs, the three-way merge treats the marker-containing content AS the "session" input, producing **double-nested markers**:

```
line1
<<<<<<< session (your changes)
<<<<<<< session (your changes)
session-v
=======
main-v
>>>>>>> main
=======
main-v
>>>>>>> main
line3
```

This was NOT documented in the original bug report. It means re-merge doesn't just loop — it **corrupts** the file, making manual resolution impossible for the LLM. Each subsequent cycle adds another layer of nesting.

This discovery led to adding Rule [11] to the work unit.

### Reusable Test Fixtures

The test file provides two reusable fixtures:

- **`setup_single_file_conflict(session_id, filename, base, session, main)`** — Creates a real git repo, worktree, applies divergent changes, calls `apply_session_changes()` to trigger the first conflict, verifies markers were written. Returns a `ConflictFixture` ready for resolution testing.

- **`setup_multi_file_conflict(session_id, files)`** — Same but for multiple files. Takes array of `(filename, base, session, main)` tuples.

Both fixtures use real git repos via `common::setup_test_repo()` — no mocks.

---

## The Fix: Conflict State Tracking

### Approach: `.fspec-pending-conflicts` State File

Add a state file in the worktree to track which files have been conflict-marked. This enables `apply_session_changes()` to distinguish between:
- **First conflict detection**: no state file → write markers, create state file, return error
- **Re-merge after resolution**: state file exists → check if markers are gone → accept resolution

### New Flow in `apply_session_changes()`

```
apply_session_changes(repo_path, session_id):
    ...setup (read base tree, worktree files, main files)...

    // NEW STEP 1: Check for pending conflict state
    pending_state = read_pending_conflicts(worktree_path)

    if pending_state exists:
        // RE-MERGE PATH
        resolved = []
        still_pending = []

        for file in pending_state.files:
            content = read(worktree_path / file)
            if content contains "<<<<<<< ":
                still_pending.push(file)
            else:
                resolved.push(file)

        if still_pending is not empty:
            // Some files still have markers — tell LLM, DO NOT regenerate
            return Err(ConflictError { files: still_pending })

        // ALL conflicts resolved — proceed with apply
        delete(worktree_path / ".fspec-pending-conflicts")

        // Use resolved worktree content as-is (user's final answer)
        // Apply normally — resolved files bypass detect_conflicts entirely
        apply_worktree_to_main(base_tree_files, worktree_files, main_workdir)
    else:
        // FIRST-MERGE PATH (existing logic, lines 168-195 of session_result.rs)
        potential_conflicts = detect_conflicts(base, worktree, main)

        if potential_conflicts is not empty:
            actual_conflicts = write_conflict_markers(worktree_path, ...)

            if actual_conflicts is not empty:
                // NEW STEP 2: Write state file BEFORE returning ConflictError
                write_pending_conflicts(worktree_path, actual_conflicts)
                return Err(ConflictError { files: actual_conflicts })

            // Auto-resolved — fall through (re-read worktree)
            merged_worktree_files = collect_worktree_files(worktree_path)
            apply_worktree_to_main(base, merged_worktree_files, main_workdir)
        else:
            apply_worktree_to_main(base, worktree, main_workdir)

    // Remove session worktree
    remove_worktree(repo_path, session_id)
```

### State File Format

`.fspec-pending-conflicts` is a simple JSON file in the worktree root:

```json
{
  "files": ["README.md", "config.yml"],
  "created_at": "2026-02-26T23:51:35Z"
}
```

### New Functions to Add in `session_result.rs`

```rust
/// State file name for pending conflict tracking
const PENDING_CONFLICTS_FILE: &str = ".fspec-pending-conflicts";

/// Check if file content contains conflict markers
fn has_conflict_markers(content: &[u8]) -> bool {
    content.windows(8).any(|w| w == b"<<<<<<< ")
}

/// Read pending conflicts state from worktree
fn read_pending_conflicts(worktree_path: &Path) -> Option<Vec<String>> {
    let state_path = worktree_path.join(PENDING_CONFLICTS_FILE);
    if !state_path.exists() { return None; }
    let content = fs::read_to_string(&state_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let files = value["files"].as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    Some(files)
}

/// Write pending conflicts state to worktree
fn write_pending_conflicts(worktree_path: &Path, files: &[String]) -> Result<()> {
    let state_path = worktree_path.join(PENDING_CONFLICTS_FILE);
    let value = serde_json::json!({
        "files": files,
        "created_at": chrono::Utc::now().to_rfc3339()
    });
    fs::write(&state_path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}
```

### Change in `tree_utils.rs`

```rust
// codelet/git/src/tree_utils.rs:124
fn is_git_or_fspec_dir(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name();
    name == ".git" || name == ".fspec" || name == ".fspec-pending-conflicts"
}
```

Note: Despite the function name mentioning "dir", `filter_entry` in walkdir applies to both files and directories. The `.fspec-pending-conflicts` file will be skipped by this check. Consider renaming the function to `is_git_or_fspec_internal(entry)` for clarity.

---

## Files Affected

| File | Change | Lines |
|------|--------|-------|
| `codelet/git/src/session_result.rs` | Add pending conflict state check at top of `apply_session_changes()` (line 135). New functions: `read_pending_conflicts()`, `write_pending_conflicts()`, `has_conflict_markers()`. Add state file write after `write_conflict_markers()` returns conflicts (line 182). | ~60 lines added |
| `codelet/git/src/tree_utils.rs` | Add `.fspec-pending-conflicts` to `is_git_or_fspec_dir()` filter (line 126). | 1 line changed |
| `codelet/git/src/three_way_merge.rs` | **NO CHANGES** — merge logic is correct, bug is in the caller. | 0 |
| `codelet/napi/src/git.rs` | **NO CHANGES** — Rust API unchanged. | 0 |
| `src/tui/handlers/conflictLlmContext.ts` | **NO CHANGES** — TypeScript layer unchanged. | 0 |
| `src/tui/handlers/mergeWorktreeHandler.ts` | **NO CHANGES** — TypeScript layer unchanged. | 0 |

---

## Why BUG-098's "Re-merge" Test Didn't Catch This

BUG-098 has a test `test_re_merge_after_resolution_succeeds` (three_way_merge.rs:474) that tests `three_way_merge_text()` directly with `session_resolved == main`. Two flaws:

1. **Wrong layer**: Tests the merge function, not `apply_session_changes()`. Bug is in `detect_conflicts()` which fires BEFORE the merge.
2. **Unrealistic input**: Sets `session_resolved == main`, making merge return `Clean`. In reality, LLM often resolves to a value differing from main (e.g., keeps its own version).

The existing integration test `test_apply_re_merge_after_resolution_succeeds` (three_way_merge_integration_tests.rs:402) also cheats: it writes the resolved content to BOTH worktree AND main (line 447-451), so the three-way merge sees identical content and returns Clean. The real bug only manifests when the resolution differs from main.

**The BUG-099 integration tests test the FULL flow** with realistic inputs where session ≠ main after resolution.

---

## Edge Cases

### 1. LLM Resolves to Match Main Exactly
- `test_resolution_matching_main_succeeds` — **ALREADY PASSES** without code changes
- Existing `three_way_merge_text(base, main, main)` → `Clean` → auto-resolved
- The state-file fix also handles this: no markers → resolved → apply

### 2. Binary File Conflicts
- Binary files listed in `.fspec-pending-conflicts` too
- `has_conflict_markers()` returns false for binary → treated as resolved on re-merge
- Acceptable: no meaningful way to merge binary content

### 3. New Conflicts After Partial Resolution
- If main gains a NEW conflicting file between first and second `/merge-worktree`
- The new file won't be in `.fspec-pending-conflicts`
- On the re-merge path: resolved files are applied, then `detect_conflicts()` runs for remaining files → catches the new conflict
- The implementation must handle this hybrid path

### 4. Worktree Cleanup on Abort/Discard
- `abort_session()`/`discard_session()` remove the entire worktree directory
- `.fspec-pending-conflicts` is inside worktree → automatically cleaned up

### 5. Crash Recovery
- Crash between writing markers and creating state file → next merge starts fresh (re-detects, re-writes markers — safe)
- Crash after creating state file → next merge uses state → correct behavior

### 6. Double-Nested Markers (DISCOVERED BY INTEGRATION TEST)
- Without the fix, re-merge on an unresolved file produces markers-inside-markers
- Each cycle adds another nesting layer, corrupting the file
- The state-file check MUST run BEFORE `detect_conflicts()` to prevent this

---

## Call Chain (for understanding the code path)

```
TypeScript: handleMergeWorktree() (mergeWorktreeHandler.ts:70)
  → mergeSessionChanges() (sessionService.ts:400)
    → NAPI: mergeSession() (napi/src/git.rs:517)
      → Rust: merge_session() (session_status.rs:504)
        → get_session_diff()                           // captures what changed
        → apply_session_changes() (session_result.rs:135)  // ← THE BUG IS HERE
          → detect_conflicts() (session_result.rs:254)     // ← re-fires on re-merge
          → write_conflict_markers() (three_way_merge.rs:75) // ← overwrites resolution
          → ConflictError propagated up
      → TypeScript catches "Conflict" in error message
        → buildConflictSummary() for TUI display
        → buildConflictLlmContext() injected to LLM
          → LLM resolves → /merge-worktree again → LOOP
```

---

## Scenario ↔ Test ↔ Rule Cross-Reference

| Feature Scenario (line) | Integration Test | Rules Covered |
|---|---|---|
| Re-merge does not enter infinite loop (L52) | `test_bug_099_exact_reproduction_from_debug_session` | [0], [4], [10] |
| First merge creates state file (L67) | `test_first_merge_creates_pending_conflicts_state_file` | [10] |
| Resolved file accepted (L77) | `test_resolved_file_accepted_on_remerge` | [6], [8] |
| Unresolved file no overwrite (L88) | `test_unresolved_file_returns_error_without_overwrite` | [7], [11] |
| Multi-file partial resolution (L98) | `test_multi_file_partial_resolution` | [7] |
| Resolution matches main (L107) | `test_resolution_matching_main_succeeds` | [0] |
| State file excluded from collection (L115) | `test_pending_conflicts_excluded_from_worktree_collection` | [9] |
| Double-nested markers corruption (L121) | `test_bug_099_markers_regenerated_overwrite_resolution` | [11] |

---

## Related Work Units

| ID | Title | Relationship |
|----|-------|-------------|
| BUG-098 | Merge conflict markers never written to worktree files | Parent bug — fixed marker writing, missed the loop |
| GIT-036 | /merge-worktree slash command | Implements the handler that triggers this code path |
| GIT-038 | Conflict details injected to LLM via injectLlmContext | Builds the conflict guidance message |
