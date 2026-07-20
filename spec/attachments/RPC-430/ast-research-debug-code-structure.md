# AST Research: RPC-430 /debug Command Parity Gaps

## Date: 2026-07-20
## Purpose: Document existing code structure for /debug command implementation

---

## 1. App State (`codelet/fspec-tui/src/app/state.rs`)

### Struct Fields (Lines 29-82)
```rust
pub struct App {
    pub(crate) compositor: Compositor,
    pub(crate) action_tx: UnboundedSender<Action>,
    pub(crate) action_rx: UnboundedReceiver<Action>,
    pub(crate) backend: Arc<dyn FspecBackend>,
    pub(crate) theme: Arc<Theme>,
    pub(crate) navigator: Navigator,
    pub(crate) board_store: BoardStore,
    pub(crate) agent_view_store: AgentViewStore,
    pub(crate) should_quit: bool,
    pub(crate) should_render: bool,
    pub(crate) subscriber_tasks: Vec<JoinHandle<()>>,
    pub(crate) pending_tasks: Vec<JoinHandle<()>>,
    pub(crate) active_session_tx: watch::Sender<Option<SessionId>>,
    pub(crate) active_session_rx: watch::Receiver<Option<SessionId>>,
    pub(crate) pending_input_save_handle: Option<JoinHandle<()>>,
    pub(crate) search_history_debounce_handle: Option<tokio::task::AbortHandle>,
    pub(crate) applied_default_thinking: std::collections::HashSet<SessionId>,
    pub(crate) viewer_port: Option<u16>,
    pub(crate) viewer_handle: Option<codelet_attachment_viewer::ViewerHandle>,
    pub(crate) clipboard: Osc52Clipboard<Box<dyn std::io::Write + Send>>,
    pub(crate) reconnect_notice: Option<(SessionId, u64)>,
    pub(crate) reconnect_dismiss_handle: Option<JoinHandle<()>>,
    pub(crate) compaction_hide_handles: std::collections::HashMap<SessionId, JoinHandle<()>>,
    // ❌ MISSING: pre_session_debug_enabled: bool
}
```

### Constructor (Lines 86-129)
- `App::new(backend)` creates action bus, calls `with_action_bus`
- `App::with_action_bus(backend, action_tx, action_rx)` initializes all fields to defaults
- **No `pre_session_debug_enabled` field exists** — must be added

---

## 2. Slash Debug Handler (`codelet/fspec-tui/src/app/dispatch_slash_debug.rs`)

### handle_slash_debug (Lines 36-58)
```rust
pub(crate) fn handle_slash_debug(&mut self) {
    // ❌ GAP: Silent no-op when no session
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        return;
    };
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    // ❌ GAP: Wrong debug directory - uses project-relative path
    let debug_dir = std::env::var("FSPEC_DEBUG_DIR")
        .unwrap_or_else(|_| ".fspec/debug".to_string());
    let backend = self.backend.clone();
    let action_tx = self.action_tx.clone();
    let session_for_send = session_id;
    let handle = tokio::spawn(async move {
        let text = match backend.toggle_debug(session_for_send.clone(), debug_dir).await {
            Ok(path) => format!("[debug] capture toggled → {path}"),
            Err(e) => format!("[error] /debug failed: {e}"),
        };
        let _ = action_tx.send(Action::EmitSessionNotice(session_for_send, text));
    });
    self.pending_tasks.push(handle);
}
```

### try_dispatch_slash_debug (Lines 69-71)
```rust
#[allow(dead_code)]
pub(crate) fn try_dispatch_slash_debug(&mut self, _action: &Action) -> bool {
    false
}
```

---

## 3. Session Chrome Refresh (`codelet/fspec-tui/src/app/dispatch_session_chrome.rs`)

### refresh_session_chrome (Lines 24-58)
```rust
pub(crate) fn refresh_session_chrome(&mut self, session: SessionId) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    // Spawns get_model_info → Action::ModelInfoLoaded
    let h1 = tokio::spawn(async move {
        if let Ok(info) = backend.get_model_info(s1.clone()).await {
            let _ = action_tx.send(Action::ModelInfoLoaded(s1, info));
        }
    });
    self.pending_tasks.push(h1);

    // TUI-093: apply_default_thinking_level_if_needed → spawns set_thinking_level + get_thinking_level
    if !self.apply_default_thinking_level_if_needed(&session) {
        // Spawns get_thinking_level → Action::ThinkingLevelLoaded
    }

    // RPC-022: spawns get_session_role → Action::SessionRoleLoaded
    self.spawn_get_session_role(session);
    // ❌ MISSING: No debug state hydration (get_debug_enabled)
}
```

**Called from:**
- `dispatch_create_session_dialog.rs:157` — after SessionCreated
- `dispatch_resume_search_views.rs:130` — in handle_attach_to_session

---

## 4. Session Creation Dialog (`codelet/fspec-tui/src/app/dispatch_create_session_dialog.rs`)

### SessionCreated dispatch path
After `create_session` returns a SessionId:
1. `Action::SessionCreated(session_id)` is dispatched
2. `handle_session_created` (line ~140) appends SessionContext to agent_view_store
3. Calls `self.refresh_session_chrome(session.clone())`
4. Publishes to `active_session_tx`
5. **❌ MISSING: No check of pre_session_debug_enabled to propagate debug state**

---

## 5. AgentViewStore Debug State (`codelet/fspec-tui/src/store/agent_view/isolation_state.rs`)

### debug_enabled_by_session (Lines 86-97)
```rust
pub fn debug_enabled_for(&self, session: &SessionId) -> Option<bool> {
    self.debug_enabled_by_session.get(session).copied()
}

pub fn set_debug_enabled(&mut self, session: SessionId, enabled: bool) {
    self.debug_enabled_by_session.insert(session, enabled);
}
```

---

## 6. FspecBackend Debug Methods (`codelet/fspec-tui/src/transport/mod.rs`)

```rust
// Already exist with default implementations:
async fn get_debug_enabled(&self, _session_id: SessionId) -> Result<bool> { Ok(false) }
async fn set_debug_enabled(&self, _session_id: SessionId, _enabled: bool) -> Result<()> { Ok(()) }
async fn toggle_debug(&self, _session_id: SessionId, _debug_dir: String) -> Result<String> { Ok(String::new()) }
async fn set_debug_directory(&self, _path: String) -> Result<()> { Ok(()) }
```

Both `EmbeddedFspecBackend` and `WebSocketFspecBackend` override these with real delegates.

---

## Summary of Required Changes

1. **state.rs**: Add `pre_session_debug_enabled: bool` to `App` struct + initialize in constructor
2. **dispatch_slash_debug.rs**: 
   - Fix debug directory to resolve `~/.fspec` via `std::env::var("HOME")`
   - Add pre-session toggle path when no active session exists
3. **dispatch_session_chrome.rs**: Add debug hydration spawn for `get_debug_enabled`
4. **dispatch_create_session_dialog.rs**: After session creation, propagate `pre_session_debug_enabled` if true
