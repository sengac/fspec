# AST Research — LIMITS-001 Parent Story

**Date:** 2026-04-16
**Work Unit:** LIMITS-001 (Parent)

## Summary

LIMITS-001 is a parent story with no direct implementation. All implementation is delivered through children LIMITS-002 through LIMITS-007.

## AST Research

Comprehensive AST research was performed at the epic level and attached to CTX-005:
- `spec/attachments/CTX-005/context-window-resolution-audit.md` — complete audit of all 6 providers

Each child story also performed its own AST research:
- LIMITS-002: `spec/attachments/LIMITS-002/ast-research-provider-limits.md`
- LIMITS-005: `spec/attachments/LIMITS-005/ast-research-compaction-chain.md`
- LIMITS-006: `spec/attachments/LIMITS-006/ast-research-badge-display-chain.md`
- LIMITS-007: `spec/attachments/LIMITS-007/ast-research-model-limits-chain.md`

## Key Files Modified

### Rust (codelet/providers/src/)
- `model_limits.rs` — NEW: ModelLimitsResolver trait + resolve_model_limits() function
- `manager.rs` — MODIFIED: context_window()/max_output_tokens() now resolve through ModelLimitsResolver
- `claude.rs` — MODIFIED: impl ModelLimitsResolver for ClaudeProvider (clamps to 200k)
- `openai.rs` — MODIFIED: impl ModelLimitsResolver for OpenAiProvider
- `gemini.rs` — MODIFIED: impl ModelLimitsResolver for GeminiProvider
- `codex/mod.rs` — MODIFIED: impl ModelLimitsResolver for CodexProvider (should_send_max_output=false)
- `zai.rs` — MODIFIED: impl ModelLimitsResolver for ZaiProvider
- `copilot/provider.rs` — MODIFIED: impl ModelLimitsResolver for CopilotProvider

### TypeScript (src/tui/)
- `components/__tests__/sessionheader-badge-threshold.test.tsx` — MODIFIED: updated expected values
- `components/__tests__/rust-authoritative-context-window.test.ts` — MODIFIED: updated expected values
