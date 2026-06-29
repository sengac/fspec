# RPC-386 — AgentManager handler binds to the wrong SessionManager

**Type:** Bug
**Epic:** rust-frontend (Distributed Rust Frontend — tarpc + ratatui)
**Supersedes/corrects:** RPC-385 (the `session_created` broadcast fix that "did absolutely nothing")

---

## 1. Symptom (as reported)

> "I just tried spinning up the new code and it doesn't create agents with AgentManager."

In the pure-Rust `fspec` binary (combined/daemon/client modes), calling the
`AgentManager` tool with `action='spawn'` from a foreground agent appears to do
nothing: no subordinate agent runs, and no subordinate tab appears in the Rust
TUI. This is **worse** than RPC-385's original symptom (invisible-but-running):
here the subordinate neither runs nor is visible.

---

## 2. Root cause — two different SessionManager objects in one process

There are **two distinct `SessionManager` instances** reachable in the Rust
process, and the AgentManager handler writes to the wrong one.

### 2a. The daemon/TUI stack uses a *locally-owned, injected* instance

`codelet/fspec/src/common.rs::build_service` (the single chokepoint for
`combined::run` and `daemon::run`) constructs its own manager and installs the
real agent-loop hooks on it:

```rust
// codelet/fspec/src/common.rs  (~line 112)
let manager = SessionManager::new();                    // FRESH, independent instance
manager.set_hooks(Arc::new(FspecAgentHooks::new()));    // real hooks live HERE
let session_manager: Arc<dyn SessionManagerHandle> = Arc::new(manager);
SharedFspecService::with_session_manager(watcher, session_manager).with_cwd(...)
```

Everything the Rust TUI observes flows from **this** instance:
- foreground sessions (`create_session`)
- `chunks_rx` / `logs_rx` / `status_changes_rx`
- `session_created_rx()` — the RPC-385 tab-creation broadcast
  (`SessionManagerHandle::session_created_rx` forwards
  `session_created_tx().subscribe()` from the attached handle —
  `codelet/core/src/session_manager_handle.rs:620`,
  `codelet/rpc/src/lib.rs:873`,
  `codelet/fspec-tui/src/transport/embedded.rs:953`).

This instance has **`FspecAgentHooks`**, whose `spawn_agent_loop` actually
launches the NAPI-free agent loop.

### 2b. The AgentManager handler uses the *global singleton*

The AgentManager handler closure hardcodes the process-global singleton:

```rust
// codelet/agent-loop/src/agent_manager_handler.rs:43  (create_handler)
let session_manager = SessionManager::instance();
// ...also at:
//   line 909  (create_async_handler — await_idle / profile)
```

`SessionManager::instance()` is a separate `OnceLock<SessionManager>`:

```rust
// codelet/sessions/src/session_manager.rs:314
pub fn instance() -> &'static SessionManager {
    static INSTANCE: OnceLock<SessionManager> = OnceLock::new();
    INSTANCE.get_or_init(SessionManager::new)   // brand-new manager, NoopSessionManagerHooks
}
```

`new()` (line 181) and `instance()` (line 314) return **two different
managers**. In the `fspec` process, **nobody installs `FspecAgentHooks` on the
singleton** — `build_service` only set hooks on its local instance.

### 2c. Why "spawn does absolutely nothing"

When the foreground agent calls `AgentManager(action='spawn')`:

1. `AgentManagerTool::call` → `execute_agent_manager` → registered handler
   closure → `handle_spawn(session_manager = SessionManager::instance(), …)`.
2. `handle_spawn` calls `instance().create_session_with_id(...)` — the
   subordinate is created in the **singleton**, which:
   - still has the default **`NoopSessionManagerHooks`**, whose
     `spawn_agent_loop` is an empty no-op
     (`codelet/sessions/src/session_manager.rs:111-117`). The subordinate's
     agent loop **never starts** — it can never process a message. → *dead*.
   - is a **different manager than the one the TUI subscribes to**, so
     `list_sessions`, `chunks_rx`, and the `session_created_rx` tab path all read
     the *local* instance and never see it. → *invisible*.
3. `handle_spawn` returns `Spawned { session_id }` (technically true — a dead
   session object exists in the singleton), so no error surfaces.

---

## 3. Why the RPC-385 fix "did absolutely nothing"

RPC-385 added `session_created_tx.send(...)` in `create_session_with_id` /
`create_isolated_session_with_id` plus a `session_created_rx` subscriber in the
embedded backend. That broadcast fires on **whichever manager runs
`create_session_with_id`** — the **singleton** for spawns. The embedded TUI
transport subscribes to the **daemon-owned** `build_service` manager. Different
objects → the broadcast was sent into a channel nobody in the TUI listens to.
RPC-385 treated a downstream symptom and could never work while the upstream
binding was wrong.

**Corollary (important):** Fixing the binding (this card) makes the RPC-385
machinery finally function — spawning into the daemon-owned manager fires
`session_created_tx` on exactly the channel the embedded backend already
subscribes to, so the subordinate tab appears *and* the agent loop runs. Both
symptoms are fixed by one root-cause change.

---

## 4. Why the NAPI/TypeScript frontend is unaffected

In the NAPI path everything is unified on the singleton:
- `codelet/napi/src/session_hooks.rs:70` →
  `SessionManager::instance().set_hooks(Arc::new(NapiSessionManagerHooks))`
  installs real hooks on the **singleton**.
- NAPI session creation also goes through the singleton.

So in TS-land the handler's `SessionManager::instance()` *is* the same manager
that has working hooks and that the frontend observes. The Rust `fspec` binary
broke this invariant by introducing a locally-owned `SessionManager::new()` in
`build_service` (mandated by RPC-044) while the lifted AgentManager handler in
`codelet-agent-loop` still hard-codes `SessionManager::instance()`.

---

## 5. Architectural mandate (NAPI = boundary between daemon and client)

The early/foundational cards establish that the daemon **owns one injected
instance**; the global singleton is a NAPI-only legacy footgun:

- **RPC-040** (`move-session-manager.md`): *"`SessionManager::instance()` is a
  footgun… The Rust frontend (RPC-044) must NOT use it — RPC-044 constructs a
  fresh `Arc<SessionManager>`."*
- **RPC-044** (`wire-into-fspec-binary.md`): *"only the daemon should construct
  the `SessionManager`. The client connects via WS to the daemon's service."*
  Regression tests assert `build_service` builds `Arc::new(SessionManager::new())`
  and uses `SharedFspecService::with_session_manager(watcher, session_manager)`
  (never the bare `SharedFspecService::new(watcher)`).
- **RPC-002** (`rpc-002-feasibility.md` §6): in remote mode the manager lives in
  `fspec-daemon`; clients attach over WS; the single-listener chunk callback
  becomes a `broadcast::Sender<(SessionId, StreamChunk)>` with one subscriber per
  client.
- **RPC-072** (`architecture.md`): session lifecycle (`spawn_agent_loop`,
  AgentManager spawn) lives in the NAPI-free `codelet-agent-loop` crate, wired via
  `FspecAgentHooks`; `handle_spawn` already takes `session_manager: &SessionManager`
  by reference — confirming injection is the intended shape; only its *source*
  (`instance()`) is wrong.

**Dependency-arrow rule (must not regress):** `rpc → napi`, `fspec → napi`,
`fspec-tui → napi`, `sessions → napi`, `agent-loop → napi` are all FORBIDDEN. The
fix must not reach into `codelet-napi`.

Therefore the only architecturally-correct fix is **dependency injection**: hand
the AgentManager sync + async handlers the *same* `SessionManager` that created
the spawner session — never `SessionManager::instance()` in the injected
(non-NAPI) path.

---

## 6. Blast radius (precise)

`SessionManager::instance()` appears in **exactly one production file**:

```
codelet/agent-loop/src/agent_manager_handler.rs:43    create_handler  (sync: spawn/list/get_status/close/message/set_role)
codelet/agent-loop/src/agent_manager_handler.rs:909   create_async_handler (await_idle / profile)
(1301, 1315 are tests)
```

Every other per-session handler (session_search, deep_search, graph_search,
schedule, bridge) already uses injected references (`&inner_session`,
`session.id`, `session.project`) — **none** reach for the singleton. So the fix
is localized to the AgentManager handler + the plumbing that supplies it a
manager reference.

---

## 7. Call path (for reference)

```
LLM tool call: AgentManager{action:"spawn"}
  → codelet/tools/src/agent_manager/mod.rs        AgentManagerTool::call
  → codelet/tools/src/agent_manager/handler.rs    execute_agent_manager(session_id, action)
  → (registered closure)                          codelet/agent-loop/src/agent_manager_handler.rs::create_handler
        let session_manager = SessionManager::instance();   ← BUG (must be the injected manager)
  → handle_spawn(session_manager, …)              create_session_with_id on the WRONG manager

Registration site:
  codelet/agent-loop/src/agent_loop.rs:670        register_agent_manager_handler(session.id, &inner_session, project)
  codelet/agent-loop/src/bridges.rs:78            create_handler(project, model_string, ctx, max_out)
  Launched by:
  codelet/agent-loop/src/hooks.rs:38              FspecAgentHooks::spawn_agent_loop  (has `session: Arc<BackgroundSession>`)
```

The agent loop already holds `session: Arc<BackgroundSession>` — the session
created by the owning manager. The owning manager reference must be made
reachable from there and threaded into `create_handler` / `create_async_handler`.

---

## 8. Proposed fix (to be finalized during specifying)

**Goal:** the AgentManager handler operates on the *owning* `SessionManager`
(the one that created the spawner session), not `SessionManager::instance()`.

Recommended mechanism (injection, backward-compatible with NAPI):

1. Give `BackgroundSession` an optional back-reference to its owning manager,
   e.g. `owning_manager: Weak<SessionManager>` (empty by default).
2. Make `SessionManager` self-aware when wrapped in `Arc` (a `Weak<Self>` set via
   an `Arc` constructor / initializer). `build_service` already wraps it in
   `Arc` — use that path so the daemon manager populates `self_weak`.
3. In `create_session_with_id` / `create_isolated_session_with_id`, stamp the new
   session's `owning_manager` from `self_weak`.
4. Thread the resolved manager into the AgentManager handler:
   `register_agent_manager_handler` (and the async handler registration) pass the
   `Arc<SessionManager>` resolved from `session.owning_manager().upgrade()`.
   `create_handler` / `create_async_handler` capture it instead of calling
   `instance()`.
5. **Fallback for NAPI:** when `owning_manager` is absent (NAPI path, where the
   singleton legitimately owns everything), fall back to
   `SessionManager::instance()` so NAPI behaviour is byte-for-byte unchanged.

This keeps the singleton working for NAPI while making the injected `fspec`
binary correct, and does not touch the forbidden `napi` dependency.

> Note: the exact plumbing (Weak self-ref vs. trait-signature change vs. storing
> the handle on `BackgroundSession`) is a specifying-phase decision. The
> invariant the tests must lock down is behavioural, not structural (see §9).

---

## 9. Acceptance shape (drives Example Mapping → scenarios)

Behavioural invariants the fix must guarantee:

1. **Same-manager spawn:** A session created by manager *M* whose agent invokes
   `AgentManager::spawn` causes the subordinate to be created in **M**, not in
   `SessionManager::instance()`.
2. **Subordinate runs:** When *M* has real hooks (`FspecAgentHooks`), the spawned
   subordinate's agent loop starts (its `spawn_agent_loop` runs), so a subsequent
   `message` is processed and produces output chunks.
3. **Subordinate is visible:** Spawning fires `M.session_created_tx`, which the
   embedded backend's `session_created_rx()` (already wired in RPC-385) delivers,
   so the TUI creates a subordinate tab.
4. **Chain-of-command intact:** `add_supervisor(subordinate, spawner)` is recorded
   on **M**, and `list` / `get_status` / `close` / `message` from the spawner all
   resolve the subordinate on **M**.
5. **Async actions parity:** `await_idle` (and `profile`) operate on **M** too
   (the async handler must not use `instance()` in the injected path).
6. **NAPI unchanged:** With no owning-manager back-reference (NAPI path), the
   handler still resolves the singleton — existing AMGR tests stay green.
7. **No forbidden deps:** `codelet-agent-loop` / `codelet-sessions` gain no
   dependency on `codelet-napi`.

---

## 10. Verification

- New Rust integration test in `codelet/agent-loop/` (or `codelet/sessions/`):
  build a non-singleton `SessionManager` with stub hooks, register the
  AgentManager handler against it, invoke `spawn`, and assert the subordinate
  appears in **that** manager's `list_sessions()` and that
  `session_created_tx().subscribe()` received the event — while
  `SessionManager::instance().list_sessions()` does **not** contain it.
- Existing AMGR-* suites (`codelet/tools/src/agent_manager/tests.rs`,
  `codelet/agent-loop/tests/...`) must remain green (NAPI/singleton parity).
- `cargo build` + `cargo clippy` clean for `codelet-agent-loop`,
  `codelet-sessions`, `codelet-fspec`.
- Manual: run the `fspec` binary, spawn via AgentManager, confirm a live,
  message-responsive subordinate tab in the Rust TUI.
