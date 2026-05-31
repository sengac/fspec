# RPC-070 — All affected call sites (audit table)

Comprehensive grep results so reviewers can see the full blast radius.

## Primary file: `codelet/sessions/src/handle_impl.rs`

```
$ grep -n 'block_on\|loop_block_on\|block_in_place' codelet/sessions/src/handle_impl.rs

 13: //! `tokio::runtime::Handle::current().block_on(...)`. They MUST be
 53: /// `tokio::runtime::Handle::current().block_on(...)`. The trait MUST be
 67:     /// `tokio::runtime::Handle::current().block_on(...)`. MUST be
 79:             .block_on(async { SessionManager::create_session(self, &model, &project).await })
603:     /// `tokio::runtime::Handle::current().block_on(...)`. MUST be
620:        let info = tokio::runtime::Handle::current().block_on(async {
877:        let custom_result = runtime.block_on(async move {
1251:        loop_block_on(async move {
1262:        let removed = loop_block_on(async move {
1275:        let entries = loop_block_on(async move {
1285: /// Centralising the `Handle::current().block_on(...)` call here keeps
1288: fn loop_block_on<F>(fut: F) -> F::Output
1293:     tokio::runtime::Handle::current().block_on(fut)
```

### Method-by-method breakdown

| Method | Line | Current bridge | Risk |
|--------|-----:|----------------|------|
| `create_session` | 78–80 | `Handle::current().block_on(…)` | **CRITICAL** — confirmed crash |
| `create_isolated_session` | 620–623 | `Handle::current().block_on(…)` | **HIGH** — same pattern, untested under tarpc |
| `test_provider_connection` | 875–879 | builds own `Runtime`, then `runtime.block_on` | **MEDIUM** — works on macOS today but allocates per call |
| `loop_add` | 1251 | via `loop_block_on(…)` | **HIGH** — same pattern as create_session |
| `loop_cancel` | 1262 | via `loop_block_on(…)` | **HIGH** |
| `loop_list` | 1275 | via `loop_block_on(…)` | **HIGH** |
| `loop_block_on` helper | 1288–1294 | `Handle::current().block_on(fut)` | **CRITICAL** — used by three methods |

## Confirmation that NAPI uses the safe pattern

```
$ grep -n 'block_on\|block_in_place' codelet/napi/src/session_bindings.rs
```

NAPI's `#[tokio_main]` macro runs the runtime on a separate thread from V8.
The V8 thread that calls into Rust via NAPI bindings is NOT a tokio worker —
`Handle::current().block_on()` from that thread enters the runtime fresh, so
no nested-driver panic. (Confirmed by the fact that this code has shipped to
TypeScript consumers for months without crashing.)

## Confirmation that the canonical pattern is `block_in_place`

```
$ grep -rn 'block_in_place' codelet/

codelet/tools/src/schedule/handler.rs:21:/// The handler is synchronous — async work uses block_in_place internally.
codelet/tools/src/pre_tool/handler.rs:…
```

These callers already live under the tarpc handler and don't panic — they use
the safe `tokio::task::block_in_place(|| Handle::current().block_on(…))` pattern.

## Confirmation that the tarpc handler is the nested-runtime context

```
$ grep -n 'create_session' codelet/rpc/src/lib.rs

740: impl FspecServiceImpl { … }
761:         async fn create_session(self, _: tarpc::context::Context, role: Option<String>) -> SessionId {
762:             self.inner.session_handle.create_session(role)
763:         }
```

Line 761 is the `async fn` whose `Self::Future` is what tarpc polls — the
problematic synchronous bridge at handle_impl.rs:79 runs INSIDE this future.

## No other consumers need patching

```
$ grep -rn 'fn create_session\b' codelet/

codelet/core/src/session_manager_handle.rs:75:    fn create_session(&self, role: Option<String>) -> SessionId;
codelet/sessions/src/handle_impl.rs:71:    fn create_session(&self, role: Option<String>) -> SessionId {
codelet/sessions/src/session_manager.rs:245:    pub async fn create_session(&self, model: &str, project: &str)
codelet/rpc/src/test_support.rs:…  (mock impls — keep sync)
codelet/rpc/src/lib.rs:761:        async fn create_session(self, …)
```

The trait is implemented in exactly one production place (handle_impl.rs) and
the stubs in `test_support.rs`. The stub impls don't bridge to async work, so
they need no change for Option B.

## Existing test gaps

```
$ rg '#\[ignore\]' codelet/rpc/tests/cross_frontend_parity.rs

20: #[ignore = "blocked on RPC-069 — stub provider not registered in ProviderManager"]
…
```

Four scenarios that would have exercised `create_session("stub/canned")` over
the live WS transport are `#[ignore]`'d pending RPC-069. None of these would
have caught the present bug because they fail at provider lookup BEFORE
reaching the panic.

Adding a "minimal" integration test (RPC-070 acceptance #5) that simply calls
`create_session` over the live transport — without exercising the provider —
will plug the hole permanently.

## Summary

- 6 production call sites + 1 helper.
- 1 file (`codelet/sessions/src/handle_impl.rs`).
- 0 trait surface changes for Option B.
- 1 new integration test to add.
- 1 e2e regression test to keep.

This is a tightly scoped fix with strong regression coverage proposed.
