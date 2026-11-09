# BUG-347 root cause (to confirm): codelet-tools tests depend on the host's RLCD service

Seen during the MCP-009 regression run (`cargo test --profile ci-test -p codelet-tools --no-fail-fast`) on a machine with an RLCD service running:

- `apply_patch_tool_test`: `test_delete_file_removes_from_disk` and `test_multi_file_patch_mixed_operations` fail on `.unwrap()` of `Err(Blocked { tool: "apply_patch", message: "Blocked: RLCD security: P(risk)=0.35..0.54 >= blockThreshold 0.35" })`.
- `core_file_tools_test::test_edit_tool_old_string_not_found`, `read_image_dimension_validation_test::test_read_tool_allows_corrupt_header_image_through` and `read_multimodal_test::test_handle_corrupted_image_gracefully` fail too, probably the same way.
- In `rlcd001_service_supervisor`, three scenarios fail: `scenario_live_pidfile_healthy_on_different_port_reanchors`, `scenario_all_consumers_read_shared_status_and_fail_open` and `scenario_all_ports_occupied_fails_open`.
- Which tests fail changes from run to run, because the risk score comes from a live model.

Hypothesis: the RLCD security layer (RLCD-004) finds the host service through the user's real config, pidfile or port. These tests do not redirect that to a temp dir, so the result depends on whatever service is running on the host. This breaks the "Redirect, don't intercept" rule in AGENTS.md.
