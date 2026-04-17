# Context Window Resolution Audit — Complete Provider Inventory

**Date:** 2026-04-16
**Auditor:** Claude Code (deep review)
**Scope:** All providers, all resolution paths, all compaction trigger points

---

## Executive Summary

The CTX-005 epic (CTX-006 through CTX-009) is **fundamentally broken**. The stated goal was "make Rust the single source of truth for context window" — but `ProviderManager::select_model()` unconditionally stores models.dev registry values into `model_context_window`, which **shadows the provider's compile-time constants**. For Claude Opus 4.6, this means:

- models.dev reports `context: 1,000,000` and `output: 128,000`
- `select_model()` stores `model_context_window = Some(1_000_000)`
- `context_window()` returns `1,000,000` (never reaches `claude::CONTEXT_WINDOW = 200,000`)
- Compaction threshold: `1,000,000 - 32,000 = 968,000`
- **Badge shows [968k], fill shows 9% at 87k tokens**
- **Real usable limit is ~200k (beta header not sent), compaction will NEVER trigger in time**

Additionally, context window resolution is scattered across 6+ providers with inconsistent patterns — constants in some, env vars in others, live APIs in one, JSON allowlists in another. There is no single unified resolution strategy.

---

## Problem 1: models.dev Unconditionally Overrides Provider Constants

### The Code (`manager.rs:300-302`)
```rust
// MODEL-005: Store per-model context limits from models.dev registry
self.model_context_window = Some(model_info.limit.context as usize);
self.model_max_output_tokens = Some(model_info.limit.output as usize);
```

### The Resolution (`manager.rs:684-686`)
```rust
pub fn context_window(&self) -> usize {
    self.model_context_window
        .unwrap_or_else(|| self.provider_constant_context_window())
}
```

Since `model_context_window` is always `Some(...)` after `select_model()`, the fallback to provider constants is **dead code** for any registry-selected model. This defeats the entire purpose of `claude::CONTEXT_WINDOW = 200_000`.

### Impact By Provider

| Provider | Constant | models.dev says | What actually gets used | Correct? |
|----------|----------|-----------------|-------------------------|----------|
| Claude Opus 4.6 | 200,000 | 1,000,000 | 1,000,000 | ❌ **WRONG** — 1M beta header not sent |
| Claude Sonnet 4 | 200,000 | 200,000 | 200,000 | ✅ Accidentally correct (values agree) |
| Gemini 2.5 Pro | 1,000,000 | 1,000,000 | 1,000,000 | ✅ Values agree |
| GPT-4o | 128,000 | 128,000 | 128,000 | ✅ Values agree |
| Copilot | 200,000 | N/A (own API) | Live API value | ✅ Correct path |
| Codex | 272,000 | N/A (bypass) | 272,000 or NAPI | ⚠️ Hardcoded guess |

---

## Problem 2: Context Window Sources Are Fragmented

### Current Architecture (Anti-Pattern)

```
Provider          Source 1 (constant)    Source 2 (dynamic)        Source 3 (env)
─────────────────────────────────────────────────────────────────────────────────
Claude            claude.rs:42           models.dev registry       None
                  200,000                limit.context
                                         (1M for opus-4-6)

OpenAI            openai.rs:24           models.dev registry       OPENAI_CONTEXT_WINDOW
                  128,000                limit.context             OPENAI_MAX_OUTPUT_TOKENS

Gemini            gemini.rs:20           models.dev registry       None
                  1,000,000              limit.context

Codex             codex/mod.rs:42        codex-models.json +       None
                  272,000                models.dev (OpenAI entries)

Z.AI              zai.rs:30              models.dev registry       None
                  128,000                limit.context

Copilot           copilot/mod.rs:64      Live /models endpoint     None
                  200,000                max_context_window_tokens
```

### Issues
1. **No standard resolution strategy** — each provider has its own combination of sources
2. **No provider veto** — providers cannot say "models.dev is wrong, I know better"
3. **Env var support is OpenAI-only** — inconsistent escape hatch
4. **Codex bypasses registry entirely** — uses `set_model_direct()` with baked-in 272k guess
5. **Copilot has its own API** — different from models.dev but feeding into the same `LimitInfo` format
6. **No validation** — models.dev can report 1M for a model where the API rejects >200k

---

## Problem 3: Compaction Threshold Resolution Has the Wrong Input

The `resolve_compaction_threshold()` in `compaction_threshold.rs` is well-designed — it has proper per-family defaults and a priority chain. But it receives **the wrong `context_window` value** as input because `ProviderManager::context_window()` returns models.dev data instead of the provider's intended limit.

For Claude Opus 4.6:
- Input: `context_window = 1,000,000` (models.dev)
- Claude family → returns `None` (defers to legacy)
- Legacy: `1,000,000 - min(128,000, 32,000) = 968,000`
- **Compaction at 968k, but API rejects at ~200k**

If the input were correct (200k):
- Input: `context_window = 200,000`
- Claude family → returns `None` (defers to legacy)
- Legacy: `200,000 - min(8,192, 32,000) = 191,808`
- **Compaction at ~192k — correct!**

Note: max_output_tokens is also wrong — models.dev says 128k, but Claude's constant is 8,192.

---

## Problem 4: The Badge and Fill% Are Downstream Consequences

CTX-009 changed the badge to show `compactionThreshold` instead of `contextWindow`. This was correct in intent but the threshold itself is derived from the wrong context_window (1M instead of 200k). So the badge shows [968k] and fill shows 9% — both wrong.

The fix cannot be in the badge display logic. It must be in the resolution of `context_window` itself.

---

## Problem 5: Codex Constants Are Guesses

```rust
pub const CONTEXT_WINDOW: usize = 272_000;      // "GPT-5.1 Codex context window size"
pub const MAX_OUTPUT_TOKENS: usize = 4096;       // "assumption: same as GPT-4"
```

These are hardcoded guesses for specific model generations. The Codex API doesn't expose limits. Different Codex models (gpt-5.1-codex, gpt-5.2-codex, gpt-5.4) likely have different context windows, but they all use the same 272k constant.

---

## Problem 6: `max_output_tokens` Disagrees Too

Not just context_window — max_output_tokens has the same models.dev override problem:

| Provider | Constant | models.dev | Impact |
|----------|----------|------------|--------|
| Claude | 8,192 | 128,000 | Output reservation in legacy formula changes from 8k to 32k (capped) |
| OpenAI | 4,096 | 16,384 (gpt-4o) | Less impactful (both < 32k cap) |
| Codex | 4,096 | N/A | Guessed value |

For Claude, this matters because the legacy formula is `context_window - min(max_output, 32,000)`:
- With constant max_output (8,192): `200,000 - 8,192 = 191,808`
- With models.dev max_output (128,000): `1,000,000 - 32,000 = 968,000`

---

## All Compaction Trigger Points (Must All Use Correct Threshold)

1. **Pre-prompt check** — `stream_loop.rs:301-351`
2. **CompactionHook mid-stream** — `compaction_hook.rs:161-224`
3. **Post-loop compaction** — `stream_loop.rs:1499-1525`
4. **Emergency (API error)** — `stream_loop.rs:1175-1201`
5. **Context fill display** — `stream_loop.rs:96-116`
6. **Thinking exhaustion check** — `stream_loop.rs:1033-1053` (uses context_window directly!)
7. **Retry hooks** — `stream_loop.rs:1057-1064, 1255-1261, 1354-1360`
8. **Compaction retry stream** — `compaction_retry.rs:36-47`

All of these receive the threshold from the single `resolve_compaction_threshold()` call at `stream_loop.rs:276-288`. The thinking exhaustion check (point 6) uses `context_window` directly — if that's wrong (1M instead of 200k), the 90% utilization check is also wrong.

---

## Recommended Architecture: Unified Model Limits Resolution

### Principle: Provider Has Final Authority

The provider MUST be able to override or clamp values from external registries. The resolution chain should be:

```
1. User-configured override (custom model, profile setting)
2. Provider-clamped registry value (models.dev says X, provider says "max Y")
3. Provider compile-time constant (fallback when no registry data)
```

### Proposed: `ModelLimitsResolver` Trait

```rust
pub trait ModelLimitsResolver: Send + Sync {
    /// The provider's hard maximum context window (API enforced limit).
    /// models.dev values will be clamped to this.
    fn max_context_window(&self) -> Option<usize> { None }
    
    /// The provider's hard maximum output tokens.
    fn max_output_tokens_limit(&self) -> Option<usize> { None }
    
    /// The provider's default context window when no registry data available.
    fn default_context_window(&self) -> usize;
    
    /// The provider's default max output tokens.
    fn default_max_output_tokens(&self) -> usize;
}
```

### Resolution

```rust
fn resolve_context_window(
    registry_value: Option<usize>,      // from models.dev / Copilot API
    user_override: Option<usize>,       // from custom model / profile config
    provider: &dyn ModelLimitsResolver,  // provider's limits
) -> usize {
    // User override wins but is clamped by provider max
    if let Some(user) = user_override {
        return provider.max_context_window()
            .map(|max| user.min(max))
            .unwrap_or(user);
    }
    // Registry value is clamped by provider max
    if let Some(registry) = registry_value {
        return provider.max_context_window()
            .map(|max| registry.min(max))
            .unwrap_or(registry);
    }
    // Fallback to provider default
    provider.default_context_window()
}
```

### Per-Provider Implementation

| Provider | `max_context_window()` | `default_context_window()` | Effect |
|----------|----------------------|---------------------------|--------|
| Claude | `Some(200_000)` (until CONFIG-007 opt-in) | `200_000` | Clamps models.dev 1M → 200k |
| OpenAI | `None` (trusts registry) | `128_000` | models.dev value used as-is |
| Gemini | `None` (trusts registry) | `1_000_000` | models.dev value used as-is |
| Codex | `None` | `272_000` | Uses constant when no registry data |
| Z.AI | `None` | `128_000` | models.dev value used as-is |
| Copilot | `None` | `200_000` | Live API value used as-is |
