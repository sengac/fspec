# Research: `background_session_shape` nested napi build exhausts disk

Target: `cargo test --profile ci-test -p codelet-sessions --test background_session_shape`
Failing subtest: `subprocess::scenario_codelet_napi_still_builds_against_the_re_exported_background_session`
(`rust/sessions/tests/background_session_shape/subprocess.rs:53`).

## What happens
The subtest spawns `cargo build -p codelet-napi --manifest-path <workspace>/Cargo.toml`
from inside a `ci-test`-profile test binary. That nested build uses the **dev** profile, so a
second full copy of the workspace dependency graph (lance, datafusion, arrow, tantivy, napi…)
is compiled into `target/debug` next to the `target/ci-test` copy the test itself needed.

Observed 2026-10-30 on this 460 GB / ~20 GB-free machine:
- run 1: nested build failed with `No space left on device (os error 28)`;
- run 2 (from a clean target, 20 GB free): ci-test build + nested debug build exceeded free
  space again; the subtest failed while `cargo build -p codelet-napi` run directly succeeded
  (`Finished dev profile`).
Each run takes 3–6.5 minutes. The real assertion (napi compiles against the re-exported
`BackgroundSession`) is valid and caught a genuine BUG-284 regression
(`napi/src/agent_manager_handler.rs` missing `IncomingMessage.submission_id`).

Related: BUG-147 (stale `bug146_napi_attribute_scoping`) also shells out to nested napi
builds and fails for environmental reasons.

## Suggested fix
- Run the nested command as `cargo check -p codelet-napi` (type-checks the re-export, no
  codegen/linking, far smaller), and/or
- pass `--profile ci-test` (or inherit `CARGO_TARGET_DIR`/profile from the parent) so the
  nested build reuses already-built artifacts instead of a second profile, and
- surface the child's stderr tail in the assertion message (currently only "Blocking waiting
  for file lock" lines are visible) so real compile errors are distinguishable from ENOSPC.
