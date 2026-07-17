# Case-Insensitive Tool Name Matching

## Problem Statement

LLMs frequently emit tool names with inconsistent casing. For example, an LLM might call `"Fspec"`, `"FSPEC"`, or `"fspec"` — all referring to the same tool. The current implementation performs a **case-sensitive** `HashMap` lookup, causing `ToolNotFoundError` when the casing doesn't match exactly.

This issue affects both the TypeScript agent loop and the Rust TUI port, as both rely on the same `ToolSet` implementation in the rig-core patch.

## Current Behavior

### Tool Registration

Tools are registered with their canonical name (typically lowercase):

```rust
impl Tool for Fspec {
    const NAME: &'static str = "fspec";
    // ...
}
```

The `ToolSet` stores tools in a `HashMap<String, ToolType>` keyed by the tool's name:

```rust
pub struct ToolSet {
    pub(crate) tools: HashMap<String, ToolType>,
}
```

### Tool Lookup (Current)

When an LLM calls a tool, the name is looked up directly in the HashMap:

```rust
// codelet/patches/rig-core/src/tool/mod.rs:475-485
pub async fn call(&self, toolname: &str, args: String) -> Result<String, ToolSetError> {
    if let Some(tool) = self.tools.get(toolname) {
        tracing::debug!(target: "rig",
            "Calling tool {toolname} with args:\n{}",
            serde_json::to_string_pretty(&args).unwrap()
        );
        Ok(tool.call(args).await?)
    } else {
        Err(ToolSetError::ToolNotFoundError(toolname.to_string()))
    }
}
```

This is a **case-sensitive** lookup. `"Fspec"` ≠ `"fspec"` in the HashMap.

### ToolServer Path

The `ToolServer` also uses case-sensitive lookup when handling `CallTool` requests:

```rust
// codelet/patches/rig-core/src/tool/server.rs:129-139
ToolServerRequestMessageKind::CallTool { name, args } => {
    match self.toolset.call(&name, args.clone()).await {
        Ok(result) => {
            let _ = callback_channel.send(ToolServerResponse::ToolExecuted { result });
        }
        Err(err) => {
            let _ = callback_channel.send(ToolServerResponse::ToolError {
                error: err.to_string(),
            });
        }
    }
}
```

### Tool Definitions Lookup

The `get_tool_definitions` method also uses case-sensitive lookup:

```rust
// codelet/patches/rig-core/src/tool/mod.rs:461
pub(crate) fn get(&self, toolname: &str) -> Option<&ToolType> {
    self.tools.get(toolname)
}
```

## Impact

### User-Facing Symptoms

1. **ToolNotFoundError**: When an LLM calls `"Fspec"` instead of `"fspec"`, the tool call fails with `ToolNotFoundError("Fspec")`.
2. **Agent Loop Failure**: The agent loop cannot execute the tool, causing the conversation to stall or produce an error response.
3. **Rust TUI Port**: The Rust TUI port also uses the same `ToolSet` implementation, so it would have the same issue.

### Affected Components

| Component | File | Method | Line |
|-----------|------|--------|------|
| ToolSet | `codelet/patches/rig-core/src/tool/mod.rs` | `call()` | 475-485 |
| ToolSet | `codelet/patches/rig-core/src/tool/mod.rs` | `get()` | 461 |
| ToolSet | `codelet/patches/rig-core/src/tool/mod.rs` | `contains()` | 437 |
| ToolSet | `codelet/patches/rig-core/src/tool/mod.rs` | `delete_tool()` | 452 |
| ToolServer | `codelet/patches/rig-core/src/tool/server.rs` | `handle_message()` | 129-139 |
| ToolServer | `codelet/patches/rig-core/src/tool/server.rs` | `get_tool_definitions()` | 156-204 |
| ToolServer | `codelet/patches/rig-core/src/tool/server.rs` | `remove_tool()` | 251-269 |

## Proposed Solution

### Option 1: Case-Insensitive HashMap (Recommended)

Use a case-insensitive key for the HashMap. This can be achieved by:

1. **Store canonical names (lowercase) as keys**
2. **Normalize all lookups to lowercase**

```rust
pub async fn call(&self, toolname: &str, args: String) -> Result<String, ToolSetError> {
    let normalized = toolname.to_lowercase();
    if let Some(tool) = self.tools.get(normalized.as_str()) {
        // ...
    } else {
        Err(ToolSetError::ToolNotFoundError(toolname.to_string()))
    }
}
```

**Pros:**
- Simple implementation
- No external dependencies
- Preserves original tool names in error messages
- Works for all lookup methods

**Cons:**
- Requires changes to all lookup methods
- Need to ensure tool registration also normalizes names

### Option 2: Two-Level Lookup

Maintain a case-insensitive index that maps lowercase names to canonical names:

```rust
pub struct ToolSet {
    pub(crate) tools: HashMap<String, ToolType>,
    pub(crate) case_insensitive_index: HashMap<String, String>, // lowercase -> canonical
}
```

**Pros:**
- Preserves original tool names in the HashMap
- Explicit mapping for debugging

**Cons:**
- More complex implementation
- Additional memory overhead
- Synchronization between maps

### Option 3: igno Library

Use the `igno` crate for case-insensitive string keys:

```rust
use igno::Igno;

pub struct ToolSet {
    pub(crate) tools: HashMap<Igno<String>, ToolType>,
}
```

**Pros:**
- Clean implementation
- Battle-tested library
- Handles edge cases

**Cons:**
- Additional dependency
- May not be compatible with all HashMap operations

## Recommendation

**Option 1** is recommended because:

1. It's the simplest implementation
2. No external dependencies
3. Easy to test
4. Works for both TypeScript and Rust TUI ports
5. Minimal code changes

## Implementation Plan

### Phase 1: ToolSet Changes

1. **Normalize tool names on registration** (`add_tool`, `add_tool_boxed`, `from_tools`)
2. **Normalize tool names on lookup** (`call`, `get`, `contains`)
3. **Normalize tool names on deletion** (`delete_tool`)
4. **Update error messages** to show the original name but reference the canonical name

### Phase 2: ToolServer Changes

1. **Normalize tool names in `handle_message`** for `CallTool` and `RemoveTool`
2. **Update `get_tool_definitions`** to use normalized names

### Phase 3: Testing

1. **Unit tests** for case-insensitive lookups
2. **Integration tests** with the agent loop
3. **Rust TUI port tests** to ensure compatibility

## Edge Cases

1. **Duplicate tools with different casing**: If `"fspec"` and `"FSPEC"` are both registered, the second registration should override the first (or return an error).
2. **MCP tools**: MCP tool names may have different casing conventions. Ensure the normalization works for all tool types.
3. **Error messages**: When a tool is not found, the error message should show both the requested name and the canonical name for debugging.
4. **Tool definitions**: The tool definitions sent to the LLM should use the canonical name, so the LLM learns the correct casing over time.

## References

- `codelet/patches/rig-core/src/tool/mod.rs` — ToolSet implementation
- `codelet/patches/rig-core/src/tool/server.rs` — ToolServer implementation
- `codelet/agent-loop/src/agent_loop.rs` — Agent loop that uses tool calls
- `codelet/fspec-tui/` — Rust TUI port that uses the same ToolSet
