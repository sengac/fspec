# BUG-418 AST research

- grep -n loop_abort_retry_count rust/agent-loop/src/agent_loop/*.rs: state.rs:7 (field), prompt.rs:93 (reset), finalization.rs:88-92 (cap check).
- read_tree_or_file precedent: rust/fspec/tests/handle_impl_source_support/mod.rs read_handle_impl_src.
