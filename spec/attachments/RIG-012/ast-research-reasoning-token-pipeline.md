# AST Research: Reasoning Token Propagation Pipeline

## Date: 2026-03-04
## Work Unit: RIG-012

## Structs in the Token Pipeline

### codelet-core layer
- `codelet/core/src/streaming_display/streaming_token_display.rs:11` - `TokenDisplayUpdate` (MISSING reasoning_tokens)
- `codelet/core/src/streaming_display/streaming_token_display.rs:89` - `StreamingTokenDisplay` (manages display state)
- `codelet/core/src/token_usage.rs:20` - `ApiTokenUsage` (HAS reasoning_tokens ✅)
- `codelet/core/src/compaction/model.rs:57` - compaction `TokenTracker` (MISSING reasoning_tokens)

### codelet-cli layer
- `codelet/cli/src/interactive/output.rs:45` - `impl From<TokenDisplayUpdate> for TokenInfo` (MISSING reasoning_tokens mapping)
- `codelet/cli/src/interactive/output.rs:20` - `TokenInfo` struct (MISSING reasoning_tokens)

### codelet-napi layer
- `codelet/napi/src/types.rs:123` - NAPI `TokenTracker` (MISSING reasoning_tokens)
- `codelet/napi/src/session_manager.rs:832` - `SessionTokens` (MISSING reasoning_tokens)
- `codelet/napi/src/session_manager.rs:1213` - `update_tokens` (only input/output)
- `codelet/napi/src/session_manager.rs:5778` - `StreamEvent::Tokens` conversion (MISSING reasoning_tokens mapping)

### Persistence layer
- `codelet/napi/src/persistence/types.rs:18` - `TokenUsage` (MISSING reasoning_tokens)
- `codelet/napi/src/persistence/napi_bindings.rs:482` - `NapiTokenUsage` (MISSING reasoning_tokens)
- `codelet/napi/src/persistence/napi_bindings.rs:323` - `persistence_set_session_tokens` (MISSING reasoning param)
- `codelet/napi/src/persistence/mod.rs:636` - `set_session_tokens` (MISSING reasoning param)

### TypeScript layer
- `src/tui/utils/sessionHeaderUtils.ts:39` - `TokenTracker` interface (MISSING reasoningTokens)
- `src/tui/components/SessionHeader.tsx:210` - Token display line (shows only input/output)
- `src/tui/utils/tokenStateUtils.ts:103` - `calculateContextFillPercentage` (ignores reasoning)
- `src/tui/utils/tokenStateUtils.ts:127` - `persistTokenState` (doesn't persist reasoning)

## Data Flow (Current Gap)

```
ApiTokenUsage.reasoning_tokens (✅ exists)
    ↓ creates TokenDisplayUpdate (❌ MISSING reasoning_tokens)
        ↓ converts to TokenInfo (❌ MISSING reasoning_tokens)  
            ↓ emits StreamEvent::Tokens
                ↓ converts to NAPI TokenTracker (❌ MISSING reasoning_tokens)
                    ↓ exposed to TypeScript TUI (❌ MISSING reasoningTokens)
                        ↓ SessionHeader display (❌ never shows reasoning)
```

## Reference: Codex Implementation (Working)
- `/tmp/codex/codex-rs/protocol/src/protocol.rs:1527` - TokenUsage has `reasoning_output_tokens: i64`
- `/tmp/codex/codex-rs/protocol/src/protocol.rs:1695` - `add_assign` includes reasoning
- `/tmp/codex/codex-rs/protocol/src/protocol.rs:1729` - Display shows "(reasoning N)" when > 0
- `/tmp/codex/codex-rs/codex-api/src/sse/responses.rs:130` - SSE parsing extracts reasoning_tokens
