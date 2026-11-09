# BUG-415 root cause

An unmapped test and a dropped Result in test support. Fix: add a scenario for the wire round trip (or remove the test) and make close_exec return/assert the close outcome.

## Same pattern in the BUG-171 harness (found during REFAC-060)

- rust/sessions/tests/exec_stdin_push_chunk_bug171_support/mod.rs `close_exec` drops the close action's
  Result with `let _ =`, exactly like exec_stdin_prompt_p2_support's copy. Fix both together.
- rust/sessions/tests/exec_stdin_detector_bug171_support/mod.rs `wait_for_stored_request` returns
  silently when the 10 s poll budget runs out instead of failing; the callers then assert on a
  different symptom. Return an error (or assert) on timeout.
