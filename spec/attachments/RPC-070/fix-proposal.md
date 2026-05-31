# RPC-070 — Fix proposal

Three options, recommendation, and the proposed patch.

---

## Option A — Make the trait async (cleanest, biggest blast radius)

Change `codelet/core/src/session_manager_handle.rs` so the six offending methods
become `async fn`. Then `codelet/rpc/src/lib.rs:761` simply `.await`s them inside
the already-async tarpc handler.

```rust
// codelet/core/src/session_manager_handle.rs
#[async_trait::async_trait]
pub trait SessionManagerHandle: Send + Sync {
    async fn create_session(&self, role: Option<String>) -> SessionId;
    async fn create_isolated_session(&self, …) -> IsolatedSessionInfo;
    async fn test_provider_connection(&self, …) -> TestConnectionResult;
    async fn loop_add(&self, …) -> RegisteredLoop;
    async fn loop_cancel(&self, …) -> bool;
    async fn loop_list(&self) -> Vec<RegisteredLoop>;
    // …rest of trait unchanged
}
```

**Pros**
- Removes the entire class of bugs at the type level.
- No runtime cost.
- Matches tarpc's natural model.

**Cons**
- Requires `async_trait` (or Rust 1.75+ AFIT — already on stable, but `dyn` requires `async_trait`).
- Ripples into:
  - `codelet/sessions/src/handle_impl.rs` (the prod impl)
  - `codelet/napi/src/session_bindings.rs` (the NAPI shim is sync — has to wrap into spawn_blocking or its own block_on)
  - `codelet/rpc/src/test_support.rs` and every test stub
  - `codelet/fspec-tui/...` consumers that call the trait synchronously
- Higher review/test burden.

---

## Option B — Use `tokio::task::block_in_place` (RECOMMENDED)

Wrap each offending bridge in `block_in_place` so the worker thread is temporarily
removed from the multi-thread scheduler before `block_on` is called. After the
inner future completes, the thread rejoins the scheduler.

```rust
// codelet/sessions/src/handle_impl.rs

fn create_session(&self, role: Option<String>) -> SessionId {
    let project = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let model = self
        .get_default_model()
        .unwrap_or_else(|| "anthropic/claude-opus-4-5".to_string());

    let id_string = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async {
                SessionManager::create_session(self, &model, &project).await
            })
    })
    .unwrap_or_default();

    if let Some(role_str) = role {
        if !role_str.is_empty() {
            if let Ok(session) = self.get_session(&id_string) {
                session.set_role(role_str);
            }
        }
    }
    SessionId::new(id_string)
}
```

Apply the same `block_in_place` wrapper to the other five offenders, then update
the `loop_block_on` helper:

```rust
fn loop_block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(fut)
    })
}
```

For `test_provider_connection` (line 877), which currently builds its own
`Runtime::new()`, simplify to the same pattern:

```rust
let custom_result = tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async move {
        // …existing body…
    })
});
```

**Pros**
- Minimal change — six call sites + one helper.
- No trait surface changes; NAPI shim, stub, mocks, tests all untouched.
- Already the canonical pattern used by `codelet/tools/src/schedule/handler.rs` (see line 21 doc-comment).

**Cons**
- Only legal on a multi-thread runtime. The `fspec` binary and NAPI both use
  multi-thread, and we should add a debug-assertion in the helper:
  ```rust
  debug_assert_eq!(
      tokio::runtime::Handle::current().runtime_flavor(),
      tokio::runtime::RuntimeFlavor::MultiThread,
      "SessionManagerHandle bridge requires a multi-thread runtime",
  );
  ```
- A single-thread runtime caller would panic in `block_in_place` itself — same
  failure mode as today, with a clearer message.

---

## Option C — Build a fresh runtime per call (rejected)

Currently `test_provider_connection` does this. It avoids the panic but:

- Allocates a brand-new multi-thread pool on every RPC call.
- Latency: ~10–20ms per session creation on macOS.
- Resource leak risk if the runtime drop is interrupted.

Do not adopt as the general fix. Convert the existing site (line 877) to Option B.

---

## Recommendation

**Adopt Option B now (RPC-070).** Optionally schedule Option A as RPC-071 if/when
we want a clean trait surface — that work is significant but mechanical.

---

## Acceptance criteria for the patch

1. All six call sites use `tokio::task::block_in_place(|| Handle::current().block_on(…))`.
2. `loop_block_on` helper updated to the same pattern with a `debug_assert!` on
   the runtime flavor.
3. `test_provider_connection` stops constructing its own `Runtime`.
4. Doc-comments at `handle_impl.rs:11–18` and `:51–58` rewritten to describe the
   correct contract (multi-thread runtime, no nested-driver requirement).
5. New integration test in `codelet/rpc/tests/` that:
   - Starts a real tarpc server with a `SessionManager` handle.
   - Connects a tarpc client over an in-memory `duplex`.
   - Calls `create_session` on the client.
   - Asserts no panic and a valid `SessionId` returned.
   This test would have failed today and would have caught the bug pre-RPC-068.
6. `e2e/rpc-068-work-agent-panic-repro.test.ts` updated to assert
   `not.toContain('panicked')` and `toContain('Agent')` instead of asserting the
   panic exists. Keep the test as a permanent regression guard.
7. `cargo test --workspace` and the no_napi_dependency boundary test still pass.

---

## Estimated complexity

- Source change: ~30 LOC across one file.
- Test change: ~80 LOC for the new integration test + ~10 LOC in the e2e.
- Doc change: ~20 LOC.

Suggested estimate: **3 story points** (moderate — well-scoped, but requires a
new tarpc-over-duplex integration harness).
