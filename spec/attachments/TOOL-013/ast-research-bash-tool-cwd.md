# AST Research: Bash Tool cwd Parameter

## Target Files
- `codelet/tools/src/bash.rs`

## Key Structures Found

### BashArgs Struct (line 551)
```rust
pub struct BashArgs {
    /// The bash command to execute
    pub command: String,
}
```
**Modification needed:** Add `cwd: Option<String>` field

### spawn_command Function (line 286)
```rust
fn spawn_command(command: &str) -> Result<tokio::process::Child, ToolError> {
    // Creates Command with sh -c, sets up stdio, process group
}
```
**Modification needed:** Accept `cwd: Option<&str>` param, call `.current_dir(cwd)` when Some

## Integration Points

1. `call()` method (line 573) - uses spawn_command
2. `call_with_streaming()` method (line 473) - uses spawn_command

Both methods need to pass `args.cwd` to the modified `spawn_command()`.

## Implementation Plan

1. Add `cwd: Option<String>` to `BashArgs` with description
2. Modify `spawn_command(command: &str)` → `spawn_command(command: &str, cwd: Option<&str>)`
3. In spawn_command, validate cwd exists if Some, then call `.current_dir()`
4. Update all call sites to pass cwd parameter
