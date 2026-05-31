# RPC-062 — AST research: MCP injection plumbing in `codelet-sessions::SessionManager`

**Card:** RPC-062 · **Phase:** 7.9 of RPC-030 · **Estimate:** 3 pts · **Depends on:** RPC-061

This research locks in the exact call sites RPC-062 must pin via lifecycle + source-shape tests. All findings were produced by `ast-grep` / `grep` against the working tree at the time of specifying.

## 1. `McpInjection` type origin (NAPI-free, in `codelet-tools`)

```
codelet/tools/src/lib.rs:169-176          pub use mcp::{ ... McpInjection, ... };
codelet/tools/src/mcp.rs:158              pub enum McpInjection { Notification(String), SamplingRequest { ... } }
```

Conclusion: `McpInjection` lives in the NAPI-free `codelet-tools` crate and is re-exported at the crate root.

## 2. Import inside `codelet-sessions`

```
codelet/sessions/src/session_manager.rs:58    use codelet_tools::McpInjection;
```

One import, no local redefinition.

## 3. `init_mcp_session` call sites — produced by `ast-grep --lang rust 'codelet_tools::init_mcp_session($X)'`

```
codelet/sessions/src/session_manager.rs:600:52   codelet_tools::init_mcp_session(uuid)   # create_session_with_id body
codelet/sessions/src/session_manager.rs:846:52   codelet_tools::init_mcp_session(uuid)   # create_isolated_session_with_id body
```

Both sites destructure `(mcp_injection_rx, _mcp_connections)` and forward `mcp_injection_rx` to:

```
codelet/sessions/src/session_manager.rs:604   self.hooks().spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);
codelet/sessions/src/session_manager.rs:848   self.hooks().spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);
```

## 4. `cleanup_mcp_session` call site — produced by `ast-grep --lang rust 'codelet_tools::cleanup_mcp_session($X)'`

```
codelet/sessions/src/session_manager.rs:935:13   codelet_tools::cleanup_mcp_session(uuid)   # destroy_session body
```

Exactly one call, inside `pub fn destroy_session`, after the session is removed from the sessions map.

## 5. `SessionManagerHooks::spawn_agent_loop` trait signature

```
codelet/sessions/src/session_manager.rs:83-88
    fn spawn_agent_loop(
        &self,
        session: Arc<BackgroundSession>,
        input_rx: mpsc::Receiver<PromptInput>,
        mcp_injection_rx: mpsc::Receiver<McpInjection>,
    );
```

The third parameter is `mcp_injection_rx: mpsc::Receiver<McpInjection>`. The default `NoopSessionManagerHooks` impl (line 117) and the production `NapiSessionManagerHooks::spawn_agent_loop` in `codelet/napi/src/session_hooks.rs:22-30` both honour this signature; the latter forwards to `crate::agent_loop::agent_loop(session, input_rx, mcp_injection_rx).await`.

## 6. Consumer side in `codelet-napi`

```
codelet/napi/src/agent_loop.rs:307       mut mcp_injection_rx: mpsc::Receiver<McpInjection>,
codelet/napi/src/agent_loop.rs:424       result = mcp_injection_rx.recv(), if mcp_channel_open => { ... }
```

The receiver is consumed inside a `tokio::select!` arm guarded by `mcp_channel_open`. RPC-062 only audits this from the source-shape side (the NAPI binding lives outside `codelet-sessions`).

## 7. Global per-session MCP registry (process-global state)

```
codelet/tools/src/mcp.rs:820-822   static MCP_SESSIONS: ... Lazy<Mutex<HashMap<uuid::Uuid, McpSessionState>>>
codelet/tools/src/mcp.rs:826-839   pub fn init_mcp_session(session_id) -> (Receiver<McpInjection>, McpConnectionMap)
codelet/tools/src/mcp.rs:848-866   pub fn cleanup_mcp_session(session_id)
codelet/tools/src/mcp.rs:869-872   pub fn get_mcp_connections(session_id) -> Option<McpConnectionMap>
```

`get_mcp_connections` is the public observation hook RPC-062's lifecycle test uses to verify init→Some, cleanup→None, idempotent re-init, and unknown-uuid no-op without needing real provider credentials.

## 8. Negative grep — no MCP method has leaked into the RPC surface

The following files were scanned (post-RPC-061) and contain **zero** occurrences of `init_mcp` / `cleanup_mcp` / `mcp_session` / `mcp_injection`:

- `codelet/core/src/session_manager_handle.rs`
- `codelet/rpc/src/lib.rs`
- `codelet/fspec-tui/src/transport/mod.rs`

This confirms the architecture rule from the RPC-030 attachment: *"No new RPC surface needed — MCP injection is purely internal to the agent loop."*

## 9. Dependency invariant — `codelet-sessions` has no NAPI dependency

Already enforced by `codelet/sessions/tests/no_napi_dependency.rs` (RPC-044). RPC-062 must not regress that test.

## 10. Conclusion — RPC-062 scope

The wiring is **already** in place from RPC-039 (move BackgroundSession) and RPC-040 (move SessionManager). RPC-062 lands two new test files in `codelet/sessions/tests/`:

1. `mcp_injection_lifecycle.rs` — runtime tests against `codelet_tools::{init_mcp_session, cleanup_mcp_session, get_mcp_connections}` covering the five lifecycle scenarios in `spec/features/rpc-062-mcp-injection-lifecycle.feature`.
2. `mcp_injection_source_shape.rs` — substring/AST-grep scans pinning the four touchpoints above, covering the seven source-shape scenarios in `spec/features/rpc-062-mcp-injection-source-shape.feature`.

Zero production-code edits are expected. If a source-shape scan reveals a missing call site, the repair is minimal (re-add the missing call) and stays in scope for RPC-062.
