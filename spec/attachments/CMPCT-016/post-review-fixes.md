# CMPCT-016 Post-Review Fixes

Critical review of all 5 children (CMPCT-017 through CMPCT-021) found 6 actionable issues across SOLID/DRY/COMPOSABLE concerns.

## Issues

- [x] **1. DRY Violation: Duplicated DAG wrapping format**
  - `force_inject_fallback_dag()` in `interactive_helpers.rs` hardcoded `<system-reminder><!-- type:compaction-dag -->` format
  - `inject_summary_handler.rs` had the canonical `wrap_dag_content()` doing the same thing
  - **Fix:** Moved `wrap_dag_content()` to `codelet-core::compaction::model` as the single canonical source. Both `inject_summary_handler` (napi) and `compaction_dag` (cli) now import from core.

- [x] **2. Dead Code: `filter_by_turn_range` compiler warning**
  - `session_search_handler.rs` defined `filter_by_turn_range` in production scope but it was never called
  - Filtering was done inline in `handle_show` and `handle_search`
  - Caused compiler warning on every build
  - **Fix:** Moved function into `#[cfg(test)]` module where it's actually used by tests. Production code stays clean.

- [x] **3. `eprintln!` in Production: DAG overlap warning**
  - `model.rs` used `eprintln!` for overlap warnings in `parse_dag_nodes()`
  - Project uses `tracing` for structured logging everywhere else
  - **Fix:** Replaced with `tracing::warn!`

- [x] **4. Instruction Step Numbering Typo**
  - `COMPACTION_INSTRUCTION_FRESH` had steps numbered 1, 2, 4, 3
  - Step 4 (dag-files) appeared before step 3 (call inject_summary)
  - **Fix:** Renumbered to 1, 2, 3, 4

- [x] **5. Regex Recompiled on Every Call**
  - `parse_dag_nodes()` (model.rs) compiled regex per call
  - `extract_partial_dag_nodes()` (compaction_dag.rs) compiled regex per call
  - Project already uses `once_cell::sync::Lazy` for regexes elsewhere
  - **Fix:** Both now use `once_cell::sync::Lazy` static regexes, consistent with trimmer.rs, token_estimator.rs, thinking_level_detection.rs

- [x] **6. SRP: Extract compaction DAG concerns from `interactive_helpers.rs`**
  - File was 599 lines with 12+ responsibilities
  - **Fix:** Created `cli/src/compaction_dag.rs` (257 lines) with: 3 instruction constants, `detect_existing_dag()`, `extract_partial_dag_nodes()`, `force_inject_fallback_dag()`. Re-exports from `interactive_helpers` maintain backward compatibility. `interactive_helpers.rs` reduced to 378 lines.

## Summary

| File | Before | After |
|------|--------|-------|
| `cli/src/interactive_helpers.rs` | 599 lines | 378 lines |
| `cli/src/compaction_dag.rs` | (new) | 257 lines |
| `core/src/compaction/model.rs` | 426 lines | 443 lines (+wrap_dag_content, Lazy regex) |
| `napi/src/inject_summary_handler.rs` | 566 lines | 550 lines (-wrap_dag_content) |
| `napi/src/session_search_handler.rs` | 1610 lines | 1606 lines (-dead fn, +test-only fn) |

Compiler warnings: 0 (was 1 — `filter_by_turn_range` dead code)
All existing tests pass with no changes to test imports (backward-compat re-exports).
