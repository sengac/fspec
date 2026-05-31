# RPC-072 — Root Cause Analysis

> The fspec Rust binary installs a NO-OP `SessionManagerHooks` impl, so the
> agent loop never spawns and Work Agent sessions cannot drive an LLM.
> Typing into a session produces a `UserInput → Running → Idle` chunk burst
> and nothing else.

---

## 1. Observed Symptom

**Date observed:** 2026-05-27
**Branch:** `codelet-integration`
**Binary:** Rust `fspec`
**Reproduction:**

1. Launch `fspec` (the Rust binary, not the Node CLI).
2. Navigate to a DONE work unit on the BoardView.
3. Press Enter to open the Work Agent.
4. Type "please review this card" + Enter.

Observed chunk sequence (regardless of RPC-071's rendering fix):

```
UserInput { text: "please review this card" }
SessionStateChange { state: Running }
SessionStateChange { state: Idle }
```

After RPC-071, the user only sees:

```
user> please review this card
```

…and then nothing. No assistant reply. No tool calls. No `Done` chunk.
The session has effectively absorbed the input into the void.

---

## 2. Where the Wiring Stops

### 2.1 The chunk emission path is fine

`codelet/sessions/src/background_session.rs:1080-1108` — `send_input`:

```rust
pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String> {
    self.set_pending_input(None);
    self.handle_output(StreamChunk::user_input(input.clone()));   // ← UserInput chunk emitted
    self.set_status(SessionStatus::Running);                       // ← SessionStateChange(Running) emitted
    self.reset_interrupt();
    self.input_tx
        .try_send(PromptInput { input, thinking_config })          // ← here's where it dies
        .map_err(|e| {
            self.set_status(SessionStatus::Idle);                  // ← SessionStateChange(Idle) emitted on failure
            format!("Failed to send input: {}", e)
        })
}
```

The `try_send` is supposed to hand the prompt off to the agent loop, which
runs as a tokio task spawned via `SessionManagerHooks::spawn_agent_loop`.
The agent loop calls a Rust LLM provider, streams `Text` / `ToolCall` /
`Done` chunks back through `handle_output`, and finally sets status back
to Idle.

### 2.2 The fspec binary uses `NoopSessionManagerHooks`

`codelet/sessions/src/session_manager.rs:114-141`:

```rust
/// Default no-op implementation used by the `fspec` binary in RPC-044
/// where every NAPI subsystem is absent.
#[derive(Default)]
pub struct NoopSessionManagerHooks;

impl SessionManagerHooks for NoopSessionManagerHooks {
    fn spawn_agent_loop(
        &self,
        _session: Arc<BackgroundSession>,
        _input_rx: mpsc::Receiver<PromptInput>,    // ← dropped immediately
        _mcp_injection_rx: mpsc::Receiver<McpInjection>,
    ) {
    }
    // ... other no-op hooks
}
```

The `_input_rx: mpsc::Receiver<PromptInput>` parameter goes into a
function that does nothing. When the function returns, Rust drops the
receiver, which **closes the channel**. The very next `send_input` then
hits `TrySendError::Closed`, the error path fires `set_status(Idle)`, and
we see the `Running → Idle` flash in the screenshot.

### 2.3 The NAPI side has a real impl — the fspec binary does not

`codelet/napi/src/session_bindings.rs` (and friends) install a custom
`SessionManagerHooks` impl that spawns the actual agent loop via
`tokio::spawn(async move { agent_loop(session, input_rx, mcp_injection_rx).await })`.
That's why the Ink TUI (`src/tui/`) works — it's backed by NAPI, not by
the Rust binary's standalone SessionManager.

The fspec binary lives in `codelet/fspec/` and does NOT depend on
`codelet-napi`. It just inherits `NoopSessionManagerHooks` because nobody
ever wrote a NAPI-free agent-loop hook impl.

---

## 3. Why RPC-030 Was Supposed to Catch This

RPC-030's title:

> Wire BackgroundSession + agent management (/provider, /providers, /model)
> into the Rust AgentView via the SessionManagerHandle trait — NAPI-free
> RPC boundary audit + plan

Status: `backlog` (never started).

RPC-030 was scoped as the **planning** card for exactly this wiring. Its
acceptance was supposed to produce:

1. A boundary audit showing every NAPI dependency the Rust agent loop
   would need to replicate.
2. A plan for which crate owns the new hooks impl.
3. A plan for provider/model selection in a NAPI-free environment.
4. A plan for tool injection (Read/Write/Edit/Bash etc.) without NAPI.

RPC-031 through RPC-067 then assumed RPC-030's plan was in place and
built `codelet-sessions`, `codelet-rpc-types`, the AgentView, the
SessionFooter, the dialogs, the slash commands, the cross-transport
parity tests… all on top of an unimplemented planning card.

The hooks trait abstraction (`SessionManagerHooks`) was added in RPC-040
precisely so the fspec binary COULD have a different impl from NAPI —
but the NAPI-free impl itself was deferred to RPC-030 and never written.

**RPC-072 supersedes RPC-030 by doing the wiring directly.** A pure
planning card with no actual implementation buys us nothing now that the
symptom is user-visible and reproducible.

---

## 4. The Five Things RPC-072 Must Deliver

### 4.1 A NAPI-free `SessionManagerHooks` impl

New module: `codelet/fspec/src/agent_hooks.rs` (or a new
`codelet/agent-loop` crate if the impl grows beyond ~400 LoC).

Owns:

```rust
pub struct FspecAgentHooks {
    pub provider_registry: Arc<dyn ProviderRegistry>,
    pub tools_factory: Arc<dyn ToolFactory>,
    pub runtime: tokio::runtime::Handle,
}

impl SessionManagerHooks for FspecAgentHooks {
    fn spawn_agent_loop(
        &self,
        session: Arc<BackgroundSession>,
        input_rx: mpsc::Receiver<PromptInput>,
        mcp_injection_rx: mpsc::Receiver<McpInjection>,
    ) {
        let provider_registry = self.provider_registry.clone();
        let tools_factory = self.tools_factory.clone();
        self.runtime.spawn(async move {
            agent_loop(session, input_rx, mcp_injection_rx,
                       provider_registry, tools_factory).await
        });
    }
    // ... scheduler / footer / cleanup hooks ...
}
```

### 4.2 A NAPI-free `agent_loop` async function

Reads from `input_rx`, builds a `rig::Agent` (or whatever
`codelet-providers` exposes for NAPI-free use), invokes
`agent.prompt(text).await`, streams the result chunks back through
`session.handle_output(chunk)`, and finally calls
`session.set_status(SessionStatus::Idle)`.

This already exists NAPI-side — the work is to lift it out of NAPI's
agent_loop into a crate that doesn't depend on `napi`.

### 4.3 Provider + model resolution

The fspec binary reads `~/.fspec/config.json` (the same config the Ink
frontend uses) and resolves:

- Default provider id (e.g. `anthropic`, `openai`, `claude-code`, `stub`).
- Default model id (e.g. `claude-opus-4-5`).
- API key from env / config / OS keychain.

`/provider` and `/model` slash commands route through the same registry.

### 4.4 Tool injection

`codelet-tools` already builds the tool registry NAPI-free
(`codelet/tools/src/lib.rs:200` exports the tools). The fspec binary's
agent loop wires the same factories so the agent has access to Read,
Write, Edit, Bash, Grep, Ls, AstGrep, etc.

### 4.5 Wire it all into `codelet-fspec::common::build_service`

That's the function the fspec binary calls at startup. After RPC-072:

```rust
pub fn build_service(...) -> Arc<FspecService<...>> {
    let session_manager = Arc::new(SessionManager::new());
    let hooks = Arc::new(FspecAgentHooks::new(
        provider_registry,
        tools_factory,
        tokio::runtime::Handle::current(),
    ));
    session_manager.set_hooks(hooks);
    // ... rest of the existing wiring
}
```

---

## 5. Crate Boundary Constraint

The new `FspecAgentHooks` impl MUST NOT pull in `codelet-napi`. RPC-067
introduced dependency-rule regression tests at:

- `codelet/fspec/tests/no_napi_dependency.rs`
- `codelet/sessions/tests/no_napi_dependency.rs`

These MUST still pass after RPC-072 lands.

The Rig agent + provider crates (`codelet-providers`) are already
NAPI-free (they're consumed by both the Rust binary path and the NAPI
path), so the wiring should slot in cleanly.

---

## 6. Dependency on RPC-069

The end-to-end test in `test-plan.md` uses the **stub** provider so the
test is deterministic and doesn't require network access or API keys.
RPC-069 is currently blocked on routing `ProviderType::Custom("stub")`
through the in-memory `LlmProvider` registry. RPC-072's acceptance
includes unblocking and resolving RPC-069 (the registry change is
small — the bigger blocker was the absence of an agent loop calling the
registry at all, which RPC-072 fixes).

---

## 7. Why This Was Hidden for So Long

- The Ink TUI works (NAPI-backed) — most contributors test via Ink.
- The fspec binary's `cargo test --workspace` passes — every unit test
  uses `#[tokio::test]` to spin up its own runtime and either mocks the
  agent loop or skips it entirely.
- The cross-frontend integration test (RPC-066) uses the stub provider
  but is marked `#[ignore]` pending RPC-069.
- The e2e tui-test scaffolding (RPC-068) tests the BoardView render but
  not a round-trip prompt.

Every single test gate the project has runs to green while the binary's
core feature — actually talking to an LLM — is broken.

---

## 8. Severity

User-facing: **complete loss of the binary's primary value proposition**.
The Work Agent is the headline feature of the Rust frontend. Without it,
the binary is a fancy BoardView renderer with a chat input that swallows
text.

This card is the highest-priority RPC card in the backlog.
