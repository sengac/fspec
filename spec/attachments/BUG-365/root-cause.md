# BUG-365 root cause

The workspace denies `unsafe_code`, but the unified_exec and bash process killers opt out with `#[allow(unsafe_code)]` blocks around `libc::kill`:
- `rust/tools/src/unified_exec/spawning.rs` `build_kill_handle` (SIGKILL to the process group) and `kill_anchor_pid` (`let _ = unsafe { libc::kill(..) }`, which also drops the result).
- `rust/tools/src/bash_process.rs` (process group SIGKILL).

That is a lint suppression of a deny lint. Fix: signal through a safe API, such as the `nix` crate's `killpg`/`kill` or a `kill(1)` child process, report the result, and remove the allows.
