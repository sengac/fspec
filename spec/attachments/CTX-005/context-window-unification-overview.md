# CTX-005: Unified Context Window and Configurable Compaction Thresholds — Research Overview

## Problem Summary

There are two fundamental issues with the current context window and compaction threshold architecture:

### Issue 1: Dual Source of Truth for Context Window

The TUI and Rust have **independent** paths for resolving `contextWindow`:

| Component | Source | Resolution Path | Opus 4.6 Value |
|-----------|--------|-----------------|-----------------|
| **TUI display** (`[1M]` badge) | `providerSections` → `NapiModelInfo.context_window` | models.dev API JSON → `model.limit.context` | **1,000,000** |
| **Compaction engine** | `ProviderManager::context_window()` | Per-model override > env var > provider constant | **1,000,000** (from models.dev via `select_model()`) |
| **Actual API limit** | Anthropic API enforcement | Requires `context-1m-2025-08-07` beta header for 1M | **200,000** (without header) |

The TUI and Rust currently **agree** on 1M (both read from models.dev), but **both are wrong** because the 1M beta header is commented out in `claude.rs:99-105` (CONFIG-007 not implemented). The API enforces 200k.

**Impact:** Compaction threshold = 968,000 tokens. The fill percentage meter shows ~20% when the user is actually at the API's hard limit. Compaction never triggers proactively — only the emergency "prompt too long" error handler saves the session.

### Issue 2: Compaction Threshold Hardcoded to Context Window

The compaction threshold is a **fixed mathematical function** of context_window:

```
threshold = context_window - min(max_output, SESSION_OUTPUT_TOKEN_MAX=32,000)
```

There is **no way to set a different compaction trigger per model**. This means:
- If a model reports 1M context but should compact at 200k, there's no mechanism to configure that
- Custom models on OpenAI-compatible APIs have no way to set appropriate compaction limits
- When Anthropic ships 1M context to broader tiers, existing users can't control when compaction fires

## Architecture Decision: Three Work Units

### CTX-006: Rust-Authoritative Context Window (Single Source of Truth)
Make Rust the sole authority. TUI reads context_window from Rust, not from models.dev JavaScript-side.

### CTX-007: Per-Model Configurable Compaction Threshold
Decouple compaction trigger from context_window. Support absolute tokens or percentage.

### CTX-008: TUI Configuration and NAPI Bridge
Add user-facing fields for compaction threshold configuration in Provider Settings and Custom Model Form.

## Dependency Graph

```
CTX-006 (Rust-authoritative context window)
   ↓
CTX-007 (Per-model compaction threshold)
   ↓
CTX-008 (TUI config + NAPI bridge)
```

CTX-008 depends on both CTX-006 and CTX-007 because it needs:
- The Rust-authoritative context window to compute percentage-based thresholds
- The per-model threshold infrastructure in Rust to wire up to

## Relationship to Existing Work Units

| Work Unit | Status | Relationship |
|-----------|--------|-------------|
| **MODEL-005** | Done | Established per-model context_window in ProviderManager. CTX-006 builds on this by making the TUI consume it from Rust instead of models.dev. |
| **CONFIG-007** | Backlog | 1M context opt-in for Anthropic Tier 4. Orthogonal — CONFIG-007 handles the beta header; CTX-007 handles compaction threshold. Both are needed for correct 1M support. |
| **CTX-002** | Done | Established `calculate_usable_context()` with `SESSION_OUTPUT_TOKEN_MAX`. CTX-007 augments this with per-model overrides. |

## Constants Reference

| Constant | Value | File | Purpose |
|----------|-------|------|---------|
| `AUTOCOMPACT_BUFFER` | 50,000 | `compaction_threshold.rs:27` | Post-compaction headroom target |
| `SESSION_OUTPUT_TOKEN_MAX` | 32,000 | `compaction_threshold.rs:75` | Output reservation cap |
| `claude::CONTEXT_WINDOW` | 200,000 | `claude.rs:42` | Claude provider fallback |
| `openai::CONTEXT_WINDOW` | 128,000 | `openai.rs:31` | OpenAI provider fallback |
| `gemini::CONTEXT_WINDOW` | 1,000,000 | `gemini.rs:20` | Gemini provider fallback |
| `codex::CONTEXT_WINDOW` | 272,000 | `codex/mod.rs:42` | Codex provider fallback |
