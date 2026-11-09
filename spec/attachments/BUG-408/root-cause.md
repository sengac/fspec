# BUG-408 root cause

Split from BUG-395 (which fixed the pause and question writers).

- `rust/fspec-tui/src/app/dispatch_pause_hitl.rs` `App::handle_pause_chunk` and
  `rust/fspec-tui/src/app/dispatch_exec_stdin.rs` `App::probe_exec_stdin_for` start with
  `if tokio::runtime::Handle::try_current().is_err() { return; }` (commented "Sync unit-test fallback"): the probe is
  skipped with no log, so a paused session shows no prompt and nothing says why.
- `App::handle_exec_stdin_submit` has the same silent return, but the key handler has already moved the typed text
  out of the answer box, so the answer is lost.

Fix: the submit reports through `handle_exec_stdin_write_failed(.., "no tokio runtime: write_exec_stdin not sent")`,
which puts the text back with a red "Not sent" row; the probes log at error level (and the pause probe can show the
same red notice as `handle_paused_read_failed`). Many sync unit tests dispatch Paused chunks without a runtime, so
check which of them assert on the scrollback before adding a notice. Tests need a sync (`#[test]`) harness with an
open exec-stdin request (see tests/tui116_stdin_notice_answer_box_support).
