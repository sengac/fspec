# DeepSearch "Failed to get tool definitions" on Qwen/Fireworks — Root Cause

**Test file:** `codelet/providers/tests/openai_fireworks_deepsearch_repro.rs`
**Run:** `cargo test -p codelet-providers --test openai_fireworks_deepsearch_repro -- --include-ignored --nocapture --test-threads=1`

## Result matrix (Qwen3.6-plus via real Fireworks inference API)

| # | Layer                                | Verdict |
|---|--------------------------------------|---------|
| 1 | Raw HTTPS → Fireworks                | ✅ 200 OK — body contains `"reasoning_content"` |
| 2 | rig `CompletionsClient`, no agent    | ✅ reply "Four." |
| 3 | `OpenAIProvider` agent, **no tools** | ✅ reply "pong" |
| 4 | Agent + 1 tool, `.prompt().multi_turn()` | ❌ `panic: "The OpenAI Completions API doesn't support reasoning!"` |
| 5 | Agent + 7 tools, `.prompt().multi_turn()` | ❌ same panic |
| 6 | Agent + 7 tools, `.stream_prompt().multi_turn()` | ✅ completes, 3 tool calls made |
| 7 | Unit test: outbound wire conversion with reasoning in history | ✅ reproduces panic with zero network |

## The actual bug

Fireworks' Qwen3 models always return a `reasoning_content` field in every
`chat.completion` response (confirmed in layer 1). The patched rig-core
OpenAI Completions adapter handles this as follows:

1. **Inbound parse** (`providers/openai/completion/mod.rs:903`, PROV-081):
   captures `reasoning_content` into an `AssistantContent::Reasoning` and
   pushes it into the first-turn response choice **ahead of** text and
   tool-call entries.

2. **Chat-history accumulation** (non-streaming `PromptRequest::send`):
   persists the full assistant choice — reasoning included — into the
   rolling chat history.

3. **Outbound serialization on turn 2** (`providers/openai/completion/mod.rs:665`):
   `TryFrom<OneOrMany<message::AssistantContent>> for Vec<Message>` contains:

   ```rust
   message::AssistantContent::Reasoning(_) => {
       panic!("The OpenAI Completions API doesn't support reasoning!");
   }
   ```

   This panic is labelled in the surrounding comment as the "last line of
   defense" (PROV-081) — it fires **every time** the non-streaming
   multi-turn loop tries to re-send chat history containing reasoning,
   which is unavoidable for Qwen3/Fireworks because steps 1 and 2 always
   plumb reasoning into history for that provider.

## Why the streaming path also "fails" in the TUI (derived, not proven here)

The streaming path in `prompt_request/streaming.rs:711` only appends
`AssistantContent::Reasoning` to chat history when `signature.is_some()`.
Fireworks does not attach reasoning signatures, so layer 6 sails past —
reasoning is rendered on screen but **silently dropped** from the history
sent on subsequent turns. That is why layer 6 passes.

The TUI error `Streaming error: RequestError: Failed to get tool definitions`
surfaces when the streamed generator future is unwound mid-flight — the
panic in the non-streaming path OR a canceled oneshot (the tool-server
reply channel) both map to the same swallow-everything line at
`patches/rig-core/src/agent/completion.rs:163` / `:177`:

```rust
.map_err(|_| CompletionError::RequestError("Failed to get tool definitions".into()))?;
```

That `map_err(|_| …)` is **throwing away the real error** (`Canceled`,
`ToolsetError::ToolCallError`, `InvalidMessage(ToolError { error })`, etc.)
and replacing it with the generic "Failed to get tool definitions" string
that has been showing up in the TUI. The qwen agent spent hours chasing
this symptom; the fix is to preserve the underlying error so it can't hide
the real one again.

## Recommended fixes (out of scope for this diagnosis — noted for backlog)

1. **PROV-081 (outbound stripping)**: change the non-streaming chat-history
   accumulator (or the `TryFrom` itself) to drop `AssistantContent::Reasoning`
   instead of panicking. Reasoning is already correctly flagged as
   INBOUND-only; the panic should become a silent filter.
2. **Error preservation**: replace the two `map_err(|_| …)` sites in
   `agent/completion.rs` (around lines 163 and 177) with a version that
   carries the original `ToolServerError` payload, so future users/agents
   see the actual cause.
3. **Streaming symmetry**: confirm the streaming reasoning-without-signature
   path is by-design (reasoning shown but not persisted) or align behavior
   with non-streaming.

## How to reproduce the panic without any network

```
cargo test -p codelet-providers --test openai_fireworks_deepsearch_repro \
  layer7 -- --nocapture
```
