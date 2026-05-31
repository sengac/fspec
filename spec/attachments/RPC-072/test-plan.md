# RPC-072 — Test Plan

> The test surface that proves the wiring works and prevents regression.

---

## 1. Test Pyramid

```
                    ┌───────────────────────────────┐
                    │  Manual binary smoke test      │   1 case (gating release)
                    └───────────────────────────────┘
                ┌──────────────────────────────────────┐
                │  End-to-end via tarpc duplex          │   3 cases
                └──────────────────────────────────────┘
            ┌────────────────────────────────────────────┐
            │  Integration: codelet-agent-loop crate      │   ~10 cases
            └────────────────────────────────────────────┘
        ┌──────────────────────────────────────────────────┐
        │  Boundary tests (no_napi_dependency)              │   2 cases
        └──────────────────────────────────────────────────┘
    ┌──────────────────────────────────────────────────────────┐
    │  Unit tests inside codelet-agent-loop                     │   ~20 cases
    └──────────────────────────────────────────────────────────┘
```

---

## 2. Unit Tests (`codelet/agent-loop/src/**/*.rs`, in-file)

### 2.1 `agent_loop::tests`

| Name | Behaviour |
|------|-----------|
| `drains_input_rx_and_emits_text_chunks` | Feed 1 prompt, assert ≥1 `Text` + 1 `Done` |
| `idle_status_after_done` | After `Done`, `session.get_status() == Idle` |
| `interrupt_during_stream_emits_interrupted_chunk` | Set `is_interrupted` mid-stream, assert `Interrupted` |
| `mcp_injection_takes_priority_over_input` | Both channels have items, MCP wins |
| `unknown_provider_id_emits_error_chunk` | Provider id "bogus" → `Error { error: "..." }` |
| `missing_api_key_emits_error_chunk` | Provider needs key, none present → `Error { ... }` |
| `tool_call_chunk_is_forwarded` | Provider emits ToolCall, agent_loop relays it |
| `closed_input_rx_breaks_loop_cleanly` | Drop sender → loop exits, no panic |
| `multiple_turns_share_provider_instance` | Two prompts → same provider used twice |
| `streaming_text_chunks_are_relayed_in_order` | 3 streaming Text chunks → 3 in scrollback in order |

### 2.2 `hooks::tests`

| Name | Behaviour |
|------|-----------|
| `spawn_agent_loop_drains_input_channel` | Verify `input_rx` is consumed (i.e. NOT dropped) |
| `runtime_handle_is_used_for_spawn` | Spawning happens on the configured runtime, not `tokio::spawn` global |

### 2.3 `provider_resolve::tests`

| Name | Behaviour |
|------|-----------|
| `per_session_provider_id_wins_over_default` | Session has provider_id="anthropic", default="openai" → anthropic |
| `default_model_used_when_session_has_none` | Session.model_id == None → uses default |
| `env_var_resolution_falls_through_to_config` | env unset, config set → config value used |
| `unresolvable_provider_returns_error` | All sources empty → `AgentLoopError::NoProviderConfigured` |

---

## 3. Boundary Tests

### 3.1 `codelet/agent-loop/tests/no_napi_dependency.rs`

Per `implementation-plan.md` §2.2.

### 3.2 `codelet/fspec/tests/no_napi_dependency.rs`

Existing test — must continue to pass. The new `codelet-agent-loop` dep
is allowed; what's forbidden is a transitive `codelet-napi`.

---

## 4. Integration Tests (`codelet/agent-loop/tests/`)

### 4.1 `stub_provider_round_trip.rs`

```rust
//! Feature: spec/features/rpc072-work-agent-roundtrip.feature

#[tokio::test(flavor = "multi_thread")]
async fn stub_provider_input_to_reply() {
    // @step Given a SessionManager with FspecAgentHooks installed and the
    //       stub provider registered
    let registry = test_helpers::stub_provider_registry();
    let factory  = test_helpers::empty_tool_factory();
    let sm = Arc::new(SessionManager::new());
    sm.set_hooks(Arc::new(FspecAgentHooks::new(
        registry, factory, tokio::runtime::Handle::current(),
    )));

    // @step And a session is created with provider="stub" model="echo"
    let sid = sm.create_session("stub/echo", "/tmp").await.unwrap();
    let session = sm.get_session(&sid).unwrap();

    // @step And a subscriber is attached to the chunks broadcast
    let mut rx = sm.chunks_tx().subscribe();

    // @step When the user sends input "hello"
    session.send_input("hello".to_string(), None).unwrap();

    // @step Then within 5 seconds at least one StreamChunk::Text arrives
    let mut chunks = Vec::new();
    let timeout_at = Instant::now() + Duration::from_secs(5);
    while Instant::now() < timeout_at {
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Ok((_, chunk))) => {
                let is_done = matches!(chunk, StreamChunk::Done);
                chunks.push(chunk);
                if is_done { break; }
            }
            _ => {}
        }
    }

    // @step And a StreamChunk::Done arrives after the Text chunks
    let text_count = chunks.iter()
        .filter(|c| matches!(c, StreamChunk::Text { .. }))
        .count();
    assert!(text_count >= 1, "expected ≥1 Text chunk, got chunks: {:?}", chunks);
    assert!(matches!(chunks.last(), Some(StreamChunk::Done)),
        "stream must end with Done, got: {:?}", chunks.last());

    // @step And the session's status returns to Idle after the turn
    assert_eq!(session.get_status(), SessionStatus::Idle);
}
```

### 4.2 `interrupt_mid_stream.rs`

```rust
#[tokio::test(flavor = "multi_thread")]
async fn esc_interrupt_emits_interrupted_chunk() {
    let registry = test_helpers::slow_stub_provider_registry();  // 1s/chunk
    // ... wiring ...

    session.send_input("write me a long essay".to_string(), None).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    session.interrupt();  // simulate Esc

    let chunks = drain_until_done(&mut rx, Duration::from_secs(3)).await;
    assert!(chunks.iter().any(|c| matches!(c, StreamChunk::Interrupted { .. })),
        "expected Interrupted chunk after .interrupt(), got: {:?}", chunks);
}
```

### 4.3 `provider_switch_mid_session.rs`

```rust
#[tokio::test(flavor = "multi_thread")]
async fn slash_provider_changes_runtime_target() {
    // ... wiring ...

    session.send_input("hello".to_string(), None).unwrap();
    drain_until_done(&mut rx, Duration::from_secs(3)).await;

    session.set_model(Some("alt-stub".to_string()), Some("echo".to_string()));
    session.send_input("again".to_string(), None).unwrap();

    let chunks = drain_until_done(&mut rx, Duration::from_secs(3)).await;
    let text = extract_text(&chunks);
    assert!(text.contains("[alt-stub]"),
        "second turn must use the alt-stub provider, got: {text}");
}
```

---

## 5. End-to-End via tarpc Duplex (`codelet/fspec/tests/`)

### 5.1 `work_agent_end_to_end_rpc072.rs`

The DEFINITIVE acceptance test. Spins up the fspec service via an
in-memory tarpc duplex (the same transport `codelet/rpc` uses for
RPC-070's regression test) and proves the user-facing path works.

```rust
//! Feature: spec/features/rpc072-work-agent-roundtrip.feature
//!
//! End-to-end regression guard: every layer from tarpc dispatch
//! through SessionManagerHandle::send_input through FspecAgentHooks
//! through the agent loop through the stub provider works.

#[tokio::test(flavor = "multi_thread")]
async fn end_to_end_input_produces_text_reply() {
    // @step Given a fspec service spun up with FspecAgentHooks + stub provider
    let service = test_helpers::build_test_service_with_stub_provider().await;

    // @step And a tarpc client connected via in-memory duplex
    let client = test_helpers::tarpc_duplex_client(service.clone()).await;

    // @step When the client creates a session
    let sid = client.create_session(context::current(), None).await.unwrap();

    // @step And the client subscribes to the chunk stream
    let mut chunk_stream = client.subscribe_chunks(context::current(), sid.clone())
        .await.unwrap();

    // @step And the client sends input "hello"
    client.send_input(context::current(), sid.clone(), "hello".into()).await.unwrap();

    // @step Then within 10 seconds a StreamChunk::Text arrives
    let timeout = Duration::from_secs(10);
    let mut got_text = false;
    let mut got_done = false;
    let start = Instant::now();
    while start.elapsed() < timeout {
        match tokio::time::timeout(Duration::from_millis(100), chunk_stream.next()).await {
            Ok(Some(StreamChunk::Text { .. })) => got_text = true,
            Ok(Some(StreamChunk::Done)) => { got_done = true; break; }
            _ => {}
        }
    }

    assert!(got_text, "expected Text chunk within {:?}", timeout);
    assert!(got_done, "expected Done chunk within {:?}", timeout);
}
```

### 5.2 `tui_test_real_binary_replies.ts` (microsoft/tui-test)

Lives in `e2e/`. Spawns the real `fspec` binary and uses `tui-test`
to drive keyboard input and inspect the rendered buffer.

```typescript
import { test, expect } from '@microsoft/tui-test';
import { spawn } from 'node:child_process';

test('Work Agent replies to typed input', async ({ terminal }) => {
  // Spawn real binary
  terminal.spawn('./codelet/target/release/fspec');

  // Wait for BoardView render
  await terminal.expect('FSPEC').toBeOnScreen({ timeout: 5_000 });

  // Navigate to a DONE work unit, press Enter
  await terminal.press('Right'); // → done column
  await terminal.press('Enter');

  // Wait for AgentView
  await terminal.expect('Type a message').toBeOnScreen();

  // Type input
  await terminal.type('hello, are you there?');
  await terminal.press('Enter');

  // Assert user input appears
  await terminal.expect('user> hello, are you there?').toBeOnScreen();

  // Assert SOMETHING beyond user input appears within 30 seconds
  await terminal.expect('assistant>').toBeOnScreen({ timeout: 30_000 });

  // Assert NO Debug-dump pollution (the RPC-071 regression)
  await expect(terminal.getBuffer()).not.toContain('SessionStateChange {');
  await expect(terminal.getBuffer()).not.toContain('UserInput {');
});
```

---

## 6. Manual Smoke Test (Gating Release)

Procedure:

```bash
cd /Users/rquast/projects/fspec
cargo build -p codelet-fspec --release
./codelet/target/release/fspec
```

1. BoardView renders.
2. Navigate to a DONE work unit. Press Enter.
3. AgentView opens. Type "please review this card" + Enter.
4. Within 30 seconds, an `assistant> ...` line appears.
5. Scrollback contains NO `UserInput { ... }` or `SessionStateChange { ... }`
   raw Debug output (RPC-071 regression check).
6. Status pill returns to Idle after Done.

Pass criteria: all six steps observable.

---

## 7. Test Helper Crate / Module

`codelet/agent-loop/src/test_helpers.rs` (gated `#[cfg(any(test,
feature = "test-helpers"))]`):

```rust
pub fn stub_provider_registry() -> Arc<dyn ProviderRegistry> { ... }
pub fn slow_stub_provider_registry() -> Arc<dyn ProviderRegistry> { ... }
pub fn empty_tool_factory()    -> Arc<dyn ToolFactory> { ... }
pub fn build_test_service_with_stub_provider() -> Arc<FspecService<...>> { ... }
pub fn drain_until_done(rx: &mut Receiver<StreamChunk>, timeout: Duration) -> Vec<StreamChunk> { ... }
```

These helpers are also used by RPC-066's cross-frontend integration
test (which is currently `#[ignore]`'d pending RPC-069). After RPC-072
+ RPC-069, the cross-frontend test should be unblocked.

---

## 8. Coverage Links

```
fspec link-coverage rpc072-work-agent-roundtrip \
  --scenario "Stub provider returns a Text chunk for input 'hello'" \
  --testFile codelet/agent-loop/tests/stub_provider_round_trip.rs \
  --testLines <range> \
  --implFile codelet/agent-loop/src/agent_loop.rs \
  --implLines <range>

fspec link-coverage rpc072-work-agent-roundtrip \
  --scenario "End-to-end input via tarpc duplex produces Text reply" \
  --testFile codelet/fspec/tests/work_agent_end_to_end_rpc072.rs \
  --testLines <range> \
  --implFile codelet/fspec/src/common.rs \
  --implLines <range>
```

---

## 9. CI Gates

These tests MUST be added to the CI workflow:

- `cargo test -p codelet-agent-loop` (all unit + integration tests)
- `cargo test -p codelet-fspec --test work_agent_end_to_end_rpc072`
- `cargo test -p codelet-agent-loop --test no_napi_dependency`
- `cargo test --workspace` (no regressions)

The tui-test E2E (`e2e/tui_test_real_binary_replies.ts`) is gated
behind a longer-running CI job — it's slow because it spawns the real
binary.
