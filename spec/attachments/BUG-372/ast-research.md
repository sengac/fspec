# BUG-372 AST research

## fspec astgrep --pattern 'read_tree_or_file($A)' --lang rust --path rust
rust/rpc-server/tests/regression_invariants_rpc011.rs:122:    let body = codelet_test_helpers::source_composition::read_tree_or_file(&arch_rs)
rust/agent-loop/tests/cmpct041_pre_compaction_basis_test.rs:38:    codelet_test_helpers::source_composition::read_tree_or_file(&path)
rust/agent-loop/tests/rpc085_thinking_config.rs:38:    codelet_test_helpers::source_composition::read_tree_or_file(&path)
rust/agent-loop/tests/rpc072_work_agent_roundtrip.rs:222:    let src = codelet_test_helpers::source_composition::read_tree_or_file(&common_rs)
rust/fspec-core/tests/init.rs:283:    let core_src = read_tree_or_file(&core_path).expect("init.rs module tree readable");
rust/cli/tests/cmpct050_orphan_tool_call_recovery.rs:183:    codelet_test_helpers::source_composition::read_tree_or_file(&path)
rust/sessions/tests/rpc076_session_manager_handle_imports_shape.rs:48:    codelet_test_helpers::source_composition::read_tree_or_file(path)
rust/sessions/tests/rpc073_cloud_model_catalog.rs:191:    let src = codelet_test_helpers::source_composition::read_tree_or_file(&path)
rust/sessions/tests/bug154_set_owning_manager_in_create_session_with_id.rs:23:    codelet_test_helpers::source_composition::read_tree_or_file(path)
rust/sessions/tests/skeleton_invariants.rs:28:    codelet_test_helpers::source_composition::read_tree_or_file(path)
rust/sessions/tests/prov144_max_images_wiring_shape.rs:40:    codelet_test_helpers::source_composition::read_tree_or_file(path)
rust/sessions/tests/rpc077_handle_impl_redundant_clone_shape.rs:30:    codelet_test_helpers::source_composition::read_tree_or_file(&path)
(astgrep timed out on the whole rust tree, so these lines are from grep -rn)
