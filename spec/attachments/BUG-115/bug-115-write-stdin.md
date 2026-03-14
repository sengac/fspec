# BUG-115: Codex facade maps write_stdin to unified exec tool

## Architecture

```
Codex LLM → write_stdin (Codex-native schema)
    ↓
CodexWriteStdinFacade (codelet/tools/src/facade/codex.rs)
    ↓
UnifiedExecTool write/poll action (codelet/tools/src/ — provider-agnostic, TOOL-016)
    ↓
ProcessStore lookup → stdin write → output poll
```

## Codex-Native Tool Schema to Map

### `write_stdin` → unified exec `write` or `poll` action

```json
{
  "name": "write_stdin",
  "parameters": {
    "session_id": { "type": "number", "description": "ID of running session from exec_command" },
    "chars": { "type": "string", "description": "Characters to write (empty = poll)" },
    "yield_time_ms": { "type": "number", "description": "Wait time for output" },
    "max_output_tokens": { "type": "number" }
  },
  "required": ["session_id"]
}
```

Facade mapping:
- `session_id` (Number) → unified exec `session_id` (String, e.g., "4237")
- `chars` → unified exec `input`
- Empty `chars` or missing → unified exec `poll` action instead of `write`
- `yield_time_ms` → unified exec `yield_time_ms`
- `max_output_tokens` → unified exec `max_output_tokens`

### Output Schema (shared with exec_command)

```json
{
  "session_id": "number — present when process is still running",
  "exit_code": "number — present when process has exited",
  "output": "string — new output since last read",
  "wall_time_seconds": "number",
  "chunk_id": "string",
  "original_token_count": "number"
}
```

## The Yield-and-Resume Lifecycle

This facade only makes sense in context of the full lifecycle:

1. **LLM calls `exec_command`** → unified exec spawns process, returns `session_id` if still alive
2. **LLM calls `write_stdin`** (this facade) → unified exec writes to stdin, polls output
3. **LLM calls `write_stdin` again** → repeat as needed
4. **Process exits** → response has `exit_code`, no `session_id` → LLM stops calling

The facade translates Codex's numeric `session_id` to the unified exec's string session IDs and maps `chars` to the `input` parameter.

## Empty Write = Poll

When `chars` is empty or absent, the facade should map to the `poll` action:
- Higher minimum yield time (5000ms vs 250ms for writes)
- Used to check if long-running processes have produced output or exited
- Codex's `MIN_EMPTY_YIELD_TIME_MS = 5000ms`

## VTCode Reference

VTCode handles this as the `"write"` and `"poll"` actions of its unified `unified_exec` tool:

```json
{"action": "write", "session_id": "run-4237", "input": "print('hello')\n", "yield_time_ms": 250}
{"action": "poll", "session_id": "run-4237", "yield_time_ms": 5000}
```

Provider-agnostic — available for all providers, not just Codex.

## References

- Codex CLI write_stdin spec: `/tmp/codex/codex-rs/core/src/tools/spec.rs` (`create_write_stdin_tool()`)
- Codex write_stdin handler: `/tmp/codex/codex-rs/core/src/tools/handlers/unified_exec.rs` (line 284–308)
- Codex process manager: `/tmp/codex/codex-rs/core/src/unified_exec/process_manager.rs`
- Existing Codex facade: `codelet/tools/src/facade/codex.rs`
- Unified exec tool: TOOL-016
