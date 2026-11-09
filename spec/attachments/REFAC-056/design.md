# REFAC-056 design

BUG-399 must edit rust/sessions/tests/exec_stdin_prompt_p2.rs (337 lines). The helpers `DATA_DIR`, `data_dir`,
`fresh_session`, `wait_store_state`, `run_still_running` and `close_exec` move unchanged (made `pub`) into
`tests/exec_stdin_prompt_p2_support/mod.rs`. The five tests are unchanged; the test file is 271 lines and
exec-stdin-prompt.feature keeps one test file. Its four test mappings are remapped to the new ranges.
