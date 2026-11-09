# BUG-382 root cause

## Symptom
`cd rust && cargo fmt --all --check` lists 80 files with formatting diffs at 5b4d411e2:
agent-loop 2, cli 7, common 1, core 1, fspec-core 3, fspec-tui 24, providers 5, rpc 2, rpc-server 9, rpc-types 1,
sessions 20, tools 5.

## Cause
No step of the documented quality gate (AGENTS.md "Quality Check Integration": check, clippy, test) runs
`cargo fmt --check`, and agents format only the files they touch (crate-wide `cargo fmt` is avoided to keep diffs
scoped). Files edited without formatting are never caught.

## Fix direction
1. Format the 80 files in batches per crate (pure formatting commits, no behaviour change).
2. Add a format gate: a test or `fspec` audit check that fails when `cargo fmt --all --check` reports a diff, and
   document it in AGENTS.md "Quality Check Integration".
