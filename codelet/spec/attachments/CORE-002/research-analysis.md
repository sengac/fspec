# CORE-002: Core File Tools Implementation - Research Analysis

## Executive Summary

After analyzing the codelet TypeScript codebase and comparing it with the scaffolded codelet Rust port, the **next logical layer** to implement is the **Core File Tools** (Read, Write, Edit) within the Tool Execution bounded context.

## Rationale for This Choice

### 1. Dependency Analysis

The Foundation Event Storm identifies these Tool Execution aggregates:
- **ToolRegistry** (scaffolded)
- **BashTool** (depends on process execution - defer)
- **ReadTool** (pure file I/O - **good starting point**)
- **WriteTool** (pure file I/O - **good starting point**)
- **EditTool** (depends on ReadTool - **include with Read/Write**)
- **GrepTool** (depends on ripgrep binary - defer)
- **GlobTool** (depends on ripgrep binary - defer)

### 2. Layer-by-Layer Approach

```
Layer 0 (DONE): CORE-001 - Project Scaffold
  └── Bounded context stubs, trait definitions

Layer 1 (THIS STORY): CORE-002 - Core File Tools
  └── Read, Write, Edit tool implementations
  └── Output truncation utilities

Layer 2 (FUTURE): Bash Tool
  └── Process execution with timeout
  └── Output capture and truncation

Layer 3 (FUTURE): Search Tools (Grep, Glob)
  └── Ripgrep integration
  └── Pattern matching
```

### 3. Why File Tools First?

1. **No external dependencies** - Pure Rust std::fs operations
2. **Foundation for everything** - Agent can't modify code without file tools
3. **Well-defined interface** - Clear inputs/outputs from codelet
4. **Testable in isolation** - No async, no network, no processes

## Codelet Reference Implementation

### tools.ts Key Patterns

From `/home/rquast/projects/codelet/src/agent/tools.ts`:

#### Output Limits (shared across all tools)
```typescript
export const OUTPUT_LIMITS = {
  MAX_OUTPUT_CHARS: 30000,
  MAX_LINE_LENGTH: 2000,
  MAX_LINES: 2000,
} as const;
```

#### Read Tool
```typescript
export function executeRead(
  filePath: string,
  offset?: number,  // 1-based line number
  limit?: number    // number of lines
): string {
  // - Requires absolute path
  // - Returns lines with 1-based line numbers: "1: content"
  // - Truncates lines > 2000 chars
  // - Limits total output to 30000 chars
}
```

#### Write Tool
```typescript
export function executeWrite(filePath: string, content: string): string {
  // - Requires absolute path
  // - Overwrites existing files
  // - Returns success/error message
}
```

#### Edit Tool
```typescript
export function executeEdit(
  filePath: string,
  oldString: string,
  newString: string
): string {
  // - Requires absolute path
  // - Replaces FIRST occurrence only
  // - Returns error if oldString not found
  // - Returns success message on replace
}
```

### Truncation Utilities
```typescript
export function truncateOutput(
  lines: string[],
  maxChars: number = OUTPUT_LIMITS.MAX_OUTPUT_CHARS
): TruncateResult {
  // Returns { output, charTruncated, remainingCount, includedCount }
}

export function formatTruncationWarning(
  remainingCount: number,
  itemType: string,
  charTruncated: boolean,
  maxChars: number = OUTPUT_LIMITS.MAX_OUTPUT_CHARS
): string {
  // Returns "... [N lines truncated - output truncated at 30000 chars] ..."
}
```

## Story Scope

### In Scope
1. **Read Tool** - Read file contents with line numbers, offset, limit
2. **Write Tool** - Create/overwrite files
3. **Edit Tool** - Find and replace first occurrence
4. **Output Limits** - Shared constants for truncation
5. **Truncation Utilities** - Functions for output limiting
6. **Unit Tests** - Test each tool in isolation

### Out of Scope (Future Stories)
- Bash tool (process execution)
- Grep tool (ripgrep integration)
- Glob tool (ripgrep integration)
- LS tool (directory listing)
- AstGrep tool (AST search)
- Tool definitions for LLM (JSON schema)

## Story Point Estimate: 8 Points

| Component | Points | Rationale |
|-----------|--------|-----------|
| Read tool with line numbers, offset, limit | 2 | File I/O, line number formatting, truncation |
| Write tool | 1 | Simple file write |
| Edit tool (find/replace) | 2 | String search, error handling |
| Output truncation utilities | 1 | Shared functions |
| ToolOutput refinement | 1 | Result type with truncation info |
| Unit tests | 1 | Test coverage for all tools |
| **Total** | **8** | Fits well in one sprint |

## Proposed File Structure

```
src/tools/
├── mod.rs          # Re-exports, Tool trait (exists)
├── limits.rs       # OUTPUT_LIMITS constants
├── truncation.rs   # truncate_output, format_truncation_warning
├── read.rs         # ReadTool implementation
├── write.rs        # WriteTool implementation
├── edit.rs         # EditTool implementation
└── error.rs        # ToolError type

tests/
├── read_tool_test.rs
├── write_tool_test.rs
└── edit_tool_test.rs
```

## Business Rules (from codelet AGENT-001)

1. File paths must be absolute for Read, Edit, and Write tools
2. Read tool returns lines with 1-based line numbers
3. Read tool truncates after 2000 lines by default
4. Read tool truncates lines longer than 2000 characters
5. Edit tool replaces first occurrence only (no backup)
6. Write tool overwrites existing files without prompting
7. All tools apply 30000 character output limit
8. Truncation warnings include remaining count and reason

## Examples (from codelet)

1. Read `/home/user/src/index.ts` → Returns numbered lines "1: ...", "2: ..."
2. Read file with offset=50, limit=100 → Returns lines 50-149
3. Read file > 2000 lines → Truncation warning appended
4. Write to `/home/user/new.ts` → Creates file, returns success
5. Write to existing file → Overwrites, returns success
6. Edit file replacing "foo" with "bar" → Replaces first match
7. Edit file with non-existent string → Returns error message

## Implementation Notes

### Rust-Specific Considerations

1. Use `std::fs::read_to_string` for Read tool
2. Use `std::fs::write` for Write tool
3. Use `str::find` + slicing for Edit tool
4. Return `Result<ToolOutput, ToolError>` from all tools
5. Consider using `thiserror` for error types
6. Use `Path::is_absolute()` for validation

### Alignment with Scaffolded Types

The existing `src/tools/mod.rs` defines:
- `Tool` trait with `execute` returning `Result<ToolOutput>`
- `ToolOutput` struct with `content` and `is_error` fields
- `ToolRegistry` for managing tools

These need refinement:
- Add `truncated` field to ToolOutput
- Add `remaining_count` field to ToolOutput

## Next Steps After This Story

Once CORE-002 is complete, the next stories in order:

1. **CORE-003**: Bash Tool Implementation (process execution)
2. **CORE-004**: Search Tools (Grep/Glob with ripgrep)
3. **CORE-005**: Tool Definitions for LLM (JSON schema generation)
