# BUG-375 AST research

- `grep -n 'async fn handle_' rust/tools/src/unified_exec/tool.rs` before REFAC-046: handle_run 167, handle_write 290,
  handle_poll 331, handle_list 351, handle_close 370, compared with the coverage ranges listed in root-cause.md.
- `awk` over `rust/tools/tests/exec_stdin_prompt_p4.rs`: `scenario_bash_delegation_surfaces_exec_stdin_prompt`
  31-104, `bash_delegation_preserves_one_shot_result_contract` 108-143, `bash_delegation_stdin_is_piped_and_writable`
  148-192.
- `fspec link-coverage` on a scenario that already maps a test file prints "Added second test mapping"
  (seen in TUI-116 on exec-stdin-push-tui-composer).
