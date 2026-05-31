# RPC-072 — Architecture & Crate Layout

> Where the new code lives, what depends on what, and why.

---

## 1. Crate Dependency Diagram (After RPC-072)

```
                ┌─────────────────────────────────────────────┐
                │              codelet-rpc-types              │  (no deps)
                └─────────────────────────────────────────────┘
                                     ▲
                                     │
                ┌─────────────────────────────────────────────┐
                │                codelet-core                 │  (no NAPI)
                │  - SessionManagerHandle trait              │
                │  - ProviderRegistry trait (lifted here)    │
                │  - ToolFactory trait (lifted here)         │
                └─────────────────────────────────────────────┘
                            ▲                       ▲
                            │                       │
       ┌────────────────────┴────┐    ┌─────────────┴─────────────┐
       │     codelet-sessions    │    │     codelet-providers     │
       │  - SessionManager       │    │  - StubProvider           │
       │  - BackgroundSession    │    │  - AnthropicProvider      │
       │  - SessionManagerHooks  │    │  - ClaudeCodeProvider     │
       │  - NoopSessionManager…  │    │  - OpenAIProvider         │
       └─────────────────────────┘    │  - GeminiProvider, etc.   │
                ▲                     │  - Rig-Agent constructors │
                │                     └───────────────────────────┘
                │                                 ▲
                │                                 │
                │                     ┌───────────┴───────────────┐
                │                     │      codelet-tools        │
                │                     │  - Read / Write / Edit    │
                │                     │  - Bash / Grep / Ls       │
                │                     │  - AstGrep / etc.         │
                │                     └───────────────────────────┘
                │                                 ▲
                │                                 │
       ┌────────┴─────────────────────────────────┴───────────┐
       │              codelet-agent-loop   ★ NEW ★              │
       │  - agent_loop(session, input_rx, mcp_rx,             │
       │               registry, factory) -> impl Future      │
       │  - FspecAgentHooks: SessionManagerHooks              │
       │  - Provider + model resolution                       │
       │  - Tool factory wiring                               │
       │  Crate marker: NO codelet-napi dep                   │
       └──────────────────────────────────────────────────────┘
                                ▲
                                │
                ┌───────────────┴──────────────────┐
                │            codelet-fspec         │
                │  - common::build_service         │
                │  - installs FspecAgentHooks      │
                │  - main.rs entry                 │
                └──────────────────────────────────┘
                                │
                                ▼
                ┌─────────────────────────────────────┐
                │  codelet-fspec-tui (AgentView etc.) │
                └─────────────────────────────────────┘

                ┌─────────────────────────────────────────────┐
                │             codelet-napi (unchanged)        │
                │  Has its own hooks impl + agent loop.       │
                │  Continues to back the Ink TUI.             │
                └─────────────────────────────────────────────┘
```

★ NEW ★ = the only new crate. Lives at `codelet/agent-loop/`.

---

## 2. Why a New Crate?

We could put `FspecAgentHooks` directly inside `codelet/fspec/src/`, but a
dedicated crate has three benefits:

1. **Boundary enforcement.** `codelet-agent-loop` declares zero dep on
   `codelet-napi`. The new `no_napi_dependency` test there is a single
   `cargo check` away from a clear contract.
2. **Reusability.** Anyone building a non-NAPI binary on top of
   `codelet-sessions` (e.g. a future headless daemon, a CLI test harness)
   can depend on `codelet-agent-loop` instead of re-implementing the
   wiring.
3. **Test isolation.** The integration tests for the agent loop are
   chunky (spin up tokio runtimes, simulate provider responses). Keeping
   them in their own crate keeps `codelet-fspec`'s test surface small.

---

## 3. `codelet-agent-loop` Crate Layout

```
codelet/agent-loop/
├── Cargo.toml
├── src/
│   ├── lib.rs                 # Re-exports + crate docs.
│   ├── hooks.rs               # FspecAgentHooks impl.
│   ├── agent_loop.rs          # async fn agent_loop(...).
│   ├── provider_resolve.rs    # Config → provider/model.
│   ├── tools.rs               # Tool factory wiring.
│   └── error.rs               # AgentLoopError enum.
└── tests/
    ├── no_napi_dependency.rs           # Crate-graph regression.
    ├── stub_provider_round_trip.rs     # Live end-to-end with stub.
    ├── mcp_injection.rs                # MCP path coverage.
    └── interrupt_during_stream.rs      # Esc-interrupt parity.
```

---

## 4. The `SessionManagerHooks` Wiring Path

### 4.1 Before RPC-072 (broken)

```rust
// codelet/fspec/src/common.rs
pub fn build_service(...) -> Arc<FspecService<...>> {
    let session_manager = Arc::new(SessionManager::new());
    // ↑ Defaults to NoopSessionManagerHooks. Nothing else installed.
    // ↓ Service is wired up but every send_input vanishes into the void.
    let service = FspecService::new(session_manager, ...);
    Arc::new(service)
}
```

### 4.2 After RPC-072 (fixed)

```rust
// codelet/fspec/src/common.rs
use codelet_agent_loop::FspecAgentHooks;
use codelet_providers::default_provider_registry;
use codelet_tools::default_tool_factory;

pub fn build_service(config: &FspecConfig) -> Arc<FspecService<...>> {
    let session_manager = Arc::new(SessionManager::new());

    // Resolve provider/model from config.
    let provider_registry = default_provider_registry(config);
    let tools_factory     = default_tool_factory(config);

    let hooks = Arc::new(FspecAgentHooks::new(
        provider_registry,
        tools_factory,
        tokio::runtime::Handle::current(),
    ));
    session_manager.set_hooks(hooks);

    // Tell SessionManager what the default model is so create_session
    // can fill it in without an explicit caller-supplied value.
    session_manager.set_default_model(config.default_model.clone());

    let service = FspecService::new(session_manager, ...);
    Arc::new(service)
}
```

---

## 5. `agent_loop` Pseudocode

```rust
pub async fn agent_loop(
    session: Arc<BackgroundSession>,
    mut input_rx: mpsc::Receiver<PromptInput>,
    mut mcp_rx:   mpsc::Receiver<McpInjection>,
    provider_registry: Arc<dyn ProviderRegistry>,
    tools_factory:     Arc<dyn ToolFactory>,
) {
    loop {
        tokio::select! {
            // Highest priority: handle MCP context injections.
            Some(mcp) = mcp_rx.recv() => {
                handle_mcp_injection(&session, mcp).await;
            }
            Some(prompt) = input_rx.recv() => {
                if let Err(err) = run_one_turn(
                    &session,
                    &prompt,
                    &provider_registry,
                    &tools_factory,
                ).await {
                    session.handle_output(StreamChunk::error(err.to_string()));
                }
                session.handle_output(StreamChunk::Done);
                session.set_status(SessionStatus::Idle);
            }
            else => break, // both channels closed → session destroyed
        }
    }
}

async fn run_one_turn(
    session: &BackgroundSession,
    prompt: &PromptInput,
    registry: &dyn ProviderRegistry,
    factory: &dyn ToolFactory,
) -> Result<(), AgentLoopError> {
    // 1. Resolve provider for this session.
    let provider_id = session.provider_id.read()?.clone()
        .unwrap_or_else(|| registry.default_provider_id());
    let provider = registry.get(&provider_id)
        .ok_or(AgentLoopError::UnknownProvider(provider_id))?;

    // 2. Build a Rig Agent with the session's tools and history.
    let mut agent = provider.build_agent(session, factory).await?;

    // 3. Stream the response chunks back through handle_output.
    let mut stream = agent.prompt_stream(&prompt.input).await?;
    while let Some(event) = stream.next().await {
        // Translate provider event → StreamChunk and emit.
        let chunks = translate_event(event)?;
        for chunk in chunks {
            // Check interrupt flag — see RPC-051 contract.
            if session.is_interrupted() {
                session.handle_output(
                    StreamChunk::interrupted(stream.queued_inputs()));
                return Ok(());
            }
            session.handle_output(chunk);
        }
    }
    Ok(())
}
```

---

## 6. Why Streaming Through `handle_output` Is the Right Boundary

`BackgroundSession::handle_output` is already responsible for:

- Pushing chunks into the per-session `output_buffer` (for resume/attach).
- Broadcasting on `chunks_tx` so RPC subscribers (AgentView, Ink TUI)
  pick them up live.
- Updating the on-disk message store (persistence).

So the agent loop never has to know about the AgentView. It just emits
`StreamChunk`s through one function, and every downstream consumer
(scrollback, footer, status pill, persistence) sees a consistent stream.
This is exactly the pattern RPC-041 established when broadcasting was
lifted out of NAPI.

---

## 7. Provider Resolution Contract

Order of precedence (highest first):

1. Per-session override set via `/provider <id>` slash command (stored
   on `BackgroundSession.provider_id`).
2. Default model in `~/.fspec/config.json` (`default_provider` /
   `default_model` keys).
3. `FSPEC_DEFAULT_PROVIDER` / `FSPEC_DEFAULT_MODEL` env vars.
4. Hardcoded fallback: `anthropic` / `claude-opus-4-5` (matches the
   NAPI agent's existing fallback).

API keys resolved via:

1. `FSPEC_<PROVIDER>_API_KEY` env var.
2. `~/.fspec/config.json` `providers.<id>.api_key`.
3. OS keychain entry `fspec.<provider-id>`.
4. Returning a `StreamChunk::error("missing API key for ...")` chunk
   if none of the above resolves.

For the integration tests, the **stub provider** sidesteps all of this
by short-circuiting `provider_id = "stub"` to an in-process
`StubProvider` that returns deterministic responses. See RPC-069 for the
registry routing.

---

## 8. Cargo.toml for the New Crate

```toml
# codelet/agent-loop/Cargo.toml
[package]
name = "codelet-agent-loop"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true, features = ["sync", "rt", "macros"] }
tracing = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
futures = { workspace = true }

codelet-core      = { path = "../core" }
codelet-rpc-types = { path = "../rpc-types" }
codelet-sessions  = { path = "../sessions" }
codelet-providers = { path = "../providers" }
codelet-tools     = { path = "../tools" }

# Explicit anti-dep: codelet-napi MUST NOT appear here.
# Enforced by codelet/agent-loop/tests/no_napi_dependency.rs

[dev-dependencies]
tokio = { workspace = true, features = ["sync", "rt-multi-thread", "macros", "test-util"] }
```

---

## 9. Risk: Lifting Agent Loop Out of NAPI Without Breaking NAPI

The NAPI side currently owns its own copy of the agent loop in
`codelet/napi/src/agent_loop.rs`. After RPC-072, two options:

**Option A: Diverged copies.** NAPI keeps its agent loop, the new crate
ships a fresh one. Risk: drift over time.

**Option B (Recommended): NAPI re-uses `codelet-agent-loop`.** NAPI's
`NapiSessionManagerHooks::spawn_agent_loop` simply delegates to
`codelet_agent_loop::agent_loop(...)` with NAPI-resolved provider /
tool factory. Single source of truth.

Option B is the right call, but it's a larger refactor. **For RPC-072
we ship Option A** and create a follow-up card (RPC-073) to consolidate.
The boundary tests at `codelet/sessions/tests/no_napi_dependency.rs`
already prevent NAPI types from leaking back into `codelet-sessions`,
so divergence in `codelet-napi`-side specifics won't infect the binary.
