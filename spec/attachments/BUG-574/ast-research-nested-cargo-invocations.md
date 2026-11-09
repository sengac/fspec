# AST research BUG-574: nested cargo invocations in tests
```
rust/sessions/tests/rpc076_session_manager_handle_imports_shape.rs:242:18:Command::new(env!("CARGO"))
rust/sessions/tests/rpc076_session_manager_handle_imports_shape.rs:352:18:Command::new(env!("CARGO"))
rust/sessions/tests/skeleton_invariants.rs:51:18:Command::new(env!("CARGO"))
rust/sessions/tests/skeleton_invariants.rs:101:18:Command::new(env!("CARGO"))
rust/sessions/tests/skeleton_invariants.rs:146:18:Command::new(env!("CARGO"))
rust/sessions/tests/skeleton_invariants.rs:262:18:Command::new(env!("CARGO"))
rust/sessions/tests/skeleton_invariants.rs:363:18:Command::new(env!("CARGO"))
rust/sessions/tests/session_manager_shape/codelet_sessions_builds_standalone.rs:46:18:Command::new(env!("CARGO"))
rust/sessions/tests/session_manager_shape/codelet_sessions_tests_assert.rs:20:18:Command::new(env!("CARGO"))
rust/sessions/tests/session_manager_shape/session_manager_gains_four.rs:108:18:Command::new(env!("CARGO"))
rust/sessions/tests/rpc075_scheduler_format_args_shape.rs:216:18:Command::new(env!("CARGO"))
rust/sessions/tests/background_session_shape/subprocess.rs:8:5:Command::new(env!("CARGO"))
rust/sessions/tests/background_session_shape/subprocess.rs:52:18:Command::new(env!("CARGO"))
rust/sessions/tests/mcp_injection_source_shape/consumer_and_surface.rs:124:18:Command::new(env!("CARGO"))
rust/fspec/tests/refac027_napi_removed.rs:20:18:Command::new(env!("CARGO"))
rust/graph/tests/codelet_graph_crate_lift.rs:38:18:Command::new(env!("CARGO"))
rust/graph/tests/codelet_graph_crate_lift.rs:119:18:Command::new(env!("CARGO"))
rust/graph/tests/codelet_graph_crate_lift.rs:183:17:Command::new(env!("CARGO"))
rust/graph/tests/codelet_graph_crate_lift.rs:192:16:Command::new(env!("CARGO"))
rust/graph/tests/codelet_graph_crate_lift.rs:230:18:Command::new(env!("CARGO"))
```

Findings (bug317 predicate over rust/*/tests/**/*.rs): graph/tests/codelet_graph_crate_lift.rs check/test/clippy and sessions/tests/rpc075_scheduler_format_args_shape.rs clippy lack --profile; fspec/tests/cargo_shape/cargo_build_p_fspec.rs uses Command::new("cargo") test twice without it. bug317_nested_build_profile only checks a fixed list of four files and ignores check.
