# REFAC-042 AST research

## fspec astgrep --pattern 'session_id_to_evict($$$A)' --lang rust --path rust/tools
rust/tools/tests/unified_exec_tool_test.rs:537:18:session_id_to_evict(&meta)
rust/tools/tests/unified_exec_tool_test.rs:576:18:session_id_to_evict(&meta)
rust/tools/src/unified_exec/process_store_eviction.rs:27:25:session_id_to_evict(&meta)

The test file imports it via the public re-export in `unified_exec/mod.rs`, which is kept.

## fspec astgrep --pattern 'global_store()' --lang rust --path rust/tools/src/unified_exec
exec_stdin.rs:189, owner_scope.rs:56, tool.rs:187/301/352/380/441, reaper.rs:41. All of them call methods on the store, not fields, so moving the method impls into sibling modules changes no call site.

## Internal paths
`super::process_store::{now_micros, ChildHandle, ProcessEntry, SessionInfo, global_store}` are used by spawning.rs, tool.rs, types.rs, exec_stdin.rs and reaper.rs. All of them stay in `process_store.rs`.
