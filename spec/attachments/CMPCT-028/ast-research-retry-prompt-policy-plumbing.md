# CMPCT-028 — AST Research: Retry Prompt Policy Plumbing

Research conducted with `AstGrep`/`Grep` over `codelet/cli/src/interactive/` and `codelet/cli/tests/` to enumerate every site that must change when `begin_compaction_recovery` returns a `CompactionRecoveryPolicy` instead of `()` and every site that consumes the macro-driven hard-coded `"Continue"` prompt.

Note: this research augments the CMPCT-028 plan in `spec/attachments/CMPCT-028/plan.md`, which was written against the now-deleted `compaction_retry.rs:128-133` location. The fix now lands in the CMPCT-027 in-loop restart macro.

## 1. Function signatures to change

| # | Location | Current signature | Target signature |
|---|----------|-------------------|------------------|
| 1 | `codelet/cli/src/interactive/recovery_compaction.rs:107` | `pub fn flush_partial_state_before_compaction(...) -> Result<()>` | `pub fn flush_partial_state_before_compaction(...) -> Result<bool>` |
| 2 | `codelet/cli/src/interactive/recovery_compaction.rs:179` | `pub fn begin_compaction_recovery<O: StreamOutput>(...) -> Result<()>` | `pub fn begin_compaction_recovery<O: StreamOutput>(...) -> Result<CompactionRecoveryPolicy>` |

## 2. New items to add in `recovery_compaction.rs`

- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum CompactionRecoveryPolicy { EmbedInInstruction, ResumeFromPartial }`
- `pub fn compaction_retry_prompt(policy: CompactionRecoveryPolicy) -> &'static str`

## 3. Re-export surface

File: `codelet/cli/src/interactive/mod.rs:47`

Extend the existing `pub use recovery_compaction::{ .. }` to include:
- `CompactionRecoveryPolicy`
- `compaction_retry_prompt`

## 4. Call-site inventory for `begin_compaction_recovery`

From `Grep`:

| # | File:line | Path | Action |
|---|-----------|------|--------|
| 1 | `gemini_continuation.rs:352` | D (Gemini continuation cancel) | Capture returned policy into a local; forward via `GeminiContinuationResult::CompactionNeeded(policy)` |
| 2 | `stream_loop.rs:1306` | C (hook cancel) | Capture returned policy into a local; pass into `in_loop_compaction_restart!(policy)` |
| 3 | `stream_loop.rs:1366` | B (API "prompt too long") | Capture returned policy into a local; pass into `in_loop_compaction_restart!(policy)` |

Path A (pre-prompt compaction, `stream_loop.rs:338`) does **not** call `begin_compaction_recovery` and does not need modification. Its existing `effective_prompt = if compaction_just_ran { "Continue" } else { prompt }` already reflects `EmbedInInstruction` semantics and cannot produce partial text.

## 5. Macro call-site inventory for `in_loop_compaction_restart!()`

From `Grep` on `stream_loop.rs`:

| # | Line | Path | Update |
|---|------|------|--------|
| 1 | `1085` | D | Pass policy captured from `GeminiContinuationResult::CompactionNeeded(policy)` |
| 2 | `1318` | C | Pass policy captured from `begin_compaction_recovery` at 1306 |
| 3 | `1378` | B | Pass policy captured from `begin_compaction_recovery` at 1366 |

## 6. Macro body changes (`stream_loop.rs:588-663`)

Current macro body hard-codes `"Continue"` at line 642. Change:

- Accept `$policy:expr` in the macro matcher.
- Replace `"Continue"` with `super::recovery_compaction::compaction_retry_prompt($policy)`.
- Add a `debug!` log recording the selected policy before the retry stream is issued.

## 7. Enum variant change in `GeminiContinuationResult`

File: `codelet/cli/src/interactive/gemini_continuation.rs:35-42`

Today:
```rust
pub(super) enum GeminiContinuationResult {
    NoContinuation,
    Completed,
    CompactionNeeded,
}
```

Change `CompactionNeeded` → `CompactionNeeded(CompactionRecoveryPolicy)` so the primary loop can thread the policy into the macro. Match arm in `stream_loop.rs:1065-1087` must destructure the new payload.

## 8. Existing integration tests that call `flush_partial_state_before_compaction`

`codelet/cli/tests/compaction_cancel_preservation_test.rs` lines 34, 92, 143 — each currently does `.expect("flush helper must succeed")`. With the return type change to `Result<bool>`, these calls still compile because `.expect(..)` on `Result<bool, _>` yields `bool`, which is silently discarded. No changes needed to preserve the existing semantic; new tests will exercise the return value explicitly.

## 9. New integration tests to add (`codelet/cli/tests/compaction_retry_prompt_policy_test.rs`)

Proposed test names mapped to Gherkin scenarios:

| Scenario | Test |
|----------|------|
| flush reports true when partial text appended | `flush_returns_true_when_partial_text_present` |
| flush reports false when buffer empty | `flush_returns_false_when_buffer_empty` |
| begin_compaction_recovery returns EmbedInInstruction when no partial text | `begin_compaction_recovery_returns_embed_in_instruction_when_no_partial` |
| begin_compaction_recovery returns ResumeFromPartial when partial text saved | `begin_compaction_recovery_returns_resume_from_partial_when_partial_saved` |
| compaction_retry_prompt maps EmbedInInstruction to "Continue" | `compaction_retry_prompt_embed_in_instruction_is_continue` |
| compaction_retry_prompt maps ResumeFromPartial to resume text | `compaction_retry_prompt_resume_from_partial_mentions_left_off` |

Reuse `RecordingOutput` pattern from `compaction_error_cascade_test.rs` for the two `begin_compaction_recovery` tests (they do not touch debug-capture, so no `ensure_test_data_dir()` needed).

## 10. Bounded impact summary

**Files modified:**
- `codelet/cli/src/interactive/recovery_compaction.rs` — add enum + helper fn, widen two signatures
- `codelet/cli/src/interactive/mod.rs` — extend re-export list
- `codelet/cli/src/interactive/gemini_continuation.rs` — widen `CompactionNeeded` variant, destructure at one call site
- `codelet/cli/src/interactive/stream_loop.rs` — update three `begin_compaction_recovery` call sites, widen macro signature, update three macro invocations, destructure `CompactionNeeded(policy)` at Path D match arm

**Files added:**
- `codelet/cli/tests/compaction_retry_prompt_policy_test.rs` — 6 integration tests

**Files NOT modified:**
- `codelet/cli/src/interactive/error_classifiers.rs` — doc-comment mention only
- `codelet/cli/src/interactive/recovery_*.rs` (image/network/stall/thinking/truncation) — unrelated cascades
- `codelet/cli/tests/compaction_cancel_preservation_test.rs` — return-type widening is non-breaking for existing `.expect(..).into()` usage (value silently dropped)
- `codelet/cli/src/bridge_relay.rs` — BRIDGE-020 scope
