# BUG-098: Merge Conflict Markers Never Written to Worktree Files

## Summary

When `/merge-worktree` detects conflicts, the LLM is told to "resolve the git conflict markers (`<<<<<<< / ======= / >>>>>>>`)" but **no conflict markers actually exist in the files**. The Rust merge code detects divergence but never performs a three-way merge to materialize the markers.

---

## Reproduction

**Session:** `9f4fec89-17fc-4e4d-a212-3e2d8a37a661`  
**Date:** 2026-02-26T22:57:56Z  
**Title:** *"change @README.md to say 'Da Spec-Driven' instead of 'The Spec-Driven'"*

### Steps

1. User opens an isolated session (worktree at `.fspec/worktrees/9f4fec89.../`)
2. User asks: *"change @README.md to say 'Da Spec-Driven' instead of 'The Spec-Driven'"*
3. LLM edits `README.md` in the worktree — changes "The Spec-Driven" → "Da Spec-Driven" on line 7
4. Meanwhile, main branch's `README.md` has also been modified since the session's base commit (diverged)
5. User runs `/merge-worktree`
6. System detects the conflict (README.md changed in both session and main)
7. System injects message to LLM:
   > "Merge conflicts were detected in the following files:
   >   - README.md
   > The files are in the worktree at: /home/rquast/projects/fspec/.fspec/worktrees/9f4fec89...
   > Please read each conflicting file, resolve the git conflict markers (<<<<<<< / ======= / >>>>>>>), and save the file.
   > After resolving all conflicts, run /merge-worktree again."
8. **LLM reads README.md — finds NO conflict markers**
9. LLM searches with grep for `<<<<<<<|=======|>>>>>>>` — **"No matches found"**
10. LLM is stuck: "The file doesn't appear to have any conflict markers currently"

---

## Root Cause Analysis

### The conflict detection flow

```
handleMergeWorktree() [mergeWorktreeHandler.ts]
  → mergeSessionChanges() [sessionService.ts]
    → merge_session() [codelet/napi/src/git.rs]
      → codelet_git::merge_session() [codelet/git/src/session_status.rs:504]
        → apply_session_changes() [codelet/git/src/session_result.rs:128]
          → detect_conflicts() [codelet/git/src/session_result.rs:213]
            → returns Vec<String> of conflicting file NAMES only
          → if !conflicts.is_empty() → return Err(ConflictError { files })  ← BAILS HERE
```

### What `detect_conflicts()` does (session_result.rs:213-250)

```rust
fn detect_conflicts(
    base_tree_files: &HashMap<String, Vec<u8>>,
    worktree_files: &HashMap<String, Vec<u8>>,
    main_files: &HashMap<String, Vec<u8>>,
) -> Vec<String> {
    // For each file in base:
    //   if session changed it AND main changed it → conflict
    // Also: if session added a file that exists in main with different content → conflict
    // Returns: list of file PATHS only
}
```

This is a **divergence check** — it detects THAT a conflict exists, but does not produce conflict markers. The function returns file names, not merged content.

### What `apply_session_changes()` does (session_result.rs:128-195)

```rust
pub fn apply_session_changes(...) -> Result<()> {
    // ... setup ...
    let conflicts = detect_conflicts(&base_tree_files, &worktree_files, &main_files);
    if !conflicts.is_empty() {
        return Err(GitError::ConflictError { files: conflicts });  // ← IMMEDIATE RETURN
    }
    // ... copy files, remove deleted, cleanup worktree ...
}
```

When conflicts are detected, it **returns immediately** without:
- Writing conflict markers into the worktree files
- Performing any three-way merge
- Modifying any files at all

The worktree files still contain the session's clean edits (no markers). The main worktree has its own clean version (no markers). Nobody writes the `<<<<<<<` / `=======` / `>>>>>>>` markers.

### What the TypeScript handler does (mergeWorktreeHandler.ts:120-132)

```typescript
catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (errorMessage.includes('Conflict')) {
        addStatusMessage(ctx, buildConflictSummary(errorMessage));
        ctx.injectLlmContext(
            buildConflictLlmContext(errorMessage, ctx.worktreePath)
        );
    }
}
```

It catches the error, shows a TUI summary, and tells the LLM to go resolve markers — **but the markers were never created**.

---

## The Fix

### Option A: Three-way merge in Rust (Recommended)

Modify `apply_session_changes()` in `codelet/git/src/session_result.rs` so that when conflicts are detected, it performs a **three-way merge** for each conflicting file before returning the error:

```rust
if !conflicts.is_empty() {
    // Before returning error, write conflict markers into worktree files
    for conflict_path in &conflicts {
        let base_content = base_tree_files.get(conflict_path);
        let session_content = worktree_files.get(conflict_path);
        let main_content = main_files.get(conflict_path);

        if let (Some(base), Some(session), Some(main)) =
            (base_content, session_content, main_content)
        {
            let merged = three_way_merge(base, session, main);
            let dest = worktree_path.join(conflict_path);
            fs::write(&dest, merged)?;
        }
    }
    return Err(GitError::ConflictError { files: conflicts });
}
```

The `three_way_merge()` function should produce standard git conflict markers:

```
<<<<<<< session (your changes)
Da Spec-Driven, Multi-Agent Coding Factory
=======
The Spec-Driven, Multi-Agent Coding Factory (v2.0)
>>>>>>> main
```

This can be implemented using the `similar` crate (already a dependency) or the `diffy` crate for three-way merge support. The `imara-diff` crate (used by gitoxide internally) could also work.

### Option B: Simpler — present both versions in LLM context

If a three-way merge implementation is too complex, alternatively modify `buildConflictLlmContext()` in TypeScript to:
1. Read both the worktree version and the main version of each conflicting file
2. Include BOTH in the message to the LLM
3. Ask the LLM to produce a merged version

This avoids modifying Rust code but gives the LLM less standard tooling to work with.

### Option C: Hybrid — write simple markers from TypeScript

The `mergeWorktreeHandler.ts` could read the main file + worktree file after getting the conflict error, construct simple conflict markers itself, write them into the worktree file, THEN inject the LLM context. This is hacky but avoids Rust changes.

---

## Affected Files

| File | Role |
|------|------|
| `codelet/git/src/session_result.rs` | `detect_conflicts()` returns names only, `apply_session_changes()` bails without markers |
| `codelet/git/src/error.rs` | `ConflictError` carries file names but no content |
| `src/tui/handlers/conflictLlmContext.ts` | Tells LLM to resolve markers that don't exist |
| `src/tui/handlers/mergeWorktreeHandler.ts` | Catches conflict error, delegates to LLM |

---

## Related Cards

- **GIT-036**: `/merge-worktree` slash command implementation
- **GIT-037**: Rich merge summary formatting
- **GIT-038**: Conflict details injected to LLM via `injectLlmContext`

---

## Evidence from Session Transcript

```
[22:58:37] SYSTEM → LLM: "Merge conflicts were detected in the following files:
  - README.md
  ...Please read each conflicting file, resolve the git conflict markers..."

[22:58:41] LLM reads /...worktrees/9f4fec89.../README.md — sees clean file, no markers

[22:58:47] LLM: "The file doesn't appear to have any conflict markers currently.
  Let me search more carefully:"
  → Grep for <<<<<<<|=======|>>>>>>> → "No matches found"

[22:58:52] LLM: "The file has no conflict markers — it looks like the conflict
  was already resolved by the edit I made earlier."
  → LLM is wrong — conflict was never materialized, not "already resolved"

[22:58:57] LLM attempts `git status` in worktree, tries to re-merge — stuck
```
