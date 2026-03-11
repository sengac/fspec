# GIT-039: isomorphic-git Removal Findings

**Date:** 2026-03-11
**Parent:** GIT-016 (Git operations migration to Rust)
**Related:** GIT-013 (Replace isomorphic-git with gitoxide)

## Summary

The migration from isomorphic-git to gitoxide NAPI-RS bindings (GIT-013) was **partially completed**. isomorphic-git remains as a production dependency and is still actively imported in source files and test files.

---

## What WAS Successfully Migrated

These files now use `@sengac/codelet-napi` (gitoxide/Rust):

| File | NAPI Functions Used |
|------|-------------------|
| `src/git/status.ts` | `getStagedFiles`, `getUnstagedFiles`, `getUntrackedFiles`, `getCurrentBranch` |
| `src/git/diff.ts` | `getFileDiff` |
| `src/utils/git-checkpoint.ts` | `createGhostCheckpoint`, `restoreGhostCheckpoint`, `listGhostCheckpoints`, `deleteGhostCheckpoint`, `getCheckpointDiffFiles` |

---

## Production Source Files Still Using isomorphic-git

### 1. `src/tui/components/CheckpointViewer.tsx` (line 18)

```typescript
import * as git from 'isomorphic-git';
```

**Usage — `git.resolveRef()` at 3 call sites:**
- **Line 118:** `await git.resolveRef({ fs, dir: cwd, ref })` — resolving checkpoint ref to OID during checkpoint loading
- **Line 391:** `await git.resolveRef({ fs, dir: cwd, ref: checkpoint.stashRef })` — resolving ref before single-file restore
- **Line 431:** `await git.resolveRef({ fs, dir: cwd, ref: checkpoint.stashRef })` — resolving ref before full checkpoint restore

**What's needed:** A `resolveRef(dir, refName) -> string` NAPI binding.

### 2. `src/tui/store/fspecStore.ts` (line 22)

```typescript
import git from 'isomorphic-git';
```

**Usage — `git.log()` at line 210:**
```typescript
const logs = await git.log({
  fs,
  dir: cwd,
  ref: 'refs/stash',
  depth: 10,
});
```

Used in `loadStashes()` to list git stashes for display in the TUI.

**What's needed:** A `gitLog(dir, ref, depth) -> CommitInfo[]` NAPI binding, or since the checkpoint system now uses ghost commits instead of stashes, this entire stash-loading mechanism may be obsolete and should be removed.

### 3. `src/utils/projectManagementSections/gitCheckpoints.ts` (line 27)

Documentation string still says:
> "fspec provides an intelligent checkpoint system that uses **isomorphic-git's `git.stash({ op: 'create' })`**"

This is now factually wrong — the checkpoint system uses Rust ghost commits via NAPI. Must update documentation.

### 4. `src/commands/review.ts` (line 155)

Architecture review hint string:
```typescript
'  - Architectural patterns (e.g., use isomorphic-git not child_process)'
```

Should reference gitoxide/NAPI-RS bindings instead.

---

## Test Files Still Using isomorphic-git (19 files)

### Test Helper

| File | isomorphic-git Usage |
|------|---------------------|
| `src/test-helpers/universal-test-setup.ts` | `git.init()`, `git.setConfig()`, `git.add()`, `git.commit()` — used by many tests to set up git repos |

### Test Files That Import isomorphic-git

| File | Usage |
|------|-------|
| `src/tui/components/__tests__/CheckpointViewer-restore.test.tsx` | import + vi.mock |
| `src/tui/components/__tests__/CheckpointViewer-delete.test.tsx` | import + vi.mock |
| `src/tui/__tests__/BoardView-git-context-work-unit-details.test.tsx` | import + vi.mock |
| `src/tui/__tests__/BoardView-git-watcher-fix.test.tsx` | vi.mock only |
| `src/tui/__tests__/BoardView-file-watchers.test.tsx` | import + vi.mock |
| `src/tui/__tests__/TUI-060-session-work-unit-ipc.test.ts` | import |
| `src/tui/handlers/__tests__/fixtures/mergeWorktreeFixture.ts` | import |
| `src/tui/services/__tests__/sessionService.e2e.test.ts` | import |
| `src/utils/__tests__/ipc-integration.test.ts` | import |
| `src/utils/__tests__/git-checkpoint-deleted-files.test.ts` | import |
| `src/utils/__tests__/git-checkpoint-restore-deletes-new-files.test.ts` | import |
| `src/commands/__tests__/checkpoint.test.ts` | import |
| `src/commands/__tests__/auto-checkpoint-on-status-transition.test.ts` | import |
| `src/commands/__tests__/report-bug-to-github.test.ts` | import for git init in test |
| `src/commands/__tests__/restore-checkpoint-terminology.test.ts` | import |
| `src/commands/__tests__/auto-checkpoint-cleanup.test.ts` | import |
| `src/git/__tests__/diff-binary-and-truncation.test.ts` | `git.add()`, `git.commit()` for test setup |
| `src/git/__tests__/status.test.ts` | Deprecated/skipped but file still exists |

---

## package.json Issues

1. **Dependency not removed:** `"isomorphic-git": "^1.34.0"` still in `dependencies` (line 96)
2. **Build script references it:** `--external:isomorphic-git` in the esbuild command (line 16)

---

## NAPI Bindings Gap Analysis

The Rust NAPI module (`codelet/napi/src/git.rs`) currently exposes:

| Function | Available |
|----------|-----------|
| `get_staged_files` | ✅ |
| `get_unstaged_files` | ✅ |
| `get_untracked_files` | ✅ |
| `get_file_diff` | ✅ |
| `get_current_branch` | ✅ |
| `create_ghost_checkpoint` | ✅ |
| `restore_ghost_checkpoint` | ✅ |
| `list_ghost_checkpoints` | ✅ |
| `delete_ghost_checkpoint` | ✅ |
| `get_checkpoint_diff_files` | ✅ |
| Worktree operations | ✅ |
| `resolve_ref` | ❌ MISSING — needed by CheckpointViewer.tsx |
| `git_log` | ❌ MISSING — needed by fspecStore.ts (may be obsolete) |
| `git_init` | ❌ MISSING — needed by test helpers |
| `git_add` | ❌ MISSING — needed by test helpers |
| `git_commit` | ❌ MISSING — needed by test helpers |
| `git_set_config` | ❌ MISSING — needed by test helpers |

**Note:** The `resolve_ref` function already exists in the Rust `codelet/git/src/ghost_commit.rs` as a private helper. It just needs to be exposed publicly and wired through NAPI.

---

## Spec/Feature Files Referencing isomorphic-git

~20+ feature files in `spec/features/` still reference isomorphic-git in architecture notes and descriptions. These are documentation/historical artifacts that should be updated to reflect the gitoxide architecture.

The `@isomorphic-git` tag is still registered in `spec/tags.json` and documented in `spec/TAGS.md`.

---

## Fix Plan

### Phase 1: Add Missing NAPI Bindings
1. Expose `resolve_ref` in `codelet/git` and wire through `codelet/napi/src/git.rs`
2. Add `git_init`, `git_add`, `git_commit`, `git_set_config` for test infrastructure
3. Evaluate whether `git_log` / stash loading is still needed (ghost commits replaced stashes)

### Phase 2: Update Production Source Files
1. **CheckpointViewer.tsx** — Replace `git.resolveRef()` calls with NAPI `resolveRef()`
2. **fspecStore.ts** — Remove `loadStashes()` or replace with NAPI equivalent
3. **gitCheckpoints.ts** — Update documentation strings
4. **review.ts** — Update architecture hint text

### Phase 3: Update Test Infrastructure
1. **universal-test-setup.ts** — Replace isomorphic-git with NAPI bindings for git init/add/commit
2. **All 18 test files** — Remove isomorphic-git imports/mocks, use NAPI bindings

### Phase 4: Remove isomorphic-git
1. Remove from `package.json` dependencies
2. Remove `--external:isomorphic-git` from build script
3. Run `npm install` to clean up
4. Verify build and all tests pass

### Phase 5: Update Documentation
1. Update feature files referencing isomorphic-git
2. Remove/deprecate `@isomorphic-git` tag
3. Update `spec/TAGS.md`
