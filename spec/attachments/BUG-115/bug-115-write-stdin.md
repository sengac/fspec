# BUG-115: Codex agent missing write_stdin interactive session input tool

## Problem

The Codex CLI native tool set includes a `write_stdin` tool for sending input to interactive shell sessions (e.g., responding to prompts, entering passwords, providing stdin data). This tool is missing from the Codex facade.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "write_stdin"
params:
  - session_id: Number (required) - ID of the running shell session
  - chars: String - characters to write to stdin
  - yield_time_ms: Number - how long to wait for output after writing
  - max_output_tokens: Number - cap on output tokens to return
```

This tool works in conjunction with `exec_command` (which can create persistent sessions) or `shell_command` (which may leave processes running). The `session_id` refers to a process session that was started by a previous exec/shell tool call.

## Current State

No `write_stdin` tool exists in:
- `codelet/tools/src/facade/codex.rs`
- `codelet/providers/src/codex/mod.rs`

There is no session tracking mechanism for running shell processes in the current `BashTool` implementation — each `BashTool::call()` spawns and completes a process in one call.

## Impact

- Model cannot interact with interactive processes that require stdin input
- Model cannot respond to prompts from long-running commands
- Model may attempt to call `write_stdin` and receive an unknown tool error

## Recommended Fix

This is a more complex feature that requires:

1. **Session tracking**: A mechanism to keep shell processes alive across tool calls, indexed by session ID
2. **stdin writing**: Write characters to the process's stdin pipe
3. **Output capture**: Wait `yield_time_ms` for output and return up to `max_output_tokens`

### Minimal approach
Accept the tool call, return an error explaining that interactive sessions are not supported, and suggest using non-interactive alternatives. This prevents unknown tool errors while being honest about limitations.

### Full approach
Implement process session tracking with stdin/stdout pipes. This would require significant changes to the `BashTool` architecture.

Register in `CodexProvider::create_rig_agent()`.

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- BashTool implementation: `codelet/tools/src/bash.rs`
