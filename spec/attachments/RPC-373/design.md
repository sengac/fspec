# RPC-373 — Wire D key on board to open FOUNDATION.md in browser

**Parent:** RPC-371 · **Depends on:** RPC-372 (viewer server) · **Blocks:** none

## Goal

Wire the `D`/`d` key in the Rust board view to open `spec/FOUNDATION.md` in the
user's default browser, rendered through the RPC-372 attachment viewer server.
This is the Rust port of `BoardView.tsx:314-325`.

## Current state

- `codelet/fspec-tui/src/views/board.rs` → `BoardView::handle_event` has arms for
  `f/F` (`OpenChangedFilesView`) and `c/C` (`OpenCheckpointsView`) but **no `d/D`**.
- `src/views/board/keybinding_shortcuts.rs` already renders the `D FOUNDATION.md`
  hint — it just isn't wired.
- The `Action` enum (`src/components/mod.rs`) has `OpenChangedFilesView` /
  `OpenCheckpointsView` but no `OpenFoundation`.
- App dispatch routes view-open actions via `try_dispatch_*` helpers (see
  `src/app/dispatch_changed_files.rs`).

## Architecture / lifecycle

The viewer server (RPC-372) must be **running** for the URL to work. Decide where
to own it:

- **Recommended:** start the viewer once during `App` bootstrap
  (`src/app/bootstrap.rs`) and store `Option<ViewerHandle>` on `App`
  (`src/app/state.rs`), stopping it on shutdown. The board's `D`/`A` handlers then
  read the port from `App`. This mirrors the TS `useEffect` that starts the server
  when BoardView mounts.
- The board's `handle_event` only **emits an Action** (it holds no async/runtime).
  So `D` emits `Action::OpenFoundation`; `App::dispatch` reads the viewer port,
  builds the URL, and launches the browser.

### cwd

The viewer is bound to the project cwd; `spec/FOUNDATION.md` is resolved relative
to it. Determine cwd from the same source the rest of the TUI uses (likely
`std::env::current_dir()` at bootstrap, or a value the backend already knows).
Document the chosen source in the architecture note.

## Changes

1. **Action**: add `Action::OpenFoundation` to `src/components/mod.rs`.
2. **Board key arm**: in `board.rs::handle_event`, add
   ```rust
   KeyCode::Char('d') | KeyCode::Char('D') => {
       self.emit(Action::OpenFoundation);
       return EventResult::consumed();
   }
   ```
   (place beside the existing `c`/`f` arms).
3. **Dispatch**: handle `Action::OpenFoundation` in `App::dispatch` (new
   `dispatch_viewer.rs` helper + `try_dispatch_viewer`, factored like
   `dispatch_changed_files.rs` to keep files < 300 lines):
   - If a viewer port is available, build
     `http://127.0.0.1:{port}/view/spec/FOUNDATION.md` and launch via
     `open::that(url)` on a spawned blocking/async task (don't block the dispatch
     loop; log on error, never panic).
   - If no port (server failed to start), no-op + `tracing::warn!` (parity with TS
     `if (attachmentServerPort)` guard).

## Browser launch

Use the `open` crate (already a dependency of `codelet/providers`; promote to a
workspace dep). In tests, the browser MUST NOT actually launch — gate behind a
test-environment check (mirror TS `isTestEnvironment`) OR structure the code so
the URL-building is unit-testable without invoking `open` (preferred: a pure
`foundation_url(port) -> String` function tested directly; the `open::that` call
sits in a thin wrapper that tests don't exercise).

## Scenarios (acceptance criteria)

1. **Pressing D on the board emits the OpenFoundation action** — given the board
   view is focused, when the `D` key is handled, then `Action::OpenFoundation` is
   emitted and the event is consumed.
2. **Lowercase d behaves identically** — pressing `d` also emits
   `Action::OpenFoundation` and consumes the event.
3. **OpenFoundation builds the FOUNDATION.md viewer URL from the server port** —
   given a viewer port `P`, the launched URL is exactly
   `http://127.0.0.1:P/view/spec/FOUNDATION.md`.
4. **OpenFoundation is a safe no-op when the viewer is unavailable** — given no
   viewer port, dispatching `OpenFoundation` does nothing (no panic, no URL) and
   logs a warning.

## Testing

- Board-level: construct a `BoardView` with a captured `action_tx`, feed a
  `KeyCode::Char('D')` (and `'d'`) `Event::Key`, assert the emitted `Action` and
  `EventResult::consumed()`. (Follow existing board key tests, e.g. the `f`/`c`
  arm tests.)
- URL building: unit-test `foundation_url(port)` returns the exact string.
- No-op: dispatch with `None` port and assert no browser wrapper invocation /
  no emitted follow-up (use an injectable launcher or the pure-URL split so no
  real browser opens).
- Every Gherkin step → `// @step …` comment.

## Definition of done

- 4 scenarios green; tests-first (red → green).
- `cargo build` + `cargo clippy` clean; files < 300 lines.
- The `D FOUNDATION.md` hint is now backed by real behaviour.
- Coverage links recorded for every scenario.
