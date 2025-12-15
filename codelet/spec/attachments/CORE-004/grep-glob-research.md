# CORE-004: Grep and Glob Tools Research & Analysis

## Overview

This document captures the research and analysis for implementing Grep and Glob tools in codelet using ripgrep's Rust crates as direct dependencies (not spawning a binary).

## Source Analysis: codelet/src/agent/tools.ts

### Original Implementation (TypeScript)

The codelet TypeScript implementation spawns the `rg` (ripgrep) binary:

```typescript
// Grep uses spawnSync to call ripgrep binary
const result = spawnSync(rgPath, args, {
  encoding: 'utf-8',
  maxBuffer: 10 * 1024 * 1024,
  cwd: process.cwd(),
});

// Glob uses ripgrep with --files flag
args.push('--files');
args.push('--glob', params.pattern);
```

### Key Behaviors to Port

**GrepTool:**
1. Three output modes: `files_with_matches` (default), `content`, `count`
2. Context lines: `-A` (after), `-B` (before), `-C` (both)
3. Case-insensitive: `-i` flag
4. Multiline: `-U --multiline-dotall`
5. File filters: `--glob` and `--type`
6. Line numbers: `-n` in content mode

**GlobTool:**
1. Pattern matching with `**/*.ts`, `*.{js,ts}` etc.
2. Respects `.gitignore` by default
3. Searches from specified path or current directory

## Rust Implementation Strategy

### Dependencies (Cargo.toml)

Instead of spawning the `rg` binary, we use ripgrep's underlying crates:

```toml
[dependencies]
# Ripgrep core library for regex search
grep = "0.3"
grep-regex = "0.1"
grep-searcher = "0.1"
grep-matcher = "0.1"

# Ignore crate for gitignore-aware file walking
ignore = "0.4"

# Regex for pattern compilation
regex = "1"
```

### Architecture

```
src/tools/
├── grep.rs          # GrepTool implementation using grep crate
├── glob.rs          # GlobTool implementation using ignore crate
├── mod.rs           # UPDATE: Register GrepTool, GlobTool
├── limits.rs        # EXISTING: OUTPUT_LIMITS constants
├── truncation.rs    # EXISTING: Truncation utilities
└── validation.rs    # EXISTING: Path validation
```

### GrepTool Implementation Strategy

Use the `grep` crate ecosystem:

```rust
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::WalkBuilder;

pub struct GrepTool {
    parameters: ToolParameters,
}

impl GrepTool {
    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let output_mode = args.get("output_mode").and_then(|v| v.as_str()).unwrap_or("files_with_matches");

        // Build regex matcher
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(args.get("-i").and_then(|v| v.as_bool()).unwrap_or(false))
            .multi_line(args.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false))
            .build(&pattern)?;

        // Walk files respecting gitignore
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .build();

        // Search and collect results
        let mut results = Vec::new();
        for entry in walker.flatten() {
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                // Search file with grep_searcher
            }
        }

        // Format output based on mode
        format_grep_output(&results, output_mode)
    }
}
```

### GlobTool Implementation Strategy

Use the `ignore` crate directly:

```rust
use ignore::{WalkBuilder, DirEntry};

pub struct GlobTool {
    parameters: ToolParameters,
}

impl GlobTool {
    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        // Build walker with glob pattern
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .build();

        // Filter by glob pattern using globset
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()?
            .compile_matcher();

        let mut files = Vec::new();
        for entry in walker.flatten() {
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                if glob.is_match(entry.path()) {
                    files.push(entry.path().to_string_lossy().to_string());
                }
            }
        }

        // Apply truncation and format output
        format_glob_output(&files)
    }
}
```

## Crate Analysis

### `grep` Crate (v0.3)
- Core functionality from ripgrep
- `grep-regex`: Regex pattern matching
- `grep-searcher`: File searching with context
- `grep-matcher`: Matcher trait abstraction

### `ignore` Crate (v0.4)
- Gitignore-aware directory walking
- Respects `.gitignore`, `.ignore`, global gitignore
- Fast parallel directory traversal
- Glob pattern matching via `globset`

### `globset` Crate (v0.4)
- Part of ignore ecosystem
- Fast glob pattern matching
- Supports `**`, `*`, `?`, `[...]`, `{a,b}`

## Event Storm Alignment

From foundation.json:
- **Bounded Context**: Tool Execution (id: 3)
- **Aggregates**: GrepTool (id: 18), GlobTool (id: 19)
- **Command**: SearchFiles (id: 62)
- **Events**: ToolInvoked, ToolExecuted, ToolOutputTruncated, ToolFailed

## Test Strategy

### GrepTool Tests (8 scenarios)
1. Search for pattern returns file paths (files_with_matches mode)
2. Content mode shows matching lines with line numbers
3. Glob filter only searches matching files
4. Context lines (-A) includes surrounding lines
5. Case-insensitive flag matches all cases
6. Multiline mode matches patterns spanning lines
7. Count mode returns match counts per file
8. Non-existent pattern returns "No matches found"

### GlobTool Tests (4 scenarios)
1. Pattern returns all matching files recursively
2. Path parameter limits search to directory
3. Non-existent pattern returns "No matches found"
4. Respects gitignore by default

### Integration Tests (2 scenarios)
1. GrepTool and GlobTool registered in default ToolRegistry
2. Runner can execute both tools

## Comparison: TypeScript vs Rust

| Aspect | TypeScript (codelet) | Rust (codelet) |
|--------|---------------------|------------------|
| Execution | Spawns `rg` binary | Links `grep`/`ignore` crates |
| Binary dependency | Requires ripgrep installed | No external binary |
| Performance | Process spawn overhead | Direct library calls |
| Portability | Must bundle/find binary | Compiles into binary |
| Pattern matching | ripgrep CLI flags | `grep-regex` crate |
| File walking | ripgrep `--files` | `ignore` crate |

## Story Points: 8

### Rationale
- Two tools to implement (Grep more complex than Glob)
- New crate dependencies to integrate
- Multiple output modes for Grep
- Context line support
- Multiline matching
- Reuses existing truncation infrastructure
- Similar complexity to CORE-002 + CORE-003 combined

## Dependencies

- CORE-002 (done): Provides Tool trait pattern
- CORE-003 (done): Provides truncation utilities
- No blocking dependencies

## Risks

1. **Crate API Learning Curve**: grep/ignore crate APIs differ from CLI
   - Mitigation: Well-documented crates, examples available

2. **Pattern Syntax Differences**: Regex syntax may differ slightly
   - Mitigation: Use grep-regex which matches ripgrep behavior

3. **Performance for Large Codebases**: Need to handle large file sets
   - Mitigation: ignore crate is optimized for this; add file count limits

## Acceptance Criteria Summary

- [ ] GrepTool implements Tool trait
- [ ] GrepTool uses grep crate for regex matching
- [ ] GrepTool supports files_with_matches, content, count modes
- [ ] GrepTool supports context lines (-A, -B, -C)
- [ ] GrepTool supports case-insensitive and multiline matching
- [ ] GlobTool implements Tool trait
- [ ] GlobTool uses ignore crate for gitignore-aware file matching
- [ ] GlobTool supports standard glob patterns
- [ ] Both tools truncate output at 30000 characters
- [ ] Both tools registered in ToolRegistry.with_core_tools()
- [ ] All 14 test scenarios pass
