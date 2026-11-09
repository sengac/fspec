# BUG-393 AST research

`grep -rln "300" rust/*/tests/*.rs | xargs grep -l "read_tree_or_file\|read_module_tree"`:
- rust/agent-loop/tests/cmpct041_pre_compaction_basis_test.rs
- rust/fspec-tui/tests/source_shape_rpc014.rs, source_shape_rpc015.rs, source_shape_rpc016.rs, source_shape_rpc025.rs,
  source_shape_cargo.rs (source_shape_rpc054.rs fixed by BUG-389)
- rust/rpc-server/tests/health_rpc_serverstats_rpc011.rs
- rust/sessions/tests/wt012_isolated_session_shared_creation.rs

Each needs checking: which of its reads feed a line-count ceiling.
