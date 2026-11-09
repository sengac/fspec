# BUG-365 AST research

## fspec astgrep --pattern 'libc::kill($$$A)' --lang rust --path rust/tools/src
rust/tools/src/bash_process.rs:60:17:libc::kill(-(pgid as i32), libc::SIGKILL)
rust/tools/src/unified_exec/spawning.rs:160:29:libc::kill(-(pid as i32), libc::SIGKILL)
rust/tools/src/unified_exec/spawning.rs:565:30:libc::kill(pid as i32, libc::SIGKILL)

