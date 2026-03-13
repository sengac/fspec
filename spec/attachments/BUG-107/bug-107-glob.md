# BUG-107: Codex facade exposes non-native glob tool

## Problem

The Codex facade in `codelet/tools/src/facade/codex.rs` defines a `CodexGlobFacade` struct that exposes a `glob` tool. However, the Codex CLI native tool set (sourced from `codex-rs/core/src/tools/spec.rs`) does **not** define a `glob` tool.

The native Codex tool inventory is:
- `shell_command`
- `read_file`
- `list_dir`
- `grep_files`
- `apply_patch`
- `view_image`
- `update_plan`
- `shell`
- `exec_command`
- `write_stdin`
- `request_user_input`

`glob` does not appear in this list.

## Current Implementation

**File**: `codelet/tools/src/facade/codex.rs` (lines 296-339)

```rust
pub struct CodexGlobFacade;

impl SearchToolFacade for CodexGlobFacade {
    fn provider(&self) -> &'static str { "codex" }
    fn tool_name(&self) -> &'static str { "glob" }
    // ... definition and map_params
}
```

**Registered in**: `codelet/providers/src/codex/mod.rs`

```rust
let glob = SearchToolFacadeWrapper::new(Arc::new(CodexGlobFacade), session_id);
// ...
.tool(glob)  // Codex-native glob
```

## Impact

Exposing a non-native tool name may:
- Confuse the model into thinking it has capabilities it wasn't trained on
- Lead to unexpected tool call patterns
- Pollute the tool namespace alongside actual Codex-native tools

## Recommended Fix

1. Remove `CodexGlobFacade` from `codelet/tools/src/facade/codex.rs`
2. Remove the `glob` tool registration from `CodexProvider::create_rig_agent()`
3. Remove or update the associated tests in the codex facade test module
4. Consider whether `grep_files` with the `include` glob filter parameter is sufficient to replace glob functionality

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- Gap analysis: `spec/attachments/BUG-105/codex-tool-facade-gap-analysis-2026-03-13.md`
- Full tool research: `spec/attachments/BUG-105/codex-tool-facade-research.md`
