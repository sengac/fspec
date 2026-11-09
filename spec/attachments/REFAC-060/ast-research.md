# REFAC-060 AST research

Tests at HEAD: detector_fire 104, detector_clear_on_output 235, detector_clear_on_child 308, detector_clear_on_session_removal 376, non_exit_detector_clear 439, continuous_quiet_period 527, detector_never_fired 601, clearing_exec_stdin_slot 664, exec_session_child_exit 713, successful_write_exec_stdin 791. Helpers 41-103 plus two async helpers at 184 and 205. exec_stdin_prompt_p2_support (REFAC-056) has near-identical data_dir/fresh_session/run_still_running/close_exec.

## Result

- exec-stdin-push-emission.feature keeps fire / clear-slot / child-exit / write (rules 1-2); tests in exec_stdin_push_chunk_bug171.rs (299 lines, drained_chunks stays there).
- exec-stdin-detector-clear.feature (rule 4): exec_stdin_detector_clear_bug171.rs (228 lines).
- exec-stdin-detector-cooldown.feature (rule 5 + never fired): exec_stdin_detector_cooldown_bug171.rs (250 lines).
- exec_stdin_push_chunk_bug171_support/mod.rs: data_dir, fresh_session, run_still_running, close_exec (all three binaries).
- exec_stdin_detector_bug171_support/mod.rs: wait_for_stored_request, wait_for_chunks (the two detector binaries only, so no dead code in the push binary).
- All test ranges were stale and three scenarios had no impl mapping; all ten were relinked: set_exec_stdin_request (background_session/exec_stdin.rs 4-36, plus the alive check 38-68 for child exit), write_exec_stdin (handle_impl_4.rs 67-112), spawn_exec_stdin_detector (tools unified_exec/exec_stdin.rs 194-285, plus cooldown_elapsed/record_fire 150-161 and reset_cooldown 163-173).
- Helper fallbacks noted on BUG-415 (close_exec `let _ =`, wait_for_stored_request silent timeout); the `let _ = set_data_directory` in data_dir is BUG-406.
