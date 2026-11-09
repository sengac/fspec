# REFAC-049 design

`rust/tools/src/unified_exec/exec_stdin.rs` is 389 lines. BUG-376 changes the detector and adds tests there, so the
`#[cfg(test)] mod tests` block (lines 252-389) moves verbatim to `unified_exec/exec_stdin/tests.rs`, declared as
`#[cfg(test)] mod tests;` (the same layout as `spawning.rs` + `spawning/`). The production code stays in
`exec_stdin.rs` (about 250 lines). No behaviour change; the tests still reach private items through `use super::*`.
Verification: `cargo test -p codelet-tools --lib exec_stdin` and clippy on the lib.
