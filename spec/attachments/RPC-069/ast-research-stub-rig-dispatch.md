# AST Research — RPC-069 Stub Rig Dispatch

Goal: verify against HEAD that the four anchor sites listed in the implementation guide are exactly where the guide says they are, and that no new arms have been added between specifying and testing.

## Anchor sites confirmed via AstGrep

| Site | File:Line | Pattern matched |
|---|---|---|
| `impl StubProvider` block (need to extend with `create_rig_agent`) | `codelet/providers/src/stub_provider.rs:34` | `impl StubProvider { $$$BODY }` |
| Reference `impl CompletionModel` for the Rhai backend | `codelet/providers/src/custom/rig_model.rs:231` | `impl CompletionModel for $T { $$$BODY }` |
| Provider dispatch `match` block (insert `"stub"` arm here) | `codelet/agent-loop/src/agent_loop.rs:880` | `match current_provider.as_str() { $$$ARMS }` |
| Predicate `agent_loop_dispatch_supports_provider` (add `"stub"`) | `codelet/agent-loop/src/dispatch.rs:119` | `pub fn agent_loop_dispatch_supports_provider($$$ARGS) -> bool { $$$BODY }` |

All four anchors match the implementation guide. No drift detected between specifying and testing.

## CompletionModel adapter — reference pattern

The Rhai custom-provider model at `codelet/providers/src/custom/rig_model.rs:231` is the canonical reference for implementing `impl CompletionModel for X` in this codebase. Required associated types:

- `type Response: WasmCompatSend + WasmCompatSync + Serialize + DeserializeOwned`
- `type StreamingResponse: Clone + Unpin + WasmCompatSend + WasmCompatSync + Serialize + DeserializeOwned + GetTokenUsage`
- `type Client` (we use `()`)

Required methods:

- `fn make(_client, _model) -> Self` — may panic; the stub model is constructed via `StubProvider::create_rig_agent`, not the trait factory
- `async fn completion(&self, _request) -> Result<RigCompletionResponse<Self::Response>, CompletionError>` — return `OneOrMany::one(AssistantContent::text("hi back"))` with a default `Usage`
- `async fn stream(&self, _request) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>` — yield `RawStreamingChoice::Message("hi back")` then `RawStreamingChoice::FinalResponse(StubCompletion::default())`

## Agent builder pattern (RhaiCustomProvider parallel)

Per `codelet/providers/src/custom/custom_provider.rs:193`, the agent builder pattern for a custom (non-rig-client) model is:

```rust
let model = StubModel::new();
let mut builder = rig::agent::AgentBuilder::new(model);
if let Some(p) = preamble { builder = builder.preamble(p); }
builder.build()  // → rig::agent::Agent<StubModel>
```

No `rig_client.agent(...)` call (the stub has no HTTP client).

## Stub registration call site (already wired)

Per implementation guide, the stub registration call site lives at `codelet/fspec/src/common.rs:122-126` under `#[cfg(feature = "test-stub-provider")]`. The agent-loop arm we add must call `codelet_providers::stub_provider::is_stub_registered(&current_provider)` for defence-in-depth and then construct a `StubProvider::new()` (ZST, identical to the registered instance) to call `create_rig_agent` on.

## Feature gate verification

- `codelet/agent-loop/Cargo.toml:50-56`: `test-support = ["codelet-providers/test-support"]` ✓
- `codelet/fspec/Cargo.toml:105-109`: `test-stub-provider = ["dep:codelet-providers", "codelet-providers/test-support", "codelet-agent-loop/test-support"]` ✓

## Conclusion

All anchors verified at HEAD. Implementation can proceed with high confidence. No new arms or predicates have drifted since the implementation guide was authored.
