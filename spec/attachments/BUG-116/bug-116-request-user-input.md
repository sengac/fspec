# BUG-116: Codex agent missing request_user_input tool

## Problem

The Codex CLI native tool set includes a `request_user_input` tool for prompting the user for structured input during a task. This tool is missing from the Codex facade.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "request_user_input"
params:
  - purpose: String - explanation of why input is needed
  - items: Array - input field definitions for structured user prompts
```

This tool allows the model to pause execution and ask the user for specific information (e.g., "Which database should I connect to?", "What is your preferred naming convention?"). The Codex CLI renders these as interactive prompts.

## Current State

No `request_user_input` tool exists in:
- `codelet/tools/src/facade/codex.rs`
- `codelet/providers/src/codex/mod.rs`

## Impact

- Model cannot request structured user input during complex tasks
- Model may attempt to call this tool and receive an unknown tool error
- Model falls back to asking questions in plain text, which doesn't pause execution or provide structured response handling

## Recommended Fix

Create a `RequestUserInputTool` that:

1. Accepts `purpose` (String) and `items` (Array of input field definitions)
2. Delegates to the existing `PauseHandler` mechanism (`codelet/tools/src/tool_pause.rs`) to pause the agent and wait for user input
3. Returns the user's responses as structured JSON

### Integration with existing pause system

The codebase already has a `tool_pause` module with `PauseHandler`, `PauseRequest`, and `PauseResponse` types. The `request_user_input` tool could be implemented as a specialized pause:

```rust
pause_for_user(PauseRequest {
    kind: PauseKind::UserInput,
    purpose: args.purpose,
    items: args.items,
}).await
```

The TUI would render the input fields and return the user's responses.

Register in `CodexProvider::create_rig_agent()`.

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- Tool pause system: `codelet/tools/src/tool_pause.rs`
- PauseHandler types: `PauseKind`, `PauseRequest`, `PauseResponse`
