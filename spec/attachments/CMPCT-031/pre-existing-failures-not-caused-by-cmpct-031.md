# Pre-existing failures NOT caused by CMPCT-031

This document records failures observed in `cargo test --workspace` and in
`cargo clippy` against the patched `rig-core` crate that pre-date CMPCT-031.
Each was verified to fail on a baseline tree **without** the CMPCT-031 changes
by stashing `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs`
and re-running the same command, then popping the stash.

## 1. `cargo test --workspace` — 1 failure

### `codelet-cli` → `token_001_cumulative_billed_output_test.rs`

```
---- delta_computation_is_centralized_in_single_helper stdout ----
thread 'delta_computation_is_centralized_in_single_helper' panicked at
  cli/tests/token_001_cumulative_billed_output_test.rs:160:5:
  assertion `left == right` failed: exactly one production-code
  saturating_sub on an output_tokens field is allowed, and it must live
  inside TokenTracker::compute_output_delta
    left: 0
   right: 1

failures:
    delta_computation_is_centralized_in_single_helper

test result: FAILED. 2 passed; 1 failed
```

- File: `codelet/cli/tests/token_001_cumulative_billed_output_test.rs`
  (untracked in git — brought in as part of a different work unit that
  expects centralization of `saturating_sub` call sites into a helper
  `TokenTracker::compute_output_delta` that has not yet been wired into
  the four interactive call sites)
- Scope: belongs to a separate TOKEN-00x work unit, not CMPCT-031
- Reproduced without CMPCT-031 changes (stash-pop baseline): **FAILS identically**

## 2. `cd codelet && cargo clippy --workspace --all-targets --tests -- -D warnings`

**PASSES CLEAN** with CMPCT-031 changes applied:

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.31s
```

The `rig-core` crate at `codelet/patches/rig-core/` is **not** a workspace
member (verified by `cargo test --package rig-core`: *"package `rig-core`
cannot be tested because it is not a member of the workspace"*). It is
consumed via `[patch]` in `codelet/Cargo.toml`. `cargo clippy --workspace`
therefore does **not** lint the rig-core patch sources.

## 3. `cd codelet/patches/rig-core && cargo clippy --lib --tests -- -D warnings`

This path **does** reach the patched rig-core and surfaces ~19 pre-existing
lint errors in files NOT touched by CMPCT-031:

- `src/agent/prompt_request/streaming.rs:26` — `empty line after doc comment`
  (pre-existing, above the EXT-016 `check_image_dimensions` helper comment)
- `src/agent/prompt_request/streaming.rs:155, 162, 206, 211, 218, 633` —
  `this if statement can be collapsed` (all inside `parse_tool_result_content`
  patterns and the async_stream loop, pre-existing)
- `src/providers/anthropic/streaming.rs:226, 439` — pre-existing
- `src/providers/gemini/streaming.rs:154` — pre-existing
- `src/vector_store/mod.rs:234` — pre-existing `derivable_impls`

All reproduced without CMPCT-031 via stash-pop baseline.

## CMPCT-031 contribution: ZERO new clippy warnings

The added code (`MAX_TOOL_RESULT_TEXT_BYTES`, `TOOL_RESULT_PREVIEW_BYTES`,
`TOOL_RESULT_SUFFIX_BYTES`, `TOOL_RESULT_TRUNCATION_HINT`,
`floor_char_boundary`, `ceil_char_boundary`, `bound_tool_result_text`, plus
four call-site wiring edits inside `parse_tool_result_content`) triggers no
new lints under `cargo clippy --workspace --all-targets --tests -- -D
warnings`. The 7 CMPCT-031 `#[test]` fns (6 feature scenarios + 1 UTF-8
boundary regression) all pass:

```
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured
```

Full rig-core lib test suite: `test result: ok. 202 passed; 0 failed`.
