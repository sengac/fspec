# RPC-072 — AST Research: Wiring Points

> Concrete AST locations of the symbols that RPC-072 must modify or read.
> Recorded with the AstGrep tool against the working-tree on 2026-05-27.

## 1. The broken hook — `NoopSessionManagerHooks::spawn_agent_loop`

- File: `codelet/sessions/src/session_manager.rs`
- Line:  118
- Pattern matched: `fn spawn_agent_loop(&self, $$$ARGS) { $$$BODY }`
- Match: `fn spawn_agent_loop(` (no-op body — drops `_input_rx`)

This is the function whose body must be replaced (in a NEW hooks impl, not
this one) so the session's input channel is actually drained.

## 2. The entry point that installs hooks — `build_service`

- File: `codelet/fspec/src/common.rs`
- Line:  80
- Signature: `pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>>`

This is the chokepoint where RPC-072 must swap `FspecSessionManagerHooks`
for `FspecAgentHooks` (the new impl) before `SharedFspecService::with_session_manager`
is constructed.

## 3. The chunk emission contract — `BackgroundSession::send_input`

- File: `codelet/sessions/src/background_session.rs`
- Line:  1083
- Signature: `pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String>`

This is the function that already emits UserInput + Running chunks and
calls `input_tx.try_send(PromptInput { ... })`. After RPC-072 the `try_send`
succeeds (because the new hook DRAINS input_rx instead of dropping it),
so the agent_loop receives the prompt and emits assistant chunks.

## 4. The existing no-op fspec hooks — `FspecSessionManagerHooks::spawn_agent_loop`

- File: `codelet/fspec/src/session_hooks.rs`
- Line:  29
- Signature: `fn spawn_agent_loop(&self, _session, _input_rx, _mcp_injection_rx)`

The currently-installed hooks impl. Its `spawn_agent_loop` is a no-op
that drops `_input_rx`. After RPC-072 this impl is replaced by
`codelet_agent_loop::FspecAgentHooks` (option (a) per architecture.md §2),
so the boundary is explicit and the impl can be tested independently.

## 5. The provider plumbing already present on `BackgroundSession.inner`

`create_session_with_id` constructs a `ProviderManager` and stores it in
`session.inner` (a `codelet_cli::session::Session`). This means the
agent_loop does NOT need to consult `~/.fspec/config.json` directly —
it can read the provider/model already selected on the session by
locking `session.inner`.

This simplifies the scope: no separate `ProviderRegistry` trait is
needed for the minimum-viable implementation; the loop reaches into
`session.inner.provider_manager_mut().get_<provider>()` and calls
`complete_with_tools(messages, &[])`.

## 6. The deterministic stub provider

- File: `codelet/providers/src/stub_provider.rs`
- `impl LlmProvider for StubProvider` returns
  `CompletionResponse { content: MessageContent::Text("hi back"), stop_reason: EndTurn }`
- Registration is idempotent via `register_stub_provider()` (called by
  `build_service` under `#[cfg(feature = "test-stub-provider")]`).

The minimum acceptance test (`scenario_send_input_hello_yields_canned_stream`)
already exists in `codelet/fspec/tests/cross_frontend_parity.rs` as
`#[ignore]`'d — it un-ignores naturally once RPC-072 lands.
