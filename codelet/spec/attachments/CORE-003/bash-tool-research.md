# CORE-003: Bash Tool Research & Analysis

## Overview

This document captures the research and analysis for porting the Bash tool from codelet (TypeScript) to codelet (Rust).

## Source Analysis: codelet/src/agent/tools.ts

### Original Implementation (TypeScript)

```typescript
export function executeBash(command: string): string {
  try {
    const output = execSync(command, {
      encoding: 'utf-8',
      maxBuffer: 10 * 1024 * 1024, // 10MB buffer
    });

    // Apply truncation to prevent context overflow
    const lines = output
      .split('\n')
      .map(line =>
        line.length > OUTPUT_LIMITS.MAX_LINE_LENGTH
          ? '[Omitted long line]'
          : line
      );

    const truncateResult = truncateOutput(lines);
    const finalOutput = truncateResult.output +
      formatTruncationWarning(
        truncateResult.remainingCount,
        'lines',
        truncateResult.charTruncated
      );

    return finalOutput;
  } catch (error) {
    return `Error executing command: ${err.message}\nOutput: ${err.stdout || ''}\nStderr: ${err.stderr || ''}`;
  }
}
```

### Key Behaviors to Port

1. **Synchronous Execution**: Original uses `execSync` - Rust will use `tokio::process::Command` for async
2. **Output Limits**:
   - MAX_OUTPUT_CHARS: 30000
   - MAX_LINE_LENGTH: 2000
   - Lines > 2000 chars → `[Omitted long line]`
3. **Error Handling**: Returns combined stdout/stderr on failure
4. **Truncation**: Uses shared `truncateOutput()` and `formatTruncationWarning()` utilities

## Rust Implementation Strategy

### Dependencies

Already available in Cargo.toml:
- `tokio` with "full" features (includes process spawning)
- `anyhow` for error handling

### Architecture

```
src/tools/
├── bash.rs          # NEW: BashTool implementation
├── mod.rs           # UPDATE: Register BashTool
├── limits.rs        # EXISTING: OUTPUT_LIMITS constants
├── truncation.rs    # EXISTING: Truncation utilities
└── validation.rs    # EXISTING: Path validation
```

### Key Implementation Details

1. **Process Execution**:
   ```rust
   use tokio::process::Command;
   use tokio::time::{timeout, Duration};

   let output = timeout(
       Duration::from_secs(timeout_secs),
       Command::new("sh")
           .arg("-c")
           .arg(&command)
           .output()
   ).await;
   ```

2. **Line Truncation**:
   ```rust
   fn truncate_long_lines(output: &str) -> Vec<String> {
       output.lines()
           .map(|line| {
               if line.len() > OUTPUT_LIMITS.MAX_LINE_LENGTH {
                   "[Omitted long line]".to_string()
               } else {
                   line.to_string()
               }
           })
           .collect()
   }
   ```

3. **Tool Parameters**:
   ```json
   {
     "command": {
       "type": "string",
       "description": "The bash command to execute"
     }
   }
   ```

4. **Timeout Parameter** (optional):
   ```json
   {
     "timeout": {
       "type": "integer",
       "description": "Timeout in seconds (default: 120)"
     }
   }
   ```

## Event Storm Alignment

From foundation.json:
- **Bounded Context**: Tool Execution (id: 3)
- **Aggregate**: BashTool (id: 14)
- **Command**: ExecuteBash (id: 58)
- **Events**: ToolInvoked, ToolExecuted, ToolOutputTruncated, ToolFailed

## Test Strategy

### Unit Tests (7 scenarios)

1. Execute simple command successfully
2. Execute command that fails returns error with stderr
3. Long output is truncated at character limit
4. Long lines are replaced with omission message
5. Command timeout returns error
6. BashTool is registered in default ToolRegistry
7. Runner can execute Bash tool

### Integration Points

- Integrates with existing `ToolRegistry.with_core_tools()`
- Reuses `truncation.rs` utilities
- Follows same pattern as Read/Write/Edit tools

## Comparison: TypeScript vs Rust

| Aspect | TypeScript (codelet) | Rust (codelet) |
|--------|---------------------|------------------|
| Execution | `execSync` (sync) | `tokio::process::Command` (async) |
| Buffer | 10MB maxBuffer | Rust Vec (no explicit limit) |
| Timeout | Not implemented | Configurable via parameter |
| Error Type | Error object | `anyhow::Result` |
| Truncation | Shared utility | Same shared utility pattern |

## Story Points: 5

### Rationale
- Similar complexity to CORE-002 (8 points for 3 tools)
- Single tool implementation: ~5 points
- Reuses existing truncation infrastructure
- Well-understood pattern from Read/Write/Edit
- Async process spawning adds some complexity

## Dependencies

- CORE-002 (done): Provides truncation utilities and Tool trait pattern
- No blocking dependencies

## Risks

1. **Platform Differences**: Shell behavior differs between Linux/macOS/Windows
   - Mitigation: Use `/bin/sh -c` on Unix, consider `cmd /c` on Windows later

2. **Process Hanging**: Commands may hang indefinitely
   - Mitigation: Implement timeout with `tokio::time::timeout`

3. **Large Output**: Commands may produce very large output
   - Mitigation: Already handled by truncation at 30000 chars

## Acceptance Criteria Summary

- [ ] BashTool implements Tool trait
- [ ] Executes commands via tokio::process::Command
- [ ] Truncates output at 30000 characters
- [ ] Replaces lines > 2000 chars with "[Omitted long line]"
- [ ] Returns combined stdout/stderr on failure
- [ ] Supports configurable timeout (default 120s)
- [ ] Registered in ToolRegistry.with_core_tools()
- [ ] All 7 test scenarios pass
