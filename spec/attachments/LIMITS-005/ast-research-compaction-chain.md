# AST Research: Compaction Threshold Chain Verification

## 1. resolve_compaction_threshold — Pure function in compaction_threshold.rs
Location: codelet/cli/src/compaction_threshold.rs:197
```rust
pub fn resolve_compaction_threshold(context_window, max_output, model_id, user_config) -> u64
```
Receives values from callers — no direct ProviderManager dependency.

## 2. stream_loop.rs:276 — Threshold Resolution
```rust
let context_window = session.provider_manager().context_window() as u64;
let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
let threshold = resolve_compaction_threshold(context_window, max_output_tokens, ...);
```
After LIMITS-004, `context_window()` returns clamped value (200k for Claude). ✅

## 3. stream_loop.rs:1033-1040 — Thinking Exhaustion Check
```rust
let utilization_pct = (current_tokens as f64 / context_window as f64) * 100.0;
```
Uses `context_window` from line 276 — already clamped. ✅

## 4. stream_loop.rs:98-116 — Context Fill Emission
```rust
pub(super) fn emit_context_fill_from_usage(output, usage, threshold, context_window)
```
Both `threshold` and `context_window` are passed from clamped values. ✅

## 5. Sub-Agent Propagation (session_manager.rs:4955, 4988)
```rust
let deep_search_context_window = inner_session.provider_manager().raw_model_context_window();
let spawner_context_window = inner_session.provider_manager().raw_model_context_window();
```
`raw_model_context_window()` → `Some(self.context_window())` → clamped. ✅

## 6. Session Cached Values (session_manager.rs:3377-3440)
```rust
let initial_context_window = provider_manager.context_window() as u32;
let initial_compaction_threshold = resolve_compaction_threshold(...) as u32;
session.set_model_limits(initial_context_window, initial_max_output_tokens, initial_compaction_threshold);
```
All reads go through `context_window()` → clamped. ✅

## 7. NAPI SessionModel (session_manager.rs:6795-6815)
```rust
let context_window = session.cached_context_window.load(Ordering::Acquire);
let compaction_threshold = session.cached_compaction_threshold.load(Ordering::Acquire);
```
Reads from AtomicU32 cache populated with clamped values. ✅

## Conclusion
All 7 consumers use clamped values from ProviderManager.context_window() after LIMITS-004.
No code changes required — only verification tests needed.
