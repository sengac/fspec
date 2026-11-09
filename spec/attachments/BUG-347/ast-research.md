# AST research: BUG-347
- Failing tests: `rust/tools/tests/apply_patch_tool_test.rs` (around lines 157 and 209), `core_file_tools_test.rs:353`, `read_image_dimension_validation_test.rs:250`, `read_multimodal_test.rs:111`, and `rlcd001_service_supervisor.rs`.
- Next step: find where the RLCD security check gets its endpoint and pidfile path (RLCD-001 service supervisor, RLCD-004 security layer), and see how the tests could point it at a temp `HOME`/config directory with a helper from `rust/test-helpers/`.
