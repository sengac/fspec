# PROV-140 — Wire per-profile streaming flag end-to-end (stream=false non-streaming path)

> **Depends on:** PROV-139 (which adds `streaming` to the profile schema, the
> /provider form, and persistence). This card *consumes* that flag and makes it
> actually change LLM request behaviour.

## 1. Goal

When a profile has **Streaming disabled**, the OpenAI provider must issue a
**non-streaming** chat completion request (`stream: false`, no `stream_options`)
instead of opening an SSE stream — and the agent loop must synthesize the same
`StreamChunk` output from the single JSON response so the TUI / persistence
pipeline is unchanged and the user still sees the assistant reply (delivered as
one block rather than token-by-token).

When streaming is **enabled** (the default), behaviour is byte-for-byte
identical to today.

## 2. Background — the streaming mechanism today

### 2.1 The hard-coded `stream: true` switch

The project uses a **vendored, patched copy of rig-core** at
`codelet/patches/rig-core/`. The `stream: true` injection is hard-coded in
`codelet/patches/rig-core/src/providers/openai/completion/streaming.rs`
(~line 125):

```rust
request_as_json = merge(
    request_as_json,
    json!({"stream": true, "stream_options": {"include_usage": true}}),
);
```

`CompletionModel::stream()` always injects this; there is no caller-facing
toggle. A **non-streaming** method already exists in the same crate:
`CompletionModel::completion()` (`completion/mod.rs` ~line 1331), which POSTs
without the `stream` merge and parses a single response.

### 2.2 Our provider & driver

- `codelet/providers/src/openai.rs` — `OpenAIProvider` wraps the rig client.
  - `from_api_key_with_options(...)` (~line 185) is the central constructor;
    reads `OPENAI_BASE_URL`, `OPENAI_CONTEXT_WINDOW`,
    `OPENAI_MAX_OUTPUT_TOKENS` from **environment variables**.
  - `create_rig_agent(...)` (~line 410) builds the streaming rig `Agent`.
  - `complete_with_tools(...)` (~line 532) is the **existing non-streaming**
    single-shot path (`CompletionRequestBuilder::send()`).
  - `supports_streaming()` (~line 520) currently returns hard-coded `true`.
- `codelet/core/src/rig_agent.rs` — `RigAgent::prompt_streaming*`
  (~lines 89–178) all call `.stream_prompt(prompt).multi_turn(max_depth)`.
- The interactive loop always routes through
  `codelet_cli::interactive::run_agent_stream_with_images(...)`, called from
  `codelet/agent-loop/src/agent_loop.rs`.

### 2.3 The critical finding

`supports_streaming()` is defined by **every** provider, but a grep of call
sites shows it is currently referenced **only in tests/examples** — **no
production code branches on it** to choose a streaming vs non-streaming path.
So the capability flag exists but is inert. This card makes it live.

## 3. How the flag reaches the provider

Profiles do NOT pass config directly into `OpenAIProvider`. Instead,
`set_model_direct_with_profile(...)` (`codelet/providers/src/manager.rs`
~line 573) records selection state and the caller sets
`OPENAI_API_KEY` / `OPENAI_BASE_URL` as **environment variables**;
`get_openai(session_id)` (~line 815) then reads them and constructs the
provider, which reads further options from env inside `openai.rs`.

**Two viable plumbing options** (decide during Example Mapping — capture as a
resolved question):

- **(A) Env-var bridge (lowest friction, matches existing pattern):** add
  `OPENAI_DISABLE_STREAMING` (or `OPENAI_STREAMING`), set it from the resolved
  profile at the same place the other `OPENAI_*` vars are set, and read it in
  `from_api_key_with_options`.
- **(B) Explicit field (cleaner, more work):** thread a `streaming: bool`
  parameter through `set_model_direct_with_profile` → provider construction.

Whichever is chosen, the provider gains a `streaming_enabled: bool` field and
`supports_streaming()` returns it.

> **Note:** the profile→env resolution currently lives on the TypeScript/NAPI
> side for the Ink TUI. For the Rust `/provider` path, confirm where the
> resolved profile is applied for a session created from the Rust TUI, and set
> the flag there. If the resolution is shared, prefer the shared site.

## 4. Making "disabled" actually take effect (the hard part)

Flipping the flag alone does nothing, because the agent loop unconditionally
calls `.stream_prompt()`. A real non-streaming path is required. Recommended
approach:

1. **Branch at the dispatch layer** (`rig_agent.rs` / agent-loop), not by
   hacking only the rig-core merge. When `supports_streaming()` is false for
   the active provider, drive the turn through a **non-streaming** call
   (`completion()` / `complete_with_tools()` semantics) instead of
   `stream_prompt()`.
2. **Synthesize chunks** from the single response so the downstream pipeline
   (`BackgroundOutput`, `StreamChunk`, TUI scrollback, persistence) is
   unchanged: emit the assistant text as `StreamChunk::Text` (one or a few
   chunks), preserve tool-call handling across the multi-turn loop, then
   `StreamChunk::Done` with usage mapped from the response.
3. **Preserve the multi-turn tool loop.** Non-streaming must still support
   tool calls (`finish_reason == "tool_calls"` → execute tools → send tool
   results → continue) to reach `max_depth`, mirroring the streaming loop's
   `multi_turn` behaviour. This is the main complexity — do not regress tool
   calling.
4. **Optionally** also guard the rig-core `stream: true` merge behind a flag
   for correctness, but the decisive routing belongs at the agent-loop level
   because the whole pipeline assumes chunked output.

## 5. Acceptance shape (finalize during Example Mapping)

- With a profile where `streaming = Some(false)`, a session created against it
  reports `supports_streaming() == false`.
- With streaming disabled, the outgoing OpenAI request body has
  `stream: false` (or omits `stream`) and omits `stream_options`.
- With streaming disabled, a plain assistant reply is still delivered to the
  TUI (as synthesized `StreamChunk::Text` + `Done`) and persisted identically
  to the streaming case (modulo token-by-token granularity).
- With streaming disabled, a **tool-calling** turn still executes tools and
  completes the multi-turn loop.
- With streaming enabled (default / `None`), behaviour is unchanged — the SSE
  path is taken and `stream: true` + `stream_options.include_usage` are sent.
- Behaviour verified across BOTH transports (embedded + WebSocket) — mirror the
  RPC-009 cross-transport parity test pattern.

## 6. Test strategy

- Unit: `supports_streaming()` reflects the profile flag through the chosen
  plumbing (env var or field).
- Unit/contract: the non-streaming request JSON has `stream: false` and no
  `stream_options` (assert on the serialized body — can mirror the existing
  rig keystone test `providers/tests/rhai_rig_agent_keystone_tests.rs` which
  already inspects `model.stream(request)`).
- Integration: non-streaming turn produces the same terminal `StreamChunk`
  sequence shape (Text… then Done) as streaming for a text-only reply.
- Integration: non-streaming turn with a tool call runs the tool and continues.
- Cross-transport parity test (embedded + WS).
- Regression: streaming-enabled path unchanged (existing streaming tests still
  pass).

## 7. Invariants to preserve

1. **No regression for the default (streaming-on) path.**
2. **Downstream pipeline unchanged** — synthesize `StreamChunk`s rather than
   teaching the TUI about non-streaming responses.
3. **Tool-calling multi-turn preserved** in the non-streaming path.
4. **Host-supplied tokio runtime** — `tokio::spawn` only; never
   `Runtime::new`/`Builder`.
5. **File-size discipline** — touched/new module files under 300 LoC; extract
   the non-streaming loop into its own module rather than bloating
   `agent_loop.rs` / `rig_agent.rs`.
6. **`codelet-napi` is not a dep** of the non-NAPI crates involved.
7. **Cross-transport parity** — every new behaviour tested against both
   backends.

## 8. Risks

- **Tool-call parity in the non-streaming loop** is the primary risk: the
  streaming path's `multi_turn` handling must be faithfully reproduced.
- **Vendored rig-core**: if the non-streaming path must go through rig, changes
  land in `codelet/patches/rig-core/` and must be reflected in
  `codelet/patches/rig-core.patch`. Prefer routing at the provider/agent-loop
  layer using the existing `complete_with_tools` to minimize patch churn.
- **Where profile→provider resolution happens for Rust-TUI-created sessions**
  must be confirmed so the flag is applied on the correct path.

## 9. Key files index

| Concern | Location |
|---|---|
| Provider struct + constructors | `codelet/providers/src/openai.rs` |
| `supports_streaming()` | `codelet/providers/src/openai.rs` (~line 520) |
| Non-streaming single-shot | `codelet/providers/src/openai.rs` `complete_with_tools` (~532) |
| Profile→provider (env bridge) | `codelet/providers/src/manager.rs` `set_model_direct_with_profile` (~573), `get_openai` (~815) |
| Streaming driver | `codelet/core/src/rig_agent.rs` `prompt_streaming*` (~89–178) |
| Agent loop | `codelet/agent-loop/src/agent_loop.rs` |
| Hard-coded `stream:true` | `codelet/patches/rig-core/src/providers/openai/completion/streaming.rs` (~125) |
| Non-streaming rig method | `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` `completion` (~1331) |
| rig keystone stream test | `codelet/providers/tests/rhai_rig_agent_keystone_tests.rs` |
