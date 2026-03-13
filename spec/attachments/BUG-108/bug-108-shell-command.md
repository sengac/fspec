# BUG-108: Codex shell_command facade ignores workdir and timeout_ms params

## Problem

The `CodexShellCommandFacade` exposes `workdir` and `timeout_ms` in its JSON schema but `map_params()` only extracts `command` and ignores both optional parameters.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "shell_command"
params:
  - command: String (required) - "The shell script to execute in the user's default shell"
  - workdir: String - "The working directory to execute the command in"
  - timeout_ms: Number - "The timeout for the command in milliseconds"
  - login: Boolean - "Whether to run the shell with login shell semantics. Defaults to true."
  - sandbox_permissions: String - escalation control
  - justification: String - approval justification
  - prefix_rule: Array<String> - permission prefix pattern
```

## Current Implementation

**File**: `codelet/tools/src/facade/codex.rs` (lines 72-111)

Schema includes `command`, `workdir`, `timeout_ms` — correct.

But `map_params()` only maps `command`:

```rust
fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError> {
    let command = extract_required_string(&input, "command", "shell_command")?;
    Ok(InternalBashParams::Execute { command })
}
```

The `workdir` value is silently dropped. When the model sends `{"command": "make test", "workdir": "/project"}`, the working directory is ignored.

## Missing from Schema

These Codex-native params are not even exposed in the schema:
- `login` (Boolean)
- `sandbox_permissions` (String)
- `justification` (String)
- `prefix_rule` (Array<String>)

## Recommended Fix

1. **workdir**: Map to the `cwd` field on `InternalBashParams` (or add a `cwd` field if `InternalBashParams::Execute` doesn't support it — check the `BashToolFacade` trait and `BashToolFacadeWrapper`)
2. **timeout_ms**: Map to an internal timeout parameter if the `BashTool` supports it
3. **login, sandbox_permissions, justification, prefix_rule**: These are Codex sandbox/approval features. Either add them to the schema (accepted but not mapped, for model compatibility) or implement them if the internal tool supports equivalent functionality.

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- Facade file: `codelet/tools/src/facade/codex.rs:72-111`
- BashTool implementation: `codelet/tools/src/bash.rs`
