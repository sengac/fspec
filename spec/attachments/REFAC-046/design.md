# REFAC-046 design: split unified_exec tool.rs

`rust/tools/src/unified_exec/tool.rs` is 529 lines; BUG-362 must change `handle_write`.

- `tool.rs` keeps the struct, its builder methods, the `rig::tool::Tool` impl and the shared polling functions
  (`poll_session`, `poll_session_interruptible`, ...).
- `tool/handlers.rs` (child module, `use super::*;`) receives the `impl UnifiedExecTool { handle_run, handle_write,
  handle_poll, handle_list, handle_close }` block (old lines 161-407) verbatim. Child modules see the parent's private
  imports, so nothing else changes. `mod handlers;` is appended at the end of `tool.rs`.
- Coverage: every implMapping into `tool.rs` is moved by the same line function (lines inside the block go to
  `tool/handlers.rs`, lines after it shift up), so mappings keep pointing at the same code they did before.

Verification: the codelet-tools unified_exec tests and clippy of the crate.
