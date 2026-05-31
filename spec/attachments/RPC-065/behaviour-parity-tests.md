# RPC-065 — Behaviour-parity test suite for every slash command + keyboard shortcut

**Parent:** RPC-030 · **Phase:** 8.1 · **Estimate:** 8 pts · **Depends on:** RPC-064

## Goal

For every slash command and every keyboard shortcut, write an integration test in `codelet/fspec-tui/tests/` that drives the AgentView through a deterministic `StubSessionManagerHandle` backend and asserts the exact same store-state transitions the TS frontend would produce.

The TS frontend's test suite (`src/tui/__tests__/`) is the reference.

## Test matrix

### Slash commands

| Command | Test |
|---|---|
| `/help` | Opens HelpDialog |
| `/clear` | Resets scrollback + calls `backend.clear_history` (RPC-046 test) |
| `/quit` | Sets `should_quit = true` |
| `/model` | Opens ModelSelectorDialog |
| `/thinking` | Opens ThinkingLevelDialog |
| `/thinking off|low|med|high` | Direct set (RPC-048 test) |
| `/role` | Opens RoleDialog (RPC-063 test) |
| `/role <text>` | Direct set |
| `/role` (bare from palette) | Clear role |
| `/resume` | Opens ResumeSessionView |
| `/search` | Opens SearchHistoryView (RPC-064 test) |
| `/provider` | Opens ProviderSettingsView (RPC-054 test) |
| `/providers` | Same as `/provider` (alias) |
| `/debug` | Calls `backend.toggle_debug` (RPC-055 test) |
| `/compact` | Calls `backend.compact_session` (RPC-047 test) |
| `/isolation` | TBD — confirm scope |
| `/blocklist` | Opens BlocklistView (RPC-056 test) |
| `/detach` | Calls `backend.set_work_unit_context(None)` (RPC-050 test) |
| `/merge-worktree` | Opens MergeConfirmDialog (RPC-057 test) |
| `/schedule add/list/pause/resume/remove` | Parser + RPC calls (RPC-058 test) |
| `/loop <interval> <prompt>` / `cancel` / `list` | Parser + RPC calls (RPC-059 test) |

### Keyboard shortcuts

| Shortcut | Test |
|---|---|
| `Shift+←/→` | Cycles sessions (RPC-024 already tested; verify) |
| `Shift+↑/↓` | History recall (RPC-025; verify TS parity) |
| `Tab` | Turn-selection mode |
| `Ctrl+R` | Opens SearchHistoryView |
| `Esc` cascade (5 levels) | RPC-051 test |
| `Enter` (submit) | `backend.send_input` |
| `Ctrl+C` | `backend.interrupt` |
| `PageUp` / `PageDown` / `End` | Scrollback navigation |

## Test structure

Each test follows the pattern:

```rust
use codelet_core::StubSessionManagerHandle;
use codelet_fspec_tui::test_harness::*;

#[tokio::test]
async fn slash_clear_resets_scrollback_and_calls_backend() {
    let stub = Arc::new(StubSessionManagerHandle::new());
    let mut harness = AppTestHarness::new(stub.clone()).await;

    // Setup: create a session and seed scrollback.
    harness.create_session_and_focus().await;
    harness.dispatch_text_chunk("hello").await;
    assert_eq!(harness.scrollback_lines().len(), 1);

    // Action: trigger /clear.
    harness.execute_slash_command(SlashCommandAction::Clear);
    harness.flush().await;

    // Assertions.
    assert!(harness.scrollback_lines().is_empty());
    assert_eq!(stub.clear_history_calls(), 1);
}
```

## Test harness — `codelet/fspec-tui/src/test_harness.rs`

Create or extend:

```rust
pub struct AppTestHarness {
    pub app: App,
    pub backend: Arc<DyFspecBackend>, // wrapping the stub
    pub event_sender: mpsc::Sender<Event>,
    pub dispatch_sender: mpsc::Sender<Action>,
}

impl AppTestHarness {
    pub async fn new(stub: Arc<dyn SessionManagerHandle>) -> Self;
    pub async fn create_session_and_focus(&mut self);
    pub async fn dispatch_text_chunk(&mut self, text: &str);
    pub fn execute_slash_command(&mut self, action: SlashCommandAction);
    pub async fn key(&mut self, key: KeyEvent);
    pub async fn flush(&mut self); // drain pending tokio tasks
    pub fn scrollback_lines(&self) -> Vec<Line>;
    pub fn session_status(&self) -> SessionStatus;
    // ...
}
```

## Stub instrumentation

Extend `StubSessionManagerHandle` with call counters / recorders:

```rust
pub fn clear_history_calls(&self) -> usize;
pub fn last_send_input(&self) -> Option<(SessionId, String)>;
pub fn pause_confirm_calls(&self) -> Vec<(SessionId, bool)>;
// etc.
```

## Acceptance criteria

1. Every entry in the test matrix has a `#[tokio::test]` in `codelet/fspec-tui/tests/`.
2. `AppTestHarness` exists and is reusable across tests.
3. `StubSessionManagerHandle` exposes call-counters for every method.
4. `cargo test -p codelet-fspec-tui --tests` runs all parity tests in < 30s.
5. Every test references the equivalent TS test file in a comment (for tracing).

## Risks

- Test harness setup can leak tokio tasks → use `tokio::test(flavor = "current_thread")` to keep them deterministic.
- Stale results from debounced `set_pending_input`, `search_history` etc. — harness must `flush()` properly.

## Out of scope

- Real backend integration (covered by RPC-066).
- Visual regression (ratatui snapshot tests are a separate concern).
