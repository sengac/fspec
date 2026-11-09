# REFAC-049 AST research

- `grep -n "^#\[cfg(test)\]" rust/tools/src/unified_exec/exec_stdin.rs` -> 252; the module runs to the end of file.
- The tests use private items `LAST_FIRE_MS`, `cooldown_elapsed`, `record_fire`, `reset_cooldown`, `now_ms` via
  `use super::*`; a child module file keeps that access.
- `ls rust/tools/src/unified_exec/` -> `spawning.rs` + `spawning/` and `tool.rs` + `tool/` already use the
  file-plus-directory layout.
