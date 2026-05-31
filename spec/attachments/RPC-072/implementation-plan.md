# RPC-072 — Implementation Plan

> Phased, file-level walkthrough. Each phase is independently testable so
> the card can be split into smaller PRs if 13 points is too coarse.

---

## 1. Phase Breakdown

| Phase | Description | Estimate |
|-------|-------------|----------|
| P1 | Create `codelet-agent-loop` crate skeleton + `no_napi_dependency` boundary test | 1 pt |
| P2 | Lift / re-implement `agent_loop` in the new crate (NAPI-free) | 5 pts |
| P3 | Implement `FspecAgentHooks` impl of `SessionManagerHooks` | 2 pts |
| P4 | Wire into `codelet-fspec::common::build_service` | 2 pts |
| P5 | Provider/model resolution + `/provider`, `/providers`, `/model` slash command end-to-end | 2 pts |
| P6 | End-to-end integration test via in-memory tarpc duplex + stub provider | 1 pt |
| **Total** | | **13 pts** |

13 is the upper limit per the estimation scale. If during P2 the agent
loop turns out to need more than 5 points, split into RPC-072a / 072b
along the phase boundary.

---

## 2. Phase 1 — Crate Skeleton

### 2.1 Files to create

```
codelet/agent-loop/
├── Cargo.toml         (per architecture.md)
├── src/
│   ├── lib.rs         (re-exports + crate-level docs)
│   ├── hooks.rs       (FspecAgentHooks stub)
│   ├── agent_loop.rs  (agent_loop stub)
│   └── error.rs       (AgentLoopError enum)
└── tests/
    └── no_napi_dependency.rs
```

### 2.2 `no_napi_dependency.rs`

Mirror the existing `codelet/sessions/tests/no_napi_dependency.rs`:

```rust
#[test]
fn agent_loop_crate_does_not_depend_on_napi() {
    let lockfile_path = std::env::current_dir().unwrap()
        .ancestors()
        .find(|p| p.join("Cargo.lock").exists())
        .expect("Cargo.lock not found")
        .join("Cargo.lock");
    let lockfile = std::fs::read_to_string(&lockfile_path).unwrap();
    let agent_loop_pkg = extract_pkg("codelet-agent-loop", &lockfile);
    assert!(
        !agent_loop_pkg.dependencies.iter().any(|d| d.starts_with("codelet-napi")),
        "codelet-agent-loop must not depend on codelet-napi, but it does: {:?}",
        agent_loop_pkg.dependencies,
    );
    // Also assert no transitive napi-derive / napi crates.
    assert!(
        !lockfile.contains("\nname = \"napi\""),
        "napi crate appeared in lockfile — codelet-agent-loop may have a transitive napi dep"
    );
}
```

### 2.3 Workspace registration

Edit `codelet/Cargo.toml`:

```toml
[workspace]
members = [
    # ...
    "agent-loop",        # ★ NEW ★
    # ...
]
```

---

## 3. Phase 2 — Lift `agent_loop`

### 3.1 Source the NAPI implementation

Read `codelet/napi/src/agent_loop.rs` carefully. It does:

1. Receive `PromptInput` from `input_rx`.
2. Build a Rig Agent via `codelet-providers::build_agent(...)`.
3. Stream chunks back through `session.handle_output`.
4. Handle interrupts via `session.is_interrupted()`.
5. Handle MCP injections from `mcp_injection_rx`.

### 3.2 Identify NAPI-specific bits

The NAPI version touches:

- `napi::ThreadsafeFunction` — for old callback-based chunk delivery.
  ALREADY REMOVED by RPC-041 (replaced with tokio broadcast). Should
  no longer be present.
- `napi::Error` — used in some error paths. Replace with `AgentLoopError`.
- Direct `napi_derive::napi` attrs — none should remain after RPC-041.

If RPC-041 was thorough, the NAPI `agent_loop.rs` is already mostly
NAPI-free. Verify by `cargo check -p codelet-napi --no-default-features`.

### 3.3 Move to new crate

Copy `codelet/napi/src/agent_loop.rs` into
`codelet/agent-loop/src/agent_loop.rs`. Fix the imports. Replace any
remaining NAPI types with their `codelet-rpc-types` equivalents.

### 3.4 Re-export back to NAPI (Phase 2b)

For now, keep the NAPI version in place (Option A from architecture.md
§9). Later card RPC-073 will swap NAPI's hooks impl to delegate to
`codelet_agent_loop::agent_loop`.

### 3.5 Coverage

Add unit tests in `codelet/agent-loop/tests/`:

- `agent_loop_drains_input_rx.rs` — feed prompts, assert chunks arrive.
- `agent_loop_handles_interrupt.rs` — set is_interrupted mid-stream,
  assert `Interrupted` chunk and clean shutdown.
- `agent_loop_emits_done_at_end_of_turn.rs` — every prompt ends with
  `StreamChunk::Done` + `SessionStatus::Idle`.

All use the stub provider via a hand-written `MockProvider` so they
don't depend on RPC-069.

---

## 4. Phase 3 — `FspecAgentHooks`

```rust
// codelet/agent-loop/src/hooks.rs
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::runtime::Handle;

use codelet_core::SessionManagerHooks;
use codelet_sessions::{BackgroundSession, McpInjection, PromptInput};

use crate::agent_loop::agent_loop;
use crate::{ProviderRegistry, ToolFactory};

pub struct FspecAgentHooks {
    provider_registry: Arc<dyn ProviderRegistry>,
    tools_factory: Arc<dyn ToolFactory>,
    runtime: Handle,
}

impl FspecAgentHooks {
    pub fn new(
        provider_registry: Arc<dyn ProviderRegistry>,
        tools_factory:     Arc<dyn ToolFactory>,
        runtime:           Handle,
    ) -> Self {
        Self { provider_registry, tools_factory, runtime }
    }
}

impl SessionManagerHooks for FspecAgentHooks {
    fn spawn_agent_loop(
        &self,
        session: Arc<BackgroundSession>,
        input_rx: mpsc::Receiver<PromptInput>,
        mcp_injection_rx: mpsc::Receiver<McpInjection>,
    ) {
        let registry = self.provider_registry.clone();
        let factory  = self.tools_factory.clone();
        self.runtime.spawn(async move {
            agent_loop(session, input_rx, mcp_injection_rx, registry, factory).await
        });
    }

    fn spawn_scheduler(&self, _project: String, _rt: Handle) {
        // RPC-058 path — re-use codelet-core::scheduler when wired.
        // For now, no-op; see RPC-058 hook impl.
    }

    fn ensure_scheduler_running_for_loop(&self, _: String, _: Handle) {}
    fn spawn_footer_poller(&self, _: String, _: String, _: Option<String>) {}
    fn stop_footer_poller(&self, _: &str) {}
    fn cleanup_session_loops(&self, _: uuid::Uuid) {}
}
```

The scheduler / footer / cleanup hooks are no-ops for now — they can be
wired later without blocking the headline feature.

---

## 5. Phase 4 — Wire Into `build_service`

`codelet/fspec/src/common.rs`:

```rust
use codelet_agent_loop::FspecAgentHooks;
use codelet_providers::default_provider_registry;
use codelet_tools::default_tool_factory;

pub fn build_service(config: &FspecConfig) -> Arc<FspecService<...>> {
    let session_manager = Arc::new(SessionManager::new());

    let provider_registry = default_provider_registry(config);
    let tools_factory     = default_tool_factory(config);

    let hooks = Arc::new(FspecAgentHooks::new(
        provider_registry,
        tools_factory,
        tokio::runtime::Handle::current(),
    ));
    session_manager.set_hooks(hooks);
    session_manager.set_default_model(config.default_model.clone());

    let service = FspecService::new(session_manager, ...);
    Arc::new(service)
}
```

`codelet/fspec/Cargo.toml` gains:

```toml
codelet-agent-loop = { path = "../agent-loop" }
```

---

## 6. Phase 5 — Provider/Model Slash Commands

The slash command UI plumbing already exists from RPC-022, RPC-054, etc.
What's missing is the **backend** behaviour:

- `backend.list_providers() -> Vec<ProviderInfo>` — reads from registry.
- `backend.set_provider(session_id, provider_id) -> Result<()>` — writes
  `BackgroundSession.provider_id`.
- `backend.list_models(provider_id) -> Vec<ModelEntry>` — calls
  `Provider::list_models()`.
- `backend.set_model(session_id, model_id) -> Result<()>` — writes
  `BackgroundSession.model_id`.

These are likely already in `SessionManagerHandle` (RPC-037 widened it).
Verify the wiring in `codelet/sessions/src/handle_impl.rs` is calling
through to the registry, and that the registry isn't always returning
empty.

---

## 7. Phase 6 — End-to-End Integration Test

See `test-plan.md` for the full spec. Quick summary:

```rust
// codelet/agent-loop/tests/work_agent_end_to_end_rpc072.rs

#[tokio::test(flavor = "multi_thread")]
async fn fresh_fspec_binary_session_replies_to_input() {
    // 1. Build a SessionManager with FspecAgentHooks installed,
    //    using the stub provider registry.
    let registry = test_helpers::stub_provider_registry();
    let factory  = test_helpers::stub_tool_factory();
    let sm = Arc::new(SessionManager::new());
    sm.set_hooks(Arc::new(FspecAgentHooks::new(
        registry, factory, Handle::current(),
    )));

    // 2. Create a session.
    let sid = sm.create_session("stub/echo", "/tmp").await.unwrap();
    let session = sm.get_session(&sid).unwrap();

    // 3. Subscribe to the chunks broadcast.
    let mut rx = sm.chunks_tx().subscribe();

    // 4. Send input.
    session.send_input("hello".to_string(), None).unwrap();

    // 5. Drain chunks until Done.
    let mut got_text = false;
    let timeout_at = Instant::now() + Duration::from_secs(5);
    while Instant::now() < timeout_at {
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Ok((_, StreamChunk::Text { .. }))) => { got_text = true; }
            Ok(Ok((_, StreamChunk::Done))) => break,
            _ => {}
        }
    }

    assert!(got_text, "session must emit at least one Text chunk");
}
```

---

## 8. Hook into the existing fspec binary

After P4, manually:

```bash
cd codelet
cargo run -p codelet-fspec --bin fspec
```

- BoardView renders work units.
- Navigate to a DONE work unit, press Enter.
- Type "hello".
- Should see:

```
user> hello
assistant> Hello! ... (whatever stub or live provider returns)
[done]
```

If `[done]` doesn't appear or `assistant>` is missing, the wiring is
incomplete — likely an issue with provider resolution. Use
`RUST_LOG=codelet_agent_loop=trace` to debug.

---

## 9. Rollback Plan

If something goes catastrophically wrong, revert to the previous
`build_service` (which uses `NoopSessionManagerHooks`). The binary
reverts to its pre-RPC-072 behaviour (input vanishes, but BoardView
still works). All other RPC functionality is unaffected because the
hooks abstraction was added in RPC-040 specifically to support this
kind of swap.

---

## 10. Documentation Updates

When the card lands, update:

- `codelet/agent-loop/README.md` — what the crate does, why it exists.
- `spec/FOUNDATION.md` § "Bounded Contexts" — register `agent-loop`.
- `codelet/CLAUDE.md` (if exists) — note the new crate so future
  contributors know where the agent loop lives.

---

## 11. Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `agent_loop` has hidden NAPI dependencies after the lift | M | `no_napi_dependency` boundary test catches at `cargo test` |
| Stub provider doesn't exist yet (RPC-069 blocked) | H | Hand-write a `MockProvider` in test-helpers for the integration test; RPC-069 unblocks later |
| Provider registry pattern doesn't exist yet | M | Lift from `codelet/napi/src/...` if needed; create `ProviderRegistry` trait in `codelet-core` |
| 13 points underestimates | L | Phase split into 072a / 072b along Phase 5 boundary if needed |
| Streaming `Text` merge needed for parity | L | Out of scope; tracked separately. Single `Text` chunks per turn are acceptable for RPC-072 acceptance. |

---

## 12. Definition of Done

- [ ] `codelet-agent-loop` crate exists at `codelet/agent-loop/`.
- [ ] `no_napi_dependency` test passes.
- [ ] `agent_loop` async fn lifted and tested.
- [ ] `FspecAgentHooks` impl complete.
- [ ] `codelet-fspec::build_service` installs the hooks.
- [ ] `/provider`, `/providers`, `/model` slash commands return real data.
- [ ] End-to-end integration test passes (stub provider).
- [ ] Manual binary smoke test: typed input produces assistant reply.
- [ ] `cargo test --workspace` passes.
- [ ] RPC-030 closed as superseded by RPC-072.
- [ ] RPC-069 unblocked.
- [ ] Coverage links updated.
