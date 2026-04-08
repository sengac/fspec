# AST Research — AMGR-018 (profile action rewrite)

Performed via `AstGrep` + `Grep` on 2026-04-08 to identify every surface that the
rewrite of `codelet_tools::profile::session::run_pprof_window` must touch.

## 1. Target function under rewrite

```
codelet/tools/src/profile/session.rs:185:1:
    fn run_pprof_window(sleep_duration: Duration) -> Vec<ScopeReport>
codelet/tools/src/profile/session.rs:289:1:
    fn run_pprof_window(sleep_duration: Duration) -> Vec<ScopeReport>  (Windows cfg fallback)
```

Two definitions gated on `#[cfg(unix)]` / `#[cfg(not(unix))]`. Both must be
updated to also return the new per-thread / hot-stack / sampling data.

## 2. Public result struct

```
codelet/tools/src/profile/result.rs:13:1: pub struct ProfileResult { ... }
```

Current fields: `duration_secs, started_at, ended_at, process, runtime,
scopes_by_calls, scopes_by_self_ms, channels`.

**Contract extension (additive only):** add `samples_by_thread`, `hot_stacks`,
`sampling` as new fields. Existing consumers in `ProfileSession::run()`,
`handle_profile()`, and the TS binding keep working.

## 3. AgentManagerAction::Profile variant

Seven call sites reference `AgentManagerAction::Profile`:

```
codelet/tools/src/agent_manager/types.rs:104       Profile { duration_secs, top_n, label_prefix }
codelet/tools/src/agent_manager/mod.rs:192         async dispatch gate
codelet/tools/src/agent_manager/mod.rs:155-170     JSON schema for rig::tool::Tool
codelet/tools/src/agent_manager/tests.rs:153       deserialisation contract test
codelet/tools/src/profile/tests.rs:314             round-trip contract test
codelet/napi/src/agent_manager_handler.rs:70       async routing match
codelet/napi/src/agent_manager_handler.rs:788      handle_profile dispatch
```

The new `focus: Option<String>` parameter must be added to **all** of these
sites. The JSON schema (`agent_manager/mod.rs:155-170`) documents it to the LLM;
the Profile enum variant carries it; the async dispatch forwards it to
`ProfileSession::run()`; the handler passes it along.

## 4. Cargo profile (root cause of missing symbols)

```
codelet/Cargo.toml:181
[profile.release]
lto = "fat"
strip = "symbols"        # ← kills backtrace-rs symbolisation
codegen-units = 1
```

Change to:

```
[profile.release]
lto = "fat"
strip = "none"
debug = 1                      # line-table only, no full debug info
split-debuginfo = "packed"     # emit sibling .dSYM on macOS
codegen-units = 1
```

`debug = 1` emits line tables without the full DWARF blob, keeping the binary
size reasonable. `split-debuginfo = "packed"` on macOS puts the DWARF in
`.dSYM/` sidecars so the `.node` itself stays lean.

## 5. Existing test file (AMGR-017)

```
codelet/tools/src/profile/tests.rs    472 lines, 9 scenarios (AMGR-017)
```

Tests are structured with `#[serial]` + `#[tokio::test]` and use 1-second
windows instead of 10 to keep the suite fast. New AMGR-018 tests will follow
the same conventions but will primarily drive pure attribution functions
with synthetic inputs (no real pprof profile run needed for scenarios 2-7).

## 6. Testability refactor

The current `run_pprof_window` mixes two concerns:
1. **Side effects**: start pprof guard, sleep, build report
2. **Pure aggregation**: walk frames, filter noise, group by symbol

Extract the pure aggregation into a new module:

```
codelet/tools/src/profile/attribution.rs  (new file)
    pub struct SampleStack { thread_name, thread_id, frames, count }
    pub struct FrameInfo { symbol, file, line }
    pub struct AttributionOutput { scopes_by_calls, scopes_by_self_ms,
                                   samples_by_thread, hot_stacks, sampling }
    pub fn attribute_samples(
        stacks: &[SampleStack],
        duration_secs: f64,
        sample_freq_hz: i32,
        top_n: usize,
        focus: Option<&str>,
    ) -> AttributionOutput
    pub const NOISE_FRAME_PREFIXES: &[&str] = &[...]
    pub fn is_noise_frame(symbol: &str) -> bool
```

`run_pprof_window` becomes a thin adapter that converts `pprof::Report` into
`Vec<SampleStack>` and calls `attribute_samples`. All 7 scenarios in the
feature file can then be unit-tested against `attribute_samples` with plain
synthetic data — no pprof runtime needed.

## 7. New fields on ProfileResult

```rust
pub struct ThreadSampleReport {
    pub thread_name: String,
    pub thread_id: u64,
    pub sample_count: u64,
    pub cpu_ms: f64,
}

pub struct StackFrameInfo {
    pub symbol: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

pub struct StackReport {
    pub frames: Vec<StackFrameInfo>,    // truncated to first 6 meaningful frames
    pub thread_name: String,
    pub sample_count: u64,
}

pub struct SamplingReport {
    pub total_samples: u64,
    pub resolved_rust_samples: u64,
    pub unresolved_samples: u64,
    pub cpu_cores_consumed: f64,
    pub debug_info_available: bool,
    pub hint: Option<String>,
}

pub struct ProfileResult {
    // existing fields unchanged
    pub samples_by_thread: Vec<ThreadSampleReport>,
    pub hot_stacks: Vec<StackReport>,
    pub sampling: SamplingReport,
}
```

All new types `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]` for
consistency with existing report types.
