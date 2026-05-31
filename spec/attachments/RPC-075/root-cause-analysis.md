# RPC-075 — `skeleton_invariants` clippy fails on `uninlined_format_args`

## Summary

`codelet/sessions/tests/skeleton_invariants.rs::scenario_workspace_lints_are_inherited_and_clippy_passes`
runs `cargo clippy -p codelet-sessions` and asserts zero warnings/errors.
It currently FAILS because the workspace clippy lint set treats
`clippy::uninlined_format_args` as deny-by-default, and `codelet-core`
contains 5 violations in the scheduler module.

This blocks the `skeleton_invariants` test suite from going green on
the `codelet-integration` branch and creates a persistent "pre-existing
failure" that every subsequent card has had to acknowledge and skip.

## Failing lint output (verbatim from `cargo test -p codelet-sessions --test skeleton_invariants`)

```
error: variables can be used directly in the `format!` string
  --> core/src/scheduler/agent_job.rs:57:24
   |
57 |     let session_name = format!("[scheduled] {} — {}", name, timestamp);
   |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: requested on the command line with `-D clippy::uninlined-format-args`
help: change this to
   |
57 -     let session_name = format!("[scheduled] {} — {}", name, timestamp);
57 +     let session_name = format!("[scheduled] {name} — {timestamp}");

error: variables can be used directly in the `format!` string
  --> core/src/scheduler/agent_job.rs:77:22
   |
77 |         .map_err(|e| anyhow::anyhow!("Failed to spawn scheduled session for '{}': {}", name, e))?;
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: variables can be used directly in the `format!` string
  --> core/src/scheduler/shell_job.rs:34:24
   |
34 |         .ok_or_else(|| anyhow!("Schedule '{}': missing shell configuration", name))?;

error: variables can be used directly in the `format!` string
  --> core/src/scheduler/shell_job.rs:39:20
   |
39 |           return Err(anyhow!(
40 |             "Schedule '{}': shell command is empty",
41 |             name
42 |         ));

error: variables can be used directly in the `format!` string
  --> core/src/scheduler/shell_job.rs:57:22
   |
57 |         .map_err(|e| anyhow!("Schedule '{}': failed to spawn shell: {}", name, e))?;

error: could not compile `codelet-core` (lib) due to 5 previous errors

thread 'scenario_workspace_lints_are_inherited_and_clippy_passes' (688383)
  panicked at sessions/tests/skeleton_invariants.rs:367:5:
cargo clippy -p codelet-sessions failed: ...
```

## Root cause

The 5 `format!` / `anyhow!` calls in `codelet-core/src/scheduler/{agent_job,shell_job}.rs`
use the legacy positional-argument form. Rust 1.95's clippy promotes
`uninlined_format_args` from a `style` lint to a default-warn lint, and
the workspace clippy config in `codelet/Cargo.toml` (or
`codelet/.cargo/config.toml`) escalates it further to `deny`.

These were introduced by **RPC-058** (`codelet-scheduler` engine lift)
which moved the scheduler code from `codelet-napi` into `codelet-core`.
The original `napi`-side code may have been written before clippy 1.95
landed in the toolchain, or under a `napi`-crate-level lint config that
allowed the legacy form. The new home in `codelet-core` inherits the
strict workspace lints and the violations surface.

## Affected files (full list)

| File | Lines | Count |
|---|---|---|
| `codelet/core/src/scheduler/agent_job.rs` | 57, 77 | 2 |
| `codelet/core/src/scheduler/shell_job.rs` | 34, 39-42, 57 | 3 |

## Fix

Pure mechanical refactor — replace each positional `{}` placeholder
with an inline-capture `{name}` / `{timestamp}` / `{e}` form. Zero
behavioural change.

### `agent_job.rs:57`

```rust
// Before
let session_name = format!("[scheduled] {} — {}", name, timestamp);
// After
let session_name = format!("[scheduled] {name} — {timestamp}");
```

### `agent_job.rs:77`

```rust
// Before
.map_err(|e| anyhow::anyhow!("Failed to spawn scheduled session for '{}': {}", name, e))?;
// After
.map_err(|e| anyhow::anyhow!("Failed to spawn scheduled session for '{name}': {e}"))?;
```

### `shell_job.rs:34`

```rust
// Before
.ok_or_else(|| anyhow!("Schedule '{}': missing shell configuration", name))?;
// After
.ok_or_else(|| anyhow!("Schedule '{name}': missing shell configuration"))?;
```

### `shell_job.rs:39-42`

```rust
// Before
return Err(anyhow!(
    "Schedule '{}': shell command is empty",
    name
));
// After
return Err(anyhow!("Schedule '{name}': shell command is empty"));
```

### `shell_job.rs:57`

```rust
// Before
.map_err(|e| anyhow!("Schedule '{}': failed to spawn shell: {}", name, e))?;
// After
.map_err(|e| anyhow!("Schedule '{name}': failed to spawn shell: {e}"))?;
```

## Verification

1. `cd codelet && cargo clippy -p codelet-core --all-targets -- -D warnings`
   → should compile cleanly with zero warnings
2. `cd codelet && cargo test -p codelet-sessions --test skeleton_invariants`
   → `scenario_workspace_lints_are_inherited_and_clippy_passes` passes
3. `cd codelet && cargo build --release` → still builds (smoke test)
4. `cd codelet && cargo test -p codelet-core --lib scheduler` → existing
   scheduler tests still pass (behavioural-parity check)

## Why this is its own card (not in RPC-073 or RPC-074)

- It is in `codelet-core` (different crate, different feature file
  ownership) than RPC-073 (which lives in `codelet-sessions` +
  `codelet-fspec-tui` + `codelet-fspec`) and RPC-074 (which lives in
  `codelet-fspec-tui` + `codelet-core::session_manager_handle`).
- The lint violations were introduced by RPC-058, not by the bug
  cards that surfaced the failure.
- Bundling it into another card would violate the
  "no scope creep" rule the user has consistently enforced on every
  prior bug card.

## Source-shape regression

Add a regression test (or extend `skeleton_invariants` itself) to
assert that `cargo clippy -p codelet-core --all-targets` succeeds with
`-D warnings`. The existing `skeleton_invariants` test already does
this for `codelet-sessions`; extending the same pattern to
`codelet-core` would catch future regressions of the same shape in the
scheduler module.

## Risk / out-of-scope

- This card does NOT change any runtime behaviour — `format!` output
  is byte-identical with both forms.
- This card does NOT touch the workspace lint config — the lint stays
  at `deny` (correct behaviour for a strict-typed Rust port).
- If any other crate in the workspace has the same lint violation, it
  should be filed as a sibling card, not lumped in here.
