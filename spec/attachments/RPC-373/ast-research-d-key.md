# AST Research — RPC-373 Wire D key → FOUNDATION.md

## Target surfaces in codelet/fspec-tui (confirmed by reading/ast-grep)

- `src/views/board.rs:104` `BoardView::handle_event(&self, event, store) -> EventResult`.
  Existing key arms (lines 131–185) include:
  - `KeyCode::Char('f') | KeyCode::Char('F') => self.emit(Action::OpenChangedFilesView)` (174–178)
  - `KeyCode::Char('c') | KeyCode::Char('C') => self.emit(Action::OpenCheckpointsView)` (179–183)
  → add `Char('d') | Char('D') => self.emit(Action::OpenFoundation)` in the same match.
  `self.emit` (95–99) sends on the action bus; consumed via `EventResult::consumed()`.

- `src/components/mod.rs:108` `pub enum Action` — `OpenChangedFilesView` (967),
  `OpenCheckpointsView` (979). Add `OpenFoundation` here.

- Dispatch helper pattern: `src/app/dispatch_changed_files.rs` exposes
  `try_dispatch_changed_files(&mut self, action) -> bool` called from the
  catch-all of `App::dispatch`. Mirror as `dispatch_viewer.rs` /
  `try_dispatch_viewer`.

- `src/app/bootstrap.rs:24` `App::bootstrap` already performs best-effort,
  non-fatal async setup (`backend.checkpoint_counts()`, `get_workspace_info()`
  with `debug!` on error). The viewer start fits this exact pattern:
  `match codelet_attachment_viewer::start_viewer(cwd).await { Ok(h)=>store, Err(e)=>debug!() }`.

- `src/app/state.rs:33` `pub struct App` holds `backend`, `action_tx`,
  `pending_tasks`, etc. Add `viewer_port: Option<u16>` (+ optional
  `viewer_handle: Option<ViewerHandle>` for shutdown).

## Browser launcher

- `codelet/providers/src/claude_oauth_server.rs:119` and
  `codex/codex_oauth_server.rs:118` already call `open::that(&url)`; `open = "5"`
  in `codelet/providers/Cargo.toml:56`. Promote `open` to a workspace dep and
  use `open::that(url)` in the `Some(port)` branch only.

## Test harness reference

- `tests/view_board_unit_rpc012.rs` shows the BoardView key-test pattern:
  `BoardView::new(Arc::new(Theme::default()), tx)`, build a `BoardStore`, feed
  `Event::Key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE))`, assert
  `EventResult::Consumed(None)` and `rx.try_recv()` yields the expected `Action`.
  `wu(id, status)` helper constructs `WorkUnitInfo` (attachments: Vec::new()).

## Testability seam (no real browser in tests)

Split the dispatch into a pure `App::foundation_target() -> Option<String>` =
`self.viewer_port.map(foundation_url)` where
`foundation_url(port) = format!("http://127.0.0.1:{port}/view/spec/FOUNDATION.md")`.
Unit-test the pure functions; `open::that` is invoked only inside the `Some`
branch of the dispatch handler and is not exercised by tests.
