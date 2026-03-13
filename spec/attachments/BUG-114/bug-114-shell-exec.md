# BUG-114: Codex agent missing shell and exec_command alternative exec tools

## Problem

The Codex CLI native tool set includes two additional execution tools beyond `shell_command`. Both are missing from the Codex facade.

## Codex CLI Native Spec

### `shell` — raw exec without shell interpretation

From `codex-rs/core/src/tools/spec.rs`:

```
name: "shell"
params:
  - command: Array<String> (required) - passed to execvp()
  - workdir: String - working directory
  - timeout_ms: Number - timeout in milliseconds
  - sandbox_permissions: String - escalation control
  - justification: String - approval justification
  - prefix_rule: Array<String> - permission prefix pattern
```

Key difference from `shell_command`: the `command` parameter is an **array of strings** passed directly to `execvp()`, bypassing shell interpretation. This avoids shell injection risks and is used when the model wants to run a specific binary with exact arguments.

### `exec_command` — unified exec with PTY support

From `codex-rs/core/src/tools/spec.rs`:

```
name: "exec_command"
params:
  - cmd: String (required) - command to execute
  - workdir: String - working directory
  - shell: String - shell to use (e.g., "/bin/bash")
  - tty: Boolean - whether to allocate a PTY
  - yield_time_ms: Number - how long to wait for output
  - max_output_tokens: Number - cap on output tokens
  - login: Boolean - login shell semantics
  - sandbox_permissions: String
  - justification: String
  - prefix_rule: Array<String>
```

Key feature: PTY support via `tty: true`, and output capping via `max_output_tokens`. This is the most flexible execution tool in the Codex spec.

## Current State

Neither `shell` nor `exec_command` exist in:
- `codelet/tools/src/facade/codex.rs`
- `codelet/providers/src/codex/mod.rs`

Only `shell_command` (mapped from `BashTool`) is available.

## Impact

- Model cannot use `shell` for safe argument-passing without shell interpretation
- Model cannot use `exec_command` for PTY-based interactive commands
- Model may attempt to call these tools and receive unknown tool errors

## Recommended Fix

### `shell`
Create a tool that:
1. Takes `command` as `Vec<String>`
2. Executes via `tokio::process::Command::new(command[0]).args(&command[1..])`
3. Does NOT invoke through a shell
4. Maps `workdir` to the process working directory
5. Maps `timeout_ms` to a timeout

### `exec_command`
Create a tool that:
1. Takes `cmd` as a string
2. Optionally uses a specified `shell`
3. Supports `tty: true` for PTY allocation (may require a PTY library)
4. Supports `max_output_tokens` to cap output
5. Supports `yield_time_ms` to wait for output before returning

Both should be registered in `CodexProvider::create_rig_agent()`.

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- BashTool implementation: `codelet/tools/src/bash.rs`
