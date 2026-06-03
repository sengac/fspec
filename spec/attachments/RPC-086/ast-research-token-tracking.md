# RPC-086 AST Research: Token Tracking Call Sites

Generated 2026-06-01 in support of RPC-086 structural ACDD coverage.

## Source-of-truth files

| Concern | File | Line range |
|---|---|---|
| `StreamEvent::Tokens` translation arm | `codelet/agent-loop/src/background_output.rs` | 226–243 |
| `StreamEvent::ContextFill` translation arm | `codelet/agent-loop/src/background_output.rs` | 244–249 |
| `BackgroundSession::update_tokens` | `codelet/sessions/src/background_session.rs` | 701–704 |
| `BackgroundSession::update_reasoning_tokens` | `codelet/sessions/src/background_session.rs` | 707–709 |
| `BackgroundSession::get_tokens` | `codelet/sessions/src/background_session.rs` | 712–717 |
| `cached_input_tokens` / `cached_output_tokens` / `cached_reasoning_tokens` fields | `codelet/sessions/src/background_session.rs` | 282–284, 470–472 |
| `StreamChunk::TokenUpdate` variant | `codelet/rpc-types/src/lib.rs` | 1056 |
| `StreamChunk::ContextFillUpdate` variant | `codelet/rpc-types/src/lib.rs` | 1059 |
| `StreamChunk::token_update` ctor | `codelet/rpc-types/src/lib.rs` | 1176–1178 |
| `StreamChunk::context_fill_update` ctor | `codelet/rpc-types/src/lib.rs` | 1181–1183 |
| `TokenInfo` source enum | `codelet/cli/src/interactive/output.rs` | 21–31 |
| `ContextFillInfo` source enum | `codelet/cli/src/interactive/output.rs` | 90–99 |
| `StreamOutput::emit_tokens` driver | `codelet/cli/src/interactive/output.rs` | 268 |
| `StreamOutput::emit_context_fill` driver | `codelet/cli/src/interactive/output.rs` | 273–275 |

## Canonical body — `StreamEvent::Tokens` arm

```rust
StreamEvent::Tokens(info) => {
    // Update cached tokens for sync access
    self.session
        .update_tokens(info.input_tokens as u32, info.output_tokens as u32);
    if let Some(r) = info.reasoning_tokens {
        self.session.update_reasoning_tokens(r as u32);
    }
    StreamChunk::token_update(TokenTracker {
        input_tokens: info.input_tokens as u32,
        output_tokens: info.output_tokens as u32,
        cache_read_input_tokens: info.cache_read_input_tokens.map(|v| v as u32),
        cache_creation_input_tokens: info.cache_creation_input_tokens.map(|v| v as u32),
        tokens_per_second: info.tokens_per_second,
        cumulative_billed_input: None,
        cumulative_billed_output: None,
        reasoning_tokens: info.reasoning_tokens.map(|v| v as u32),
    })
}
```

## Canonical body — `StreamEvent::ContextFill` arm

```rust
StreamEvent::ContextFill(info) => StreamChunk::context_fill_update(ContextFillInfo {
    fill_percentage: info.fill_percentage,
    effective_tokens: info.effective_tokens as f64,
    threshold: info.threshold as f64,
    context_window: info.context_window as f64,
}),
```

## Pre-existing implementation status

Both arms were already lifted as part of RPC-072/RPC-080/RPC-081 ports.
RPC-086 lands ACDD coverage pinning the contract so subsequent edits
cannot drop the `update_tokens` / `update_reasoning_tokens` calls or
the `TokenUpdate` / `ContextFillUpdate` chunk emissions without test
failure.

## Coverage strategy

Structural source-string assertions over the two arms plus census of
the rpc-types ctors, mirroring the pattern from RPC-082/083/084.
