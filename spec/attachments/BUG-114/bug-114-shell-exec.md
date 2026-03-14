# BUG-114: Codex facade maps shell and exec_command to unified exec tool

## Architecture

```
Codex LLM → shell / exec_command (Codex-native schemas)
    ↓
CodexShellFacade / CodexExecCommandFacade (codelet/tools/src/facade/codex.rs)
    ↓
UnifiedExecTool (codelet/tools/src/ — provider-agnostic, TOOL-016)
    ↓
ProcessStore + PTY/pipe spawning
```

## Codex-Native Tool Schemas to Map

### `shell` → unified exec `run` action (no shell interpretation)

```json
{
  "name": "shell",
  "parameters": {
    "command": { "type": "array", "items": { "type": "string" }, "description": "argv passed to execvp()" },
    "workdir": { "type": "string" },
    "timeout_ms": { "type": "number" }
  },
  "required": ["command"]
}
```

Facade mapping:
- `command: ["ls", "-la"]` → unified exec `run` action with `command` as array, shell interpretation disabled
- `workdir` → unified exec `workdir`
- `timeout_ms` → unified exec `timeout_secs` (convert ms → s)

### `exec_command` → unified exec `run` action (with PTY support)

```json
{
  "name": "exec_command",
  "parameters": {
    "cmd": { "type": "string", "description": "Shell command to execute" },
    "workdir": { "type": "string" },
    "shell": { "type": "string", "description": "Shell binary to use" },
    "tty": { "type": "boolean", "description": "Allocate PTY", "default": false },
    "yield_time_ms": { "type": "number", "description": "Wait time before yielding" },
    "max_output_tokens": { "type": "number" },
    "login": { "type": "boolean", "description": "Login shell semantics" }
  },
  "required": ["cmd"]
}
```

Facade mapping:
- `cmd` → unified exec `command` (string form)
- `tty` → unified exec `tty`
- `yield_time_ms` → unified exec `yield_time_ms`
- `max_output_tokens` → unified exec `max_output_tokens`
- `workdir` → unified exec `workdir`

### Output Schema (shared by exec_command and write_stdin)

```json
{
  "session_id": "number — present when process is still running",
  "exit_code": "number — present when process has exited",
  "output": "string — command output, possibly truncated",
  "wall_time_seconds": "number",
  "chunk_id": "string",
  "original_token_count": "number"
}
```

## How exec_command Yield-and-Resume Works

See TOOL-016 for the full unified exec implementation. The Codex facade just translates parameter names:

1. LLM calls `exec_command({ cmd: "python3", tty: true, yield_time_ms: 5000 })`
2. Facade maps to unified exec run action
3. Unified exec spawns PTY, collects output for yield_time_ms
4. If process exits → response has `exit_code`, no `session_id`
5. If process still running → response has `session_id` for `write_stdin` follow-up

## VTCode Reference

VTCode (`/tmp/VTCode`) consolidates shell, exec_command, and all PTY tools into one `unified_exec` tool with action-based dispatch. All legacy Codex tool names (`exec_command`, `write_stdin`, `shell`, `run_pty_cmd`, etc.) are registered as aliases that route to the unified executor. This is **provider-agnostic** — available for Anthropic, OpenAI, Gemini, Ollama, LM Studio, DeepSeek, and all other providers.

Key files:
- `vtcode-core/src/tools/registry/builtins.rs` — unified_exec registration with all aliases
- `vtcode-core/src/tools/handlers/session_tool_catalog.rs` — unified_exec_parameters() schema
- `vtcode-config/src/constants/tools.rs` — tool name constants

## References

- Codex CLI tool spec: `/tmp/codex/codex-rs/core/src/tools/spec.rs`
- Codex unified_exec handler: `/tmp/codex/codex-rs/core/src/tools/handlers/unified_exec.rs`
- Codex process manager: `/tmp/codex/codex-rs/core/src/unified_exec/process_manager.rs`
- Existing Codex facade: `codelet/tools/src/facade/codex.rs`
- Unified exec tool: TOOL-016
