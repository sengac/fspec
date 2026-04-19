# AST Research: Opus 4.7 Adaptive Thinking Code Paths

## Search: ADAPTIVE_THINKING_MODELS constant

**Pattern:** `pub const ADAPTIVE_THINKING_MODELS: &[&str] = &[$$$MODELS]`

| File | Line | Match |
|------|------|-------|
| `codelet/tools/src/facade/thinking_config.rs` | 32 | `pub const ADAPTIVE_THINKING_MODELS: &[&str] = &[CLAUDE_OPUS_4_6, CLAUDE_SONNET_4_6];` |

**Finding:** `claude-opus-4-7` is MISSING from the list. This is the root cause.

## Search: is_adaptive_thinking_model function

**Pattern:** `pub fn is_adaptive_thinking_model($$$ARGS) -> $RET { $$$BODY }`

| File | Line | Match |
|------|------|-------|
| `codelet/tools/src/facade/thinking_config.rs` | 46 | `pub fn is_adaptive_thinking_model(model: &str) -> bool { ADAPTIVE_THINKING_MODELS.contains(&model) }` |

**Finding:** Uses `.contains()` (exact equality) — adding to the array is sufficient.

## Search: Callers of is_adaptive_thinking_model

**Pattern:** `is_adaptive_thinking_model($$$ARGS)`

| File | Line | Context |
|------|------|---------|
| `codelet/tools/src/facade/thinking_config.rs` | 216 | `request_config_for_model()` — decides adaptive vs budgeted |
| `codelet/providers/src/claude.rs` | 95 | `build_beta_headers()` — decides whether to include interleaved-thinking beta |

**Finding:** Two call sites. Both will automatically work correctly once the constant is updated.

## Search: build_beta_headers function

**Pattern:** `pub fn build_beta_headers($$$ARGS) -> $RET { $$$BODY }`

| File | Line |
|------|------|
| `codelet/providers/src/claude.rs` | 81 |

**Finding:** Skips `interleaved-thinking` header when `is_adaptive_thinking_model()` returns true.

## Search: NAPI get_thinking_config function

**Pattern:** `pub fn get_thinking_config($$$ARGS) -> $RET { $$$BODY }`

| File | Line |
|------|------|
| `codelet/napi/src/thinking_config.rs` | 124 |

**Finding:** Routes Claude models through `ClaudeThinkingFacade.request_config_for_model()`, which calls `is_adaptive_thinking_model()`.

## Conclusion

All three affected layers (thinking config, beta headers, NAPI) flow through a single function `is_adaptive_thinking_model()` which reads from `ADAPTIVE_THINKING_MODELS`. Adding `CLAUDE_OPUS_4_7` to the array fixes all three layers.
