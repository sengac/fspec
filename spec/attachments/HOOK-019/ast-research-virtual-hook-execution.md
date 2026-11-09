# AST research HOOK-019: virtual hook execution
## pub fn run_for_transition($$$A) -> $R { $$$B }
```
rust/fspec-core/src/virtual_hooks_exec.rs:34:1:pub fn run_for_transition(
```
## pub(super) fn run_pre_hooks($$$A) -> $R { $$$B }
```
rust/fspec-core/src/commands/update_work_unit_status/board_state.rs:70:1:pub(super) fn run_pre_hooks(
```
## pub fn get_staged_files($$$A) -> $R { $$$B }
```
rust/git/src/status.rs:41:1:pub fn get_staged_files(dir: impl AsRef<Path>) -> Result<Vec<String>> {
```
## pub fn get_unstaged_files($$$A) -> $R { $$$B }
```
rust/git/src/status.rs:181:1:pub fn get_unstaged_files(dir: impl AsRef<Path>) -> Result<Vec<String>> {
```

Findings: run_for_transition (post-<new_status>) has no caller; run_pre_hooks runs pre-<new> via sh -c with no stdin context and ignores gitContext; codelet_git status helpers give staged/unstaged paths; command_hooks::execute_hook (HOOK-018) is the shared executor to reuse.
