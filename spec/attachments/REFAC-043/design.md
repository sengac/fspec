# REFAC-043 design

TUI-116 turns the exec-stdin composer overlay into a one-line notice plus a separate answer box, so the BUG-171 push-trigger tests in `rust/fspec-tui/tests/exec_stdin_push_chunk_bug171.rs` must change. That file is 355 lines, over the 300-line limit, so it is split first. This is a pure move:
- The helpers `sid`, `exec_request`, `key_event`, `fresh_app`, `render_app_frame`, `drain_pending`, `wait_until` and `exec_stdin_slot` move unchanged (made `pub`) into `tests/exec_stdin_push_chunk_bug171_support/mod.rs`. That is a directory module, so cargo does not build it as a separate test target. The root declares `mod exec_stdin_push_chunk_bug171_support;` and glob-imports it.
- The five test functions and every `@step` comment stay in the root file, byte for byte. It is still the feature's only test file. Result: root 274 lines, support 89 lines.
- The root keeps its existing `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` line, because this is a pure move. That suppression, and the `let _ = handle.await` in `drain_pending`, are filed as their own cards and are not changed here.
- Coverage: each scenario's test mapping is unlinked and relinked at the moved function's full range (fn line to closing brace), and its impl mappings are re-attached. The old ranges had drifted, for example 114-160 for a function that ended at line 170.
