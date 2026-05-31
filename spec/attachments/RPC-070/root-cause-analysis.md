# RPC-070 — Root-cause analysis: `Handle::current().block_on()` from inside tarpc handler

**Discovered:** 2026-05-26 during follow-up to RPC-068.
**Repro:** `e2e/rpc-068-work-agent-panic-repro.test.ts` (passes a real PTY through the compiled Rust binary).
**Captured backtrace:** `spec/attachments/RPC-070/panic-backtrace-captured.txt`.

---

## Symptom

When the user navigates to the DONE column in the Rust TUI and presses `Enter` on a work
unit, the Work Agent fails to open and the rendered buffer contains:

```
'tokio-rt-worker' (7944972) panicked at sessions/src/handle_impl.rs:79:14:
Cannot start a runtime from within a runtime. This happens because a function
(like `block_on`) attempted to block the current thread while the thread is
being used to drive asynchronous tasks.
```

The board itself renders fine (verified by `e2e/rpc-068-rust-binary-smoke.test.ts`).
The crash is isolated to the `create_session` dispatch path.

## Call stack (abridged)

From `spec/attachments/RPC-070/panic-backtrace-captured.txt`:

```
 2: tokio::runtime::context::runtime::enter_runtime
 3: tokio::runtime::handle::Handle::block_on_inner
 4: tokio::runtime::handle::Handle::block_on
 5: codelet_sessions::handle_impl::<impl ...SessionManagerHandle for SessionManager>::create_session
       at codelet/sessions/src/handle_impl.rs:79:14
 6: <codelet_rpc::FspecServiceImpl as codelet_rpc::FspecService>::create_session::{{closure}}
       at codelet/rpc/src/lib.rs:761:36
 7: <codelet_rpc::ServeFspecService<S> as tarpc::server::Serve>::serve::{{closure}}
       at codelet/rpc/src/lib.rs:55:1
 8: tarpc::server::InFlightRequest<Req,Res>::execute::{{closure}}::{{closure}}
       at tarpc-0.34.0/src/server.rs:1028:61
```

Frame 6 is the tarpc-generated handler — an `async fn` that is already being polled by
the tokio multi-thread scheduler. Frame 5 then calls `Handle::current().block_on(…)`
which tries to enter the same runtime that is currently driving frame 6, which tokio
detects and converts into the documented `enter_runtime` panic.

## The faulty contract in `handle_impl.rs`

```rust
// codelet/sessions/src/handle_impl.rs:11–18 (doc comment)
//! **Runtime requirement:** the sync→async bridges for `create_session`
//! and `create_isolated_session` use
//! `tokio::runtime::Handle::current().block_on(...)`. They MUST be
//! invoked from a thread that has an active tokio runtime — calling
//! from a non-runtime thread will panic with the standard
//! `Handle::current()` panic message. The `fspec` binary always has
//! a runtime; the napi side already uses `tokio_main` so its hooks
//! satisfy this too.
```

The comment conflates two distinct properties:

| Property | Means |
|----------|-------|
| Thread has a runtime handle | `Handle::current()` returns `Ok` (does not panic) |
| Thread isn't currently being driven by that runtime | `Handle::block_on()` is legal |

`block_on` requires the **second** property; the comment only guarantees the **first**.
The tarpc dispatcher always violates the second condition because tarpc IS the async
runtime polling the handler.

## All affected call sites

`grep -n 'block_on\|loop_block_on' codelet/sessions/src/handle_impl.rs`:

| Line | Method | Pattern |
|-----:|--------|---------|
| 78–80 | `create_session` | `Handle::current().block_on(…)` |
| 620–623 | `create_isolated_session` | `Handle::current().block_on(…)` |
| 877–879 | `test_provider_connection` | spins a fresh `Runtime::new()` then `runtime.block_on(…)` — also panics under nested runtime |
| 1251 | `loop_add` | via `loop_block_on(…)` |
| 1262 | `loop_cancel` | via `loop_block_on(…)` |
| 1275 | `loop_list` | via `loop_block_on(…)` |
| 1288–1294 | `loop_block_on` helper | `Handle::current().block_on(fut)` |

`test_provider_connection` at line 877 is subtly different — it builds a brand-new
`tokio::runtime::Runtime` and calls `block_on` on **that**. That can succeed on a
worker thread (the new runtime is unrelated to the surrounding one), but it allocates
a runtime per call and on macOS triggers nested `enter_runtime` guard panics in some
tokio versions. We treat it as broken for consistency.

## Why every existing test was green

`cargo test --workspace -p codelet-sessions` and the RPC-068 boundary audit both
shell into `#[tokio::test]`, which:

1. Creates a fresh single-thread runtime for the test.
2. Calls the test body as the **outer** future on that runtime.
3. The test body THEN invokes `handle.create_session(...)` synchronously.

Inside step 3 the test body is not itself currently being polled (it was the entry
future), so `Handle::current().block_on(...)` re-enters the test's own runtime and
returns. No panic.

The live tarpc dispatcher is the opposite: the handler IS the currently-polled
future, so re-entering the runtime panics.

The pre-existing `cross_frontend_parity` integration tests would have caught this,
but four of them are `#[ignore]`'d pending RPC-069 (the only ones that exercise
`create_session("stub/canned")` over the WS transport).

## How NAPI avoids the same trap

`codelet/napi/src/session_bindings.rs` calls `SessionManager::create_session` from
ThreadsafeFunction callbacks. NAPI's `#[tokio_main]` macro spawns the runtime on a
**separate thread** from the V8 thread that invokes the JS callback. The block_on
call happens on the V8 thread (which has the handle but is NOT being driven by it),
satisfying both properties.

`codelet/tools/src/schedule/handler.rs:21` documents the canonical workaround for
synchronous handler traits:

> The handler is synchronous — async work uses `block_in_place` internally.

`tokio::task::block_in_place` tells the multi-thread scheduler to detach the
current worker thread from the runtime for the duration of the closure, after which
`Handle::current().block_on(...)` becomes legal on that thread again. This is
exactly what `handle_impl.rs` needs.

## Why this matters for RPC-070

Without this fix the Rust binary is unusable: every user-initiated session creation
crashes the worker, which tarpc handles by closing the connection — and on the
client side the TUI freezes waiting for a `SessionId` that will never arrive.
All slash commands that depend on `/new`, `/resume`, or background sessions are
blocked by this single bug.
