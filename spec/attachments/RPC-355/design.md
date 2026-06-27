# RPC-355 — Expose git changed-file status and per-file diff to the TUI transport

Parent: **RPC-354**. This is the **data foundation** for the Rust File Changes view. No UI here.

## Goal

Add two capabilities to the TUI backend surface, delegating to the existing `codelet/git`
primitives (do **not** reimplement git logic):

1. **`changed_files()`** → the list of changed files (staged + unstaged + untracked), each with a
   relative `path`, a `change_type` (A/M/D/R), and a `staged` flag.
2. **`file_diff(path)`** → the unified diff text for one file (or `None` when there is no diff).

Both must be implemented on **both** transports (embedded + websocket) and gated on the shared
service's attached `cwd`, exactly like `checkpoint_counts()`.

## Reuse — `codelet/git`

| Need | Use |
|---|---|
| staged paths | `codelet_git::status::get_staged_files(cwd)` |
| unstaged paths | `codelet_git::status::get_unstaged_files(cwd)` |
| untracked paths (→ Added) | `codelet_git::status::get_untracked_files(cwd)` |
| per-file diff | `codelet_git::diff::get_file_diff(cwd, path)` |

### Change-type derivation
`get_staged_files`/`get_unstaged_files` return paths only. Derive A/M/D in `codelet/git` (new small
helper, gitoxide-based, **no shelling out**):
- **A** — path is untracked, OR staged but not present in the HEAD tree.
- **D** — path is tracked/indexed but missing from the working directory (`!full_path.exists()`).
- **M** — otherwise.
- **R** — best-effort; default to **M** if not cheaply detectable (matches the TS fallback in
  `src/git/status.ts::getChangeType`).

Prefer adding `get_staged_files_with_change_type(cwd)` and
`get_unstaged_files_with_change_type(cwd)` (returning a typed struct) in `codelet/git`, mirroring the
TS `getStagedFilesWithChangeType` / `getUnstagedFilesWithChangeType`. Untracked files are always `A`.

## New RPC types (`codelet/rpc-types`)

```rust
/// One changed file in the working tree (RPC-355).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedFile {
    /// Repo-relative path.
    pub path: String,
    /// Single-letter change type: "A" | "M" | "D" | "R".
    pub change_type: String,
    /// true = staged (index), false = unstaged/untracked working-tree change.
    pub staged: bool,
}
```

(Use `String` for `change_type` to stay serde/tarpc-friendly; the UI maps the letter to a color.)

## Wiring (mirror `checkpoint_counts`, RPC-015)

### 1. `FspecService` — `codelet/rpc/src/lib.rs`
Add to the service trait + impl. Template at `lib.rs:896`:

```rust
async fn changed_files(self, _ctx: Context) -> Vec<ChangedFile> {
    match self.inner.cwd() {
        Some(cwd) => collect_changed_files(cwd).unwrap_or_default(),
        None => Vec::new(),
    }
}

async fn file_diff(self, _ctx: Context, path: String) -> Option<String> {
    match self.inner.cwd() {
        Some(cwd) => codelet_git::diff::get_file_diff(cwd, &path).ok().flatten(),
        None => None,
    }
}
```

`collect_changed_files` builds the combined `Vec<ChangedFile>`: staged first (with change type),
then unstaged, then untracked (Added) — mirroring TS ordering.

### 2. `FspecBackend` trait — `codelet/fspec-tui/src/transport/mod.rs`
Add trait methods (with default impls returning empty/None so test doubles compile unchanged,
following the established RPC-037/PROV-109 convention):

```rust
async fn changed_files(&self) -> Result<Vec<ChangedFile>> { Ok(Vec::new()) }
async fn file_diff(&self, _path: String) -> Result<Option<String>> { Ok(None) }
```

### 3. `embedded.rs` — one-line delegates
```rust
async fn changed_files(&self) -> Result<Vec<ChangedFile>> {
    Ok(self.client.changed_files(context::current()).await?)
}
async fn file_diff(&self, path: String) -> Result<Option<String>> {
    Ok(self.client.file_diff(context::current(), path).await?)
}
```

### 4. `websocket.rs` — guarded delegates
Follow the existing `let client = self.client.read().await; let client = client.as_ref().ok_or(BackendError::Disconnected)?;`
pattern used by the other websocket methods.

## Rules to encode (Example Map)
- `changed_files()` returns staged + unstaged + untracked, staged entries first.
- Each entry carries a correct `change_type`: untracked → A; missing-from-workdir → D; otherwise M.
- `changed_files()` returns an empty Vec when the shared service has no cwd attached (no panic).
- `file_diff(path)` returns the unified diff for a modified file.
- `file_diff(path)` returns `None` when the file has no changes / no cwd.
- Binary files yield the sentinel `"[Binary file - no diff available]"` (from `get_file_diff`).
- Both transports (embedded + websocket) expose identical method semantics.

## Testing
- Unit-test `collect_changed_files` / change-type derivation against a real temp git repo (use the
  existing `codelet/git` test helpers / `codelet/test-helpers`). Integration-first: real fs + real
  gitoxide, no mocking of git.
- Test the embedded backend end-to-end against a temp repo (add/modify/delete files, assert the
  `ChangedFile` list and a diff round-trip).
- `cargo build` + `cargo test` for the touched crates (`codelet-git`, `codelet-rpc`,
  `codelet-rpc-types`, `codelet-fspec-tui`).

## Constraints
- No `unwrap()`/`expect()` in production paths — use `?` / `unwrap_or_default()`.
- Files under 300 lines (extract a `changed_files.rs` helper module if `lib.rs` grows).
- Every Gherkin step needs a matching `// @step` comment in the test.
