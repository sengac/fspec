# AST Research: Diff System Architecture (GIT-040)

## Worker Thread Usage (2 call sites)
- `src/tui/components/CheckpointViewer.tsx:236` — `new Worker(workerPath)`
- `src/tui/components/FileDiffViewer.tsx:63` — `new Worker(workerPath)`

## Rust diff functions (codelet-git/src/diff.rs)
- `get_file_diff(dir, filepath) -> Result<Option<String>>` — working dir vs HEAD
- `is_binary_file(dir, filepath) -> Result<bool>` — binary detection

## NAPI bindings (codelet/napi/src/git.rs)
- `get_file_diff(dir, filepath) -> Option<String>` — already exposed

## Git CLI usage in production code (needs migration)
- `src/git/diff.ts:47` — `execSync('git show "${checkpointRef}:${filepath}"')`
- `src/git/diff.ts:60` — `execSync('git show HEAD:"${filepath}"')`

## Files to delete
- `src/git/diff-worker.ts` — the worker thread
- `src/git/worker-path.ts` — path resolution for the worker
- `src/tui/components/__tests__/worker-path-resolution.test.tsx` — tests for worker path

## TypeScript imports to update
- `FileDiffViewer.tsx` imports: `Worker` from worker_threads, `getWorkerPath`
- `CheckpointViewer.tsx` imports: `Worker` from worker_threads, `getWorkerPath`
- `diff.ts` imports: `execSync` from child_process, `diffLines` from diff

## Existing Rust infrastructure
- `codelet-git/src/ghost_commit.rs` — has `get_checkpoint_diff_files()` (file list, not content)
- `codelet-git/src/tree_utils.rs` — tree file reading helpers
- `similar` crate (v2) — already used for unified diff generation in Rust
- `gix` (v0.72) with `blob-diff` feature enabled
