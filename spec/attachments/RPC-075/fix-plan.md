# RPC-075 — Fix plan

## Approach

Pure mechanical replacement of 5 positional-arg `format!` / `anyhow!`
invocations with inline-capture syntax. No behavioural change. No
public API change. No test rewrite needed beyond verifying that the
existing scheduler tests still pass.

## Change 1 — `codelet/core/src/scheduler/agent_job.rs:57`

```diff
-    let session_name = format!("[scheduled] {} — {}", name, timestamp);
+    let session_name = format!("[scheduled] {name} — {timestamp}");
```

## Change 2 — `codelet/core/src/scheduler/agent_job.rs:77`

```diff
-        .map_err(|e| anyhow::anyhow!("Failed to spawn scheduled session for '{}': {}", name, e))?;
+        .map_err(|e| anyhow::anyhow!("Failed to spawn scheduled session for '{name}': {e}"))?;
```

## Change 3 — `codelet/core/src/scheduler/shell_job.rs:34`

```diff
-        .ok_or_else(|| anyhow!("Schedule '{}': missing shell configuration", name))?;
+        .ok_or_else(|| anyhow!("Schedule '{name}': missing shell configuration"))?;
```

## Change 4 — `codelet/core/src/scheduler/shell_job.rs:39-42`

```diff
-        return Err(anyhow!(
-            "Schedule '{}': shell command is empty",
-            name
-        ));
+        return Err(anyhow!("Schedule '{name}': shell command is empty"));
```

## Change 5 — `codelet/core/src/scheduler/shell_job.rs:57`

```diff
-        .map_err(|e| anyhow!("Schedule '{}': failed to spawn shell: {}", name, e))?;
+        .map_err(|e| anyhow!("Schedule '{name}': failed to spawn shell: {e}"))?;
```

## Verification

1. `cd codelet && cargo clippy -p codelet-core --all-targets -- -D warnings 2>&1 | tee /tmp/clippy-core.txt`
   - Expect: no warnings, exit 0
2. `cd codelet && cargo test -p codelet-sessions --test skeleton_invariants 2>&1 | tee /tmp/skeleton.txt`
   - Expect: all 6 tests pass (was 5/6 with the clippy one failing)
3. `cd codelet && cargo test -p codelet-core --lib scheduler 2>&1 | tee /tmp/core-scheduler.txt`
   - Expect: existing scheduler unit/integration tests unchanged
4. `cd codelet && cargo build --release 2>&1 | tee /tmp/build-release.txt`
   - Expect: clean release build

## Feature file

Suggested name (capability-based, not card-based):
`spec/features/scheduler-format-args-clippy-compliance.feature`

Scenarios:

1. `cargo clippy -p codelet-core` succeeds with zero
   `uninlined_format_args` warnings
2. `skeleton_invariants::scenario_workspace_lints_are_inherited_and_clippy_passes`
   passes after the fix
3. `format!` output is byte-identical to the legacy positional form
   (round-trip parity test against fixed name/timestamp inputs)
4. Source-shape regression: `grep -RIn '"[^"]*{}[^"]*", [a-z_]\+,' codelet/core/src/scheduler/`
   returns zero matches (no legacy positional-arg format calls)

## Source-shape regression test

Add `codelet/sessions/tests/rpc075_scheduler_format_args_shape.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(p: &str) -> String {
    let path = workspace_root().join(p);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn scheduler_agent_job_uses_inline_format_args() {
    let body = read("core/src/scheduler/agent_job.rs");
    // No legacy positional format calls of the form
    // `format!("...{}...", var)` or `anyhow!("...{}...", var)` should remain.
    // We assert by checking that every `format!` / `anyhow!` invocation
    // in the file has at most one literal-string argument (no trailing
    // positional args).
    let suspicious = body
        .lines()
        .filter(|l| {
            (l.contains("format!(") || l.contains("anyhow!(")) && l.contains("{}")
        })
        .collect::<Vec<_>>();
    assert!(
        suspicious.is_empty(),
        "agent_job.rs still contains legacy positional `{{}}` format args — \
         RPC-075 requires inline-capture form. Offenders: {suspicious:#?}"
    );
}

#[test]
fn scheduler_shell_job_uses_inline_format_args() {
    let body = read("core/src/scheduler/shell_job.rs");
    let suspicious = body
        .lines()
        .filter(|l| {
            (l.contains("format!(") || l.contains("anyhow!(")) && l.contains("{}")
        })
        .collect::<Vec<_>>();
    assert!(
        suspicious.is_empty(),
        "shell_job.rs still contains legacy positional `{{}}` format args — \
         RPC-075 requires inline-capture form. Offenders: {suspicious:#?}"
    );
}
```

This will catch any future re-introduction of the same lint shape in
the scheduler module.

## Risk / out-of-scope

- Does NOT touch any other crate. If `codelet-providers` /
  `codelet-tools` / etc. have the same lint shape, file separately.
- Does NOT change the workspace lint level — strict-deny stays
  (this is the correct posture for a Rust port).
- Does NOT alter scheduler semantics — output strings remain byte-
  identical.
