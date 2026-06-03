# RPC-087 AST Research — Error Classification + Recovery Helpers Wiring

**Date:** 2026-06-01
**Status:** Implementation already exists. This card is regression-shape
coverage to pin the structural invariants of the classifier + recovery
helper modules and the call sites in `stream_loop.rs` that compose them.

## Why regression-shape (not behavioural)

The original RPC-087 description proposed porting the helpers from
`codelet/cli/src/interactive/` and writing one behavioural acceptance
test that injects a 429 then a success. After RPC-084 (streaming) the
dispatch routes through `codelet_cli::interactive::run_agent_stream_with_images`
(see `codelet/agent-loop/src/dispatch.rs:88`), which means **every call**
into the agent loop already flows through `stream_loop.rs`, which **already
imports and invokes** all six recovery helpers and the classifier module.

A behavioural 429-injection test would require building a scripted stub
provider that fakes HTTP 429-then-success at the rig streaming layer.
That stub is an order of magnitude more work than what the gap demands.
The actual regression risk is "someone accidentally deletes one of the
`use super::recovery_*` lines or one of the classifier call sites in
`stream_loop.rs`" — which is exactly what source-shape assertions catch
in sub-millisecond tests.

This card follows the established regression-shape pattern (RPC-149,
RPC-150, RPC-151, RPC-152, RPC-153, RPC-155, RPC-156, RPC-077, RPC-088,
RPC-089, RPC-090).

## Canonical implementation locations

### Module declarations — `codelet/cli/src/interactive/mod.rs`

```rust
// :7
mod error_classifiers;
// :12-17
mod recovery_compaction;
mod recovery_image;
mod recovery_network;
mod recovery_stall;
mod recovery_thinking;
mod recovery_truncation;
```

### Public re-exports — `codelet/cli/src/interactive/mod.rs:22-68`

```rust
pub use error_classifiers::{
    is_prompt_too_long_error, is_image_content_error,
    is_truncated_tool_call_error, is_transient_network_error,
    is_stall_timeout_error, classify_compaction_branch,
    CompactionBranch, CompactionDisagreement,
};
pub use recovery_truncation::{
    MAX_TRUNCATION_RETRIES,
    build_truncation_recovery_message,
    build_truncation_budget_exhausted_message,
};
pub use recovery_thinking::{
    MAX_THINKING_EXHAUSTION_RETRIES,
    THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD,
    is_thinking_exhaustion, build_thinking_exhaustion_recovery_message,
    build_thinking_budget_exhausted_message, downgrade_thinking_level,
};
pub use recovery_image::sanitize_image_content;
pub use recovery_compaction::{
    begin_compaction_recovery, build_compaction_budget_exhausted_message,
    compaction_retry_prompt, execute_compaction_and_capture_events,
    flush_partial_state_before_compaction, CompactionRecoveryPolicy,
    MAX_COMPACTION_RETRIES,
};
pub use recovery_network::{ MAX_NETWORK_RETRIES, network_retry_delay };
pub use recovery_stall::{
    STALL_TIMEOUT_SECS, STALL_TIMEOUT_ERROR_PREFIX,
    DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS,
    build_stall_timeout_message, build_deep_search_timeout_message,
    stall_timeout_duration, deep_search_wall_clock_timeout,
};
```

### Classifier + recovery helper imports — `codelet/cli/src/interactive/stream_loop.rs`

```rust
// :78
use super::error_classifiers::{
    is_prompt_too_long_error, is_image_content_error,
    is_truncated_tool_call_error, is_transient_network_error,
    is_stall_timeout_error, classify_compaction_branch,
    extract_prompt_cancelled, CompactionBranch,
};
// :81
use super::recovery_image::sanitize_image_content;
// :84
use super::recovery_truncation::{
    MAX_TRUNCATION_RETRIES,
    build_truncation_recovery_message,
    build_truncation_budget_exhausted_message,
};
// :87-90
use super::recovery_thinking::{
    is_thinking_exhaustion, build_thinking_exhaustion_recovery_message,
    /* … */
};
// :95
use super::recovery_network::{ MAX_NETWORK_RETRIES, network_retry_delay };
// :98
use super::recovery_stall::{ build_stall_timeout_message, stall_timeout_duration };
// :102
use super::recovery_compaction::MAX_COMPACTION_RETRIES;
```

### Classifier + recovery helper call sites — `stream_loop.rs`

```rust
// :754, :783, :803  — stall timeout message construction
build_stall_timeout_message(effective_stall_timeout.as_secs())

// :1188            — thinking exhaustion recovery prompt
build_thinking_exhaustion_recovery_message(…)

// :1477            — stall classification
if is_stall_timeout_error(&error_str) { … break; }

// :1483            — prompt-too-long classification
let is_prompt_too_long = is_prompt_too_long_error(&error_str);

// :1535-1547       — image-content classification + sanitization
if is_image_content_error(&error_str) { …
    let sanitized = sanitize_image_content(&mut session.messages);
}

// :1564-1639       — truncation classification + recovery + budget
if is_truncated_tool_call_error(&error_str) { …
    let recovery_prompt = build_truncation_recovery_message(&error_str);
    let budget_error = build_truncation_budget_exhausted_message(MAX_TRUNCATION_RETRIES);
}

// :1649-1737       — network classification + bounded retry
if is_transient_network_error(&error_str) { …
    if network_retry_count <= MAX_NETWORK_RETRIES {
        let delay = network_retry_delay(network_retry_count);
        …
    }
}

// :1363            — compaction branch classification
let branch = classify_compaction_branch(&e, &token_state);

// :1325, :1450, :1516, :1869
super::recovery_compaction::begin_compaction_recovery(…)
super::recovery_compaction::execute_compaction_and_capture_events(…)
```

### Dispatch route — `codelet/agent-loop/src/dispatch.rs:88`

```rust
codelet_cli::interactive::run_agent_stream_with_images(
    agent, $input, $images, $inner,
    $session.is_interrupted.clone(),
    $session.compaction_in_progress.clone(),
    $session.interrupt_notify.clone(),
    $output,
).await
```

Every agent-loop dispatch flows through `run_agent_stream_with_images`,
which means every classifier + recovery helper in `stream_loop.rs` is
in-path for every turn.

## Invariants to pin

1. `codelet/cli/src/interactive/mod.rs` declares modules
   `error_classifiers`, `recovery_compaction`, `recovery_image`,
   `recovery_network`, `recovery_stall`, `recovery_thinking`,
   `recovery_truncation`.

2. `codelet/cli/src/interactive/mod.rs` re-exports the public surface:
   - classifier predicates: `is_prompt_too_long_error`,
     `is_image_content_error`, `is_truncated_tool_call_error`,
     `is_transient_network_error`, `is_stall_timeout_error`,
     `classify_compaction_branch`
   - network constants: `MAX_NETWORK_RETRIES`, `network_retry_delay`
   - stall identifiers: `STALL_TIMEOUT_ERROR_PREFIX`
   - image helper: `sanitize_image_content`

3. `codelet/cli/src/interactive/recovery_network.rs`:
   - `pub const MAX_NETWORK_RETRIES: u32 = 3`
   - `pub fn network_retry_delay(attempt: u32) -> Duration`
   - exponential backoff `BASE * 2^(attempt-1)` with base = 1000 ms

4. `codelet/cli/src/interactive/error_classifiers.rs`:
   - `is_stall_timeout_error` uses `super::recovery_stall::STALL_TIMEOUT_ERROR_PREFIX`
     (single source of truth — no duplicated string)
   - `is_transient_network_error` recognizes
     `"error sending request"`, `"connection reset"`,
     `"connection refused"`, `"operation timed out"` and friends
   - `classify_compaction_branch` returns `CompactionBranch::Recover` /
     `NotCompaction` variants with optional `CompactionDisagreement`

5. `codelet/cli/src/interactive/stream_loop.rs` imports all six
   helper modules + the classifier module via `use super::recovery_*`
   and `use super::error_classifiers::{…}` and contains at least one
   call site for each:
   - `is_stall_timeout_error(…)`
   - `is_prompt_too_long_error(…)`
   - `is_image_content_error(…)` + `sanitize_image_content(…)`
   - `is_truncated_tool_call_error(…)` + `build_truncation_recovery_message(…)`
   - `is_transient_network_error(…)` + `network_retry_delay(…)` +
     `MAX_NETWORK_RETRIES`
   - `classify_compaction_branch(…)` + `begin_compaction_recovery(…)`

6. `codelet/agent-loop/src/dispatch.rs` contains exactly one
   `codelet_cli::interactive::run_agent_stream_with_images(` call inside
   the `run_with_provider!` macro body, so every agent-loop dispatch
   funnels into the recovery-wired streaming engine.

## Implementation strategy

One regression-shape test file `codelet/cli/tests/rpc087_error_recovery_wiring_shape.rs`
(plus a sibling under `codelet/agent-loop/tests/` for the dispatch
invariant) using:

- source-string substring assertions for module declarations and `use`
  imports
- direct use of the re-exported public API to assert constants (e.g.
  `MAX_NETWORK_RETRIES == 3`) and `network_retry_delay` behaviour
- direct invocation of `is_transient_network_error("HTTP 429 …")` and
  `is_stall_timeout_error(STALL_TIMEOUT_ERROR_PREFIX)` to assert
  classifier semantics

All sub-millisecond. No HTTP, no stub provider, no rig stream.

## Re-estimate

Original 8 pts was for "port from NAPI + write behavioural acceptance
test". The port is already done (RPC-072 refit + RPC-084 streaming
landed the underlying call into `run_agent_stream_with_images`); the
behavioural acceptance test is structurally identical to the
classifier-level tests that already live inside `error_classifiers.rs`.

Regression-shape coverage of the wiring is **2 points**.
