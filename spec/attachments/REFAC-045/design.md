# REFAC-045 design: split unified_exec spawning.rs

`rust/tools/src/unified_exec/spawning.rs` is 593 lines and BUG-362 must change both stdin forwarders in it.

## Split (no behavior change)
- `spawning.rs` (lines 1-188 today): module doc, `SpawnResult`, `SPAWN_TERM`, `OS`, and the shared helpers
  (`apply_non_pager_env`, `cap_output_buffer`, `spawn_reader_task`, `build_kill_handle`). It declares the child
  modules and re-exports `spawn_pipe_process` / `spawn_pty_process`, so every caller path stays
  `super::spawning::spawn_*_process`.
- `spawning/pipe.rs`: `spawn_pipe_process` (old lines 190-325), moved verbatim.
- `spawning/pty.rs`: `spawn_pty_process`, `kill_anchor_pid`, `build_pty_kill_handle` (old lines 327-593), verbatim.
- Child modules `use super::*;` so the private helpers and imports stay private to `spawning`.

## Verification
Behavior-preserving move, guarded by the existing unified_exec tests (`codelet-tools` `unified_exec` and
`exec_stdin_prompt_p1` targets) plus a compile and clippy of `codelet-tools`. Coverage mappings in
`unified-exec-tool.feature.coverage` and `windows-unified-exec-platform-shell.feature.coverage` that point at the moved
lines are re-pointed to the new files.
