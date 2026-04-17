# CMPCT-030 — AST Research: Tautological Compaction Tests

## Purpose

Map the tautological tests slated for deletion to their replacement targets,
and identify the minimal public surface each replacement test must invoke.

## Tautology Inventory

### 1. `codelet/cli/tests/gemini_continuation_compaction_test.rs` (187 lines)

All 4 `#[test]` functions are tautological:

| Test | Tautology |
|------|-----------|
| `test_graceful_handling_when_compaction_triggered_during_continuation` | `assert!("PromptCancelled".contains("PromptCancelled"))`; `assert!(true /* compaction_needed */)` |
| `test_session_continues_after_compaction_during_continuation` | Same pattern — local `let is_compaction_cancel = "PromptCancelled".contains("PromptCancelled");` |
| `test_partial_model_output_is_preserved_during_compaction` | `assert!(saved_text.unwrap() == partial_text)` where `saved_text` was set from `partial_text` four lines above |
| `documented_control_flow` | Empty body — "passes by reaching this point" |

None of them import from `codelet_cli::interactive::*`. Zero production call paths exercised.

### 2. `codelet/cli/tests/empty_turn_compaction_fix_test.rs` (141 lines)

All 3 `#[test]` functions rebuild the production gate locally:

```rust
let should_compact = effective_tokens > threshold && !session.turns.is_empty();
assert!(should_compact, "...");
```

`should_compact` is local bookkeeping — no call to the real gate in
`stream_loop.rs`. `test_defensive_check_logic` is the worst offender,
with empty and non-empty vecs shoved into tautological boolean
expressions.

### 3. `compaction_convergence_watchdog_test.rs::test_normal_compaction_no_watchdog`

```rust
let compaction_flag = Arc::new(AtomicBool::new(true));
compaction_flag.store(false, Ordering::SeqCst);
assert!(!compaction_flag.load(Ordering::Relaxed));
```

Sets an AtomicBool to false, asserts it is false. The rest of that file
uses real `force_inject_fallback_dag` / `extract_partial_dag_nodes` /
`COMPACTION_ESCALATION_MESSAGE` and stays.

## Replacement Targets (Already Public)

| Helper | Source | Exported at |
|--------|--------|-------------|
| `classify_compaction_branch` | `codelet/cli/src/interactive/error_classifiers.rs:258` | `codelet_cli::interactive` |
| `begin_compaction_recovery` | `codelet/cli/src/interactive/recovery_compaction.rs` | `codelet_cli::interactive` |
| `flush_partial_state_before_compaction` | `codelet/cli/src/interactive/recovery_compaction.rs` | `codelet_cli::interactive` |
| `compaction_retry_prompt` | `codelet/cli/src/interactive/recovery_compaction.rs` | `codelet_cli::interactive` |
| `CompactionRecoveryPolicy` | `codelet/cli/src/interactive/recovery_compaction.rs` | `codelet_cli::interactive` |
| `convert_messages_to_turns` | `codelet/cli/src/interactive_helpers.rs:33` | `codelet_cli::interactive_helpers` |

## Gemini Continuation Call Sites (AST-confirmed)

```
gemini_continuation.rs:341  classify_compaction_branch(&e, parent_token_state)
gemini_continuation.rs:361  super::recovery_compaction::begin_compaction_recovery(
                               session, parent_token_state, display,
                               text, output, false,
                            )
```

The `false` at site 361 is the Path D invariant — the continuation User
prompt is mid-flight and must not be popped.

## Stream-End-with-Flag Policy (Inline Duplicate)

`gemini_continuation.rs:395-408` reimplements the policy rule:

```rust
let policy = if partial_text_saved {
    CompactionRecoveryPolicy::ResumeFromPartial
} else {
    CompactionRecoveryPolicy::EmbedInInstruction
};
```

This is the same rule `flush_partial_state_before_compaction` returns
via its `Result<bool>` (CMPCT-028). The new integration test compares
the two sides against an identical fixture to catch future drift.

## Structural Assertions Scope

Production refactor is explicitly out of scope for this card, so we
cannot drive a real stream. The grep-style assertions below are the
cheapest way to nail down contracts that the CMPCT-023/026/028 cards
established:

- `gemini_continuation.rs` must reference `classify_compaction_branch` (CMPCT-026 contract)
- `gemini_continuation.rs` must reference `begin_compaction_recovery` (CMPCT-023 contract)
- `gemini_continuation.rs` must pass `false` for `pop_user_prompt` (CMPCT-023 Path D)
- The deleted test files must not reappear

Combined with direct helper invocation of `flush_partial_state_before_compaction`
and `convert_messages_to_turns`, this is tight enough to catch regressions
without requiring the `process_stream` extraction the original plan
proposed.
