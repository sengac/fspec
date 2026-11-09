# REFAC-045 AST research

Top-level items of `rust/tools/src/unified_exec/spawning.rs` (grep `^pub fn|^fn|^pub type|^const`):
`SpawnResult` (21), `SPAWN_TERM` (31), `OS` (35), `apply_non_pager_env` (39), `cap_output_buffer` (46),
`spawn_reader_task` (63), `build_kill_handle` (147), `spawn_pipe_process` (191), `spawn_pty_process` (341),
`kill_anchor_pid` (557), `build_pty_kill_handle` (580).

Callers of the two spawn functions go through `super::spawning::` (tool.rs), so re-exports keep them unchanged.
`kill_anchor_pid` and `build_pty_kill_handle` are only used by the PTY path; the reader task, kill handle, env and
cap helpers are shared. Coverage files mapping `unified_exec/spawning.rs`: unified-exec-tool,
windows-unified-exec-platform-shell.
