# AST Research: rig::completion::CompletionModel and rig::tool::Tool integration surface

## Goal
Identify the exact rig contract our `RhaiCustomProviderModel` must implement
and the rig `Tool` trait surface that `RhaiToolWrapper` must satisfy so the
agent built by `CustomProvider::create_rig_agent` is a real
`rig::agent::Agent<RhaiCustomProviderModel>` driven by rig's normal
multi-step prompt loop.

## Findings

### rig::completion::CompletionModel
- File: `codelet/patches/rig-core/src/completion/request.rs:384`
- Trait shape:
  - `type Response`
  - `type StreamingResponse: GetTokenUsage`
  - `type Client`
  - `fn make(client, model_name) -> Self`
  - `async fn completion(req: CompletionRequest) -> Result<CompletionResponse<Response>, CompletionError>`
  - `async fn stream(req: CompletionRequest) -> Result<StreamingCompletionResponse<StreamingResponse>, CompletionError>`
- `RhaiCustomProviderModel::Client = ()` and `make()` is intentionally
  `unimplemented!` because the model carries a fully wired Rhai handle that
  cannot be reconstructed from a name.

### rig::tool::Tool
- File: `codelet/patches/rig-core/src/tool/mod.rs:106`
- Trait shape requires `const NAME: &'static str` plus `name()`,
  `definition()`, `call()` methods.
- We pick a sentinel for `NAME` and override `name()` so the dynamic Rhai
  tool name surfaces to the LLM. This pattern matches what the existing
  `FacadeToolWrapper` family (e.g. `codelet/tools/src/facade/wrapper.rs:59`)
  does.

### rig::agent::AgentBuilder
- File: `codelet/patches/rig-core/src/agent/builder.rs:42`
- `AgentBuilder::tool(self, tool)` consumes self and returns
  `AgentBuilderSimple<M>` which has its own `.tool()` chain. We mirror the
  existing claude/openai/codex pattern.

### Streaming bridge
- File: `codelet/patches/rig-core/src/streaming.rs:73` defines
  `RawStreamingChoice<R>` variants `Message`, `ReasoningDelta`,
  `ToolCallDelta`, `ToolCall`, `Usage`, `FinalResponse`.
- We map `super::stream::StreamChunk` → `RawStreamingChoice` 1-to-1, with
  `StopReason` collected and surfaced as the `FinalResponse` payload.

### Existing rig integration patterns reused
- `codelet/providers/src/claude.rs:507` — claude `create_rig_agent`
- `codelet/providers/src/codex/mod.rs:331` — codex `create_rig_agent`
- `codelet/providers/src/copilot/rig_agent.rs:56` — copilot
- `codelet/providers/src/adapter.rs:135` — `convert_assistant_content`
  helper used by all providers when bridging rig responses back to our
  `ContentPart` format.

## Decisions
- Skip rig's `CompletionClient` — we don't need it because we call
  `AgentBuilder::new(model)` directly with our pre-built model.
- `RhaiCustomProviderModel::stream()` posts via a new `post_sse` helper in
  `codelet/providers/src/custom/http.rs` that auto-injects
  `Accept: text/event-stream` and returns the body as a Bytes stream.
- `internal_dispatch.rs` runs the inner internal tool implementations
  (`ReadTool`, `WriteTool`, etc.) directly — facade-wrapper concerns like
  worktree validation and BLOCK-006 stage permissions are tracked as
  follow-up work but not blockers for the keystone Rhai integration.
