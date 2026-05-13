# RPC-009 AST research — view layer + Action enum + backend trait surface

## Scope

This research locks in the EXACT shapes that the RPC-009 test/impl phase
must respect, so the failing tests we write reference real types
correctly (no inventing types).

## Existing trait surface (RPC-008)

Source: `codelet/fspec-tui/src/transport/mod.rs`

```rust
#[async_trait]
pub trait FspecBackend: Send + Sync {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>>;
    async fn list_sessions(&self)   -> Result<Vec<SessionInfo>>;
    async fn create_session(&self, role: Option<String>) -> Result<SessionId>;
    async fn send_input(&self, id: SessionId, text: String) -> Result<()>;
    async fn interrupt(&self, id: SessionId) -> Result<()>;
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>;
    fn chunks_rx(&self)     -> broadcast::Receiver<(SessionId, StreamChunk)>;
    fn logs_rx(&self)       -> broadcast::Receiver<LogRecord>;
}
```

Both `EmbeddedFspecBackend` and `WebSocketFspecBackend` implement this in
`codelet/fspec-tui/src/transport/{embedded,websocket}.rs`. `MockBackend`
in `codelet/fspec-tui/tests/common/mod.rs` will be extended in RPC-009.

## Existing Action enum (RPC-008)

Source: `codelet/fspec-tui/src/components/mod.rs:77`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    Redraw,
    Custom(String),
}
```

RPC-009 must extend this by **adding** seven variants while preserving
the existing three: `LoadWorkUnits`, `WorkUnitsLoaded(Vec<WorkUnitInfo>)`,
`SessionCreated(SessionId)`, `ChunkReceived(SessionId, StreamChunk)`,
`InputSubmitted(String)`, `Interrupt`, `FocusNext`. The
`Clone, Debug, PartialEq, Eq` derives stay (verified `StreamChunk: Clone`
on line 338 of rpc-types).

## Existing Component trait + EventResult + Priority (RPC-008)

```rust
pub trait Component: Send {
    fn priority(&self) -> Priority { Priority::Medium }
    fn is_active(&self) -> bool    { true }
    fn id(&self) -> &str;
    fn handle_event(&mut self, _event: &Event) -> EventResult { EventResult::ignored() }
    fn update(&mut self, _action: Action) -> Option<Action>   { None }
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}
```

`EventResult::Ignored(Option<Callback>)` and
`EventResult::Consumed(Option<Callback>)` with helpers `::ignored()` and
`::consumed()`. Each new view (`WorkUnitsListView`, `AgentReplView`,
`RootView`, `FooterView`) implements this trait.

## App shell (RPC-008) — what RPC-009 mutates

Source: `codelet/fspec-tui/src/app.rs`

- `App::new(backend: Arc<dyn FspecBackend>)`: pre-populates compositor
  with `HelloComponent` at Background. **RPC-009: replace HelloComponent
  with RootView at Background.**
- `App::run(self) -> Result<()>`: tokio::select! over EventStream +
  action_rx + render-tick. **RPC-009: add the explicit bootstrap
  sequence (list_work_units → seed; create_session → seed; spawn three
  subscriber tasks) BEFORE entering the select loop. Add special-case
  dispatch for InputSubmitted/Interrupt/FocusNext BEFORE
  compositor.update inside the action_rx arm.**
- `App::handle_event` already special-cases `?`/`q`. **RPC-009: add Tab
  → emits Action::FocusNext (mirrors the `?` pattern).**

## Type shapes consumed by views (codelet/rpc-types/src/lib.rs)

```rust
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
}

pub struct SessionId { pub value: String }
impl SessionId { pub fn new(v: impl Into<String>) -> Self { ... } }
impl From<&str> for SessionId { ... }
impl From<String> for SessionId { ... }

pub enum StreamChunk {
    Text { text: String, correlation_id: Option<String>,
           observed_correlation_ids: Option<Vec<String>> },
    Thinking { thinking: String, correlation_id: ..., observed_correlation_ids: ... },
    ToolCall { tool_call: ToolCallInfo, ... },
    ToolResult { tool_result: ToolResultInfo, ... },
    ToolProgress { tool_progress: ToolProgressInfo, ... },
    SessionStateChange { state: SessionState },
    UserNotification { message: String, severity: NotificationSeverity },
    Interrupted { queued_inputs: Vec<String> },
    TokenUpdate { tokens: TokenTracker },
    ContextFillUpdate { context_fill: ContextFillInfo },
    Done,
    Error { error: String },
    // ... 23 variants total
}

// Helper constructors:
impl StreamChunk {
    pub fn text(text: String) -> Self { ... }
    pub fn thinking(thinking: String) -> Self { ... }
    pub fn tool_call(info: ToolCallInfo) -> Self { ... }
    // ...
}
```

**CRITICAL:** `StreamChunk::Text` is a struct variant — NOT a tuple
variant. Tests must construct via either the helper
`StreamChunk::text("hi".into())` or the explicit struct form
`StreamChunk::Text { text: ..., correlation_id: None,
observed_correlation_ids: None }`. Feature files have been corrected to
use the helper form.

## tui-input crate (the only new dependency)

Crate: `tui-input = "0.10"` (https://crates.io/crates/tui-input)

Public surface RPC-009 will use:

```rust
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

let mut input: Input = Input::default();
input.handle_event(&crossterm_event);  // forward CrosstermEvent
input.value();              // &str — current buffer
input.visual_cursor();      // usize — cursor x in *visual* cells
input.reset();              // clear the buffer
```

The crate provides a `Backend::Crossterm` event-handler bridge
(`EventHandler::handle_event(&mut self, &crossterm::event::Event)`) so we
DO NOT need to translate KeyCode/Modifiers ourselves — we just forward
the crossterm Event when the AgentReplView is focused (after we've
already inspected for Enter / Ctrl+C).

## Existing fixtures (RPC-008 Q-FIX-1)

Source: `codelet/fspec-tui/tests/common/mod.rs`

- `temp_service() -> (TempDir, Arc<SharedFspecService>)` — real
  WorkUnitsWatcher over a tempdir.
- `start_ws_server(...) -> (SocketAddr, JoinHandle<()>)` — real
  bind_and_serve on 127.0.0.1:0.
- `ws_url(addr) -> url::Url`.
- `workspace_root() -> PathBuf`.
- `SEED_WORK_UNITS_JSON` constant — two seeded work units (AUTH-001 done,
  AUTH-002 implementing).

**RPC-009 will extend:** add `MockBackend` impl of `FspecBackend` (with
scripted `create_session`/`send_input`/`interrupt` + per-call counters +
chunks_tx broadcast publisher), plus a `mock_with_two_seeded_units()`
fixture helper.

## Snapshot harness (RPC-008 Q-SNAP-1)

`insta::assert_yaml_snapshot!` over a `Vec<String>` of cell rows
extracted from a `ratatui::backend::TestBackend` buffer. RPC-009 adds
three new snapshot files alongside the existing `help_dialog__*` ones:

- `repl_bootstrap__80x24.snap`
- `repl_after_first_chunk__80x24.snap`
- `repl_after_submit__80x24.snap`

…plus an updated `help_dialog__centered_popup_80x24.snap` (the body text
changes to one-line-per-key per RPC-009 rule [14]).

## Source-shape regression tests (RPC-008)

`codelet/fspec-tui/tests/source_shape_trait.rs` and
`codelet/fspec-tui/tests/source_shape_cargo.rs` scan the source tree to
assert: (a) Q9 — no `Runtime::new()` / `tokio::runtime::Builder`; (b) no
direct imports of `codelet_napi`, `codelet_core`, `tarpc`,
`tokio_tungstenite` from view files; (c) `[dependencies]` excludes
`codelet-napi` and `codelet-core` and INCLUDES `tui-input`.

**RPC-009 widens these scans** to cover `src/views/{work_units_list,
agent_repl,root,footer}.rs`.

## New file inventory (production code)

```
codelet/fspec-tui/src/views/mod.rs
codelet/fspec-tui/src/views/work_units_list.rs
codelet/fspec-tui/src/views/agent_repl.rs
codelet/fspec-tui/src/views/root.rs
codelet/fspec-tui/src/views/footer.rs
```

Each ≤ 300 LoC, each implements `Component`, none directly imports
`codelet_napi` / `codelet_core` / `tarpc` / `tokio_tungstenite`.

## New integration tests

```
codelet/fspec-tui/tests/app_with_mock_backend_repl.rs
codelet/fspec-tui/tests/embedded_app_smoke.rs
codelet/fspec-tui/tests/ws_app_smoke.rs
```

`source_shape_*.rs` widen scans (no new file, just updated globs).

## Cross-references

- Plan: `spec/attachments/RPC-009/plan.md`
- Predecessor work unit: RPC-008 (FspecBackend + App shell)
- Parent: RPC-002 (master plan)
- Successor: RPC-010 (binary entry points)
