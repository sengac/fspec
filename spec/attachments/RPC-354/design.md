# RPC-354 — File Changes view in the Rust TUI (port of `ChangedFilesViewer`)

## Problem

In the **TypeScript** TUI, the board ("BoardView") advertises a chord
`C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent`. Pressing **F**
opens a full-screen **Changed Files** view (`src/tui/components/ChangedFilesViewer.tsx`)
that shows a dual-pane layout: a list of changed git files (staged + unstaged, each with an
A/M/D/R status indicator) on the left, and the selected file's unified diff on the right.

In the **Rust** ratatui TUI (`codelet/fspec-tui`), the `F Changed Files` chord is **hint-only**.
`codelet/fspec-tui/src/views/board/keybinding_shortcuts.rs` paints the literal string but its
module header states:

> *"The C / F / D / / keybindings are hint-only in this card — wiring lands in subsequent RPC-002 children."*

Confirmed gaps in the Rust TUI:

- ❌ No `ViewMode::ChangedFiles` variant in `views/navigator.rs` (only `Board`, `Agent`,
  `ProviderSettings`, `Blocklist`, `ModelSelector`).
- ❌ No `Action` to open/close the view.
- ❌ No `KeyCode::Char('f')`/`Char('F')` arm in `BoardView::handle_event` (falls through to
  `_ => ignored()`).
- ❌ No view component (no dual-pane / diff-rendering code anywhere).
- ❌ Git changed-file status + per-file diff are **not** exposed through the TUI transport
  (`transport/mod.rs` exposes `checkpoint_counts()` but no git status/diff method).

The underlying git primitives **already exist** in `codelet/git` and must be reused, not
reimplemented.

## Existing reusable primitives (`codelet/git`)

| Function | Signature | Notes |
|---|---|---|
| `status::get_staged_files` | `(dir) -> Result<Vec<String>>` | paths only, no change type |
| `status::get_unstaged_files` | `(dir) -> Result<Vec<String>>` | paths only |
| `status::get_untracked_files` | `(dir) -> Result<Vec<String>>` | untracked = Added |
| `diff::get_file_diff` | `(dir, filepath) -> Result<Option<String>>` | unified diff; returns `"[Binary file - no diff available]"` for binary; `Err(FileNotFound)` for deleted files |

The git crate does **not** currently derive change type (A/M/D/R). The TS reference
(`src/git/status.ts::getChangeType`) shells out to `git diff --name-status`. In Rust we should
derive A/M/D from gitoxide state without shelling out:
- **A (Added)** — untracked file, or staged file absent from HEAD.
- **D (Deleted)** — tracked/indexed file missing from the working directory.
- **M (Modified)** — otherwise.
- **R (Renamed)** — best-effort; default to **M** (matches the TS fallback) unless cheaply detectable.

## TypeScript reference behaviour (`ChangedFilesViewer.tsx` + `FileDiffViewer.tsx`)

- On mount, lazily loads file status (`loadFileStatus()` → `stagedFiles` + `unstagedFiles`),
  each item carrying `filepath`, `changeType`, `staged`.
- Builds one combined list: staged first, then unstaged. Each `FileItem` has
  `{ path, status: 'staged'|'unstaged', changeType }`.
- Dual-pane via the shared `FileDiffViewer`:
  - **Left**: file list. Row = `> A path/to/file` where the cursor `>` marks the selection and the
    status letter is colored: **A=green, M=yellow, D=red, R=cyan** (default M=yellow).
  - **Right**: diff pane. Loads the selected file's diff and renders colored +/- lines.
- Keys: **Esc** back to board; **Tab / Left / Right** switch pane focus; **Up/Down** move the file
  selection (the diff pane re-loads for the newly selected file); **PgUp/PgDn** scroll.
- Footer: `ESC: Back | Tab: Switch Panes | ↑↓: Navigate | PgUp/PgDn: Scroll`.

## Solution — split into two children

### RPC-355 — Backend / transport (data foundation)
Expose changed-file status (with change type) and per-file diff through the TUI transport,
delegating to `codelet/git`. Implemented on both embedded and websocket transports. Gated on the
shared service's attached `cwd`, mirroring `checkpoint_counts()`.

### RPC-356 — UI (depends on RPC-355)
Build the dual-pane `ChangedFilesView`, wire the `F` key on the board, and integrate
`ViewMode::ChangedFiles` into the Navigator. Reuse the existing `scroll_viewport` / `WheelVelocity`
infrastructure (added in RPC-353) for scrolling.

## Integration pattern (mirror `checkpoint_counts`, RPC-015)

End-to-end path proven by `checkpoint_counts`:

1. `codelet/git` helper (exists) ←
2. `FspecService` method in `codelet/rpc/src/lib.rs` — gate on `self.inner.cwd()`, return default
   when no cwd (`lib.rs:896` is the template) ←
3. `FspecBackend` trait method in `transport/mod.rs` + one-line delegates in `embedded.rs`
   (`context::current()`) and `websocket.rs` (the `client.read().await` + `BackendError::Disconnected`
   guard) ←
4. `Action` variants + `App::dispatch` wiring ←
5. The view consumes the data.

## Out of scope (separate future cards)
- `C` Checkpoints viewer, `D` FOUNDATION.md viewer, `A` attachments viewer (also hint-only).
- Staging/unstaging/committing from the view (TS view is read-only too).
- Rename (R) detection beyond best-effort.

## Acceptance (umbrella)
Pressing **F** on the Rust board opens a working dual-pane Changed Files view with a colored
status file list and a live diff pane, Esc returns to the board, and the data comes from the real
git working tree via the transport — matching the TypeScript `ChangedFilesViewer`.
