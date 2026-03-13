# Codex CLI Tool Facades Research

## Source: `/tmp/codex-cli/codex-rs/core/src/tools/spec.rs`

## Problem

Our `CodexProvider::create_rig_agent()` currently registers the **same tools as Claude** (`ReadTool`, `WriteTool`, `EditTool`, `BashTool`, etc.) with Claude-style naming (`Read`, `Write`, `Edit`, `Bash`, `Grep`, `Glob`, `Ls`). But the Codex backend API and GPT models are trained to use **Codex-native tool names and schemas**, which are completely different.

## Codex CLI Native Tool Definitions

### 1. `shell_command` (replaces our `Bash`)
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

### 2. `read_file` (replaces our `Read`)
```
name: "read_file"
params:
  - file_path: String (required) - "Absolute path to the file"
  - offset: Number - "The line number to start reading from. Must be 1 or greater."
  - limit: Number - "The maximum number of lines to return."
  - mode: String - "Optional mode selector: 'slice' or 'indentation'"
  - indentation: Object - nested schema for indentation-aware block mode:
    - anchor_line: Number
    - max_levels: Number
    - include_siblings: Boolean
    - include_header: Boolean
    - max_lines: Number
```

### 3. `list_dir` (replaces our `Ls`)
```
name: "list_dir"
params:
  - dir_path: String (required) - "Absolute path to the directory to list."
  - offset: Number - "The entry number to start listing from. Must be 1 or greater."
  - limit: Number - "The maximum number of entries to return."
  - depth: Number - "The maximum directory depth to traverse. Must be 1 or greater."
```

### 4. `grep_files` (replaces our `Grep`)
```
name: "grep_files"
params:
  - pattern: String (required) - "Regular expression pattern to search for."
  - include: String - "Optional glob that limits which files are searched (e.g. '*.rs')"
  - path: String - "Directory or file path to search. Defaults to session working directory."
  - limit: Number - "Maximum number of file paths to return (defaults to 100)."
```

### 5. `apply_patch` (replaces our `Edit` + `Write`)
Codex uses a **freeform text-based patching format** (not JSON) with a custom Lark grammar:
```
*** Begin Patch
*** Add File: path/to/new/file.ts
+line1
+line2
*** Update File: path/to/existing/file.ts
@@ context line
-old line
+new line
*** Delete File: path/to/delete.ts
*** End Patch
```
This is a `ToolSpec::Freeform` type with a Lark grammar, NOT a `ToolSpec::Function`.

### 6. `view_image` (replaces our image handling in `Read`)
```
name: "view_image"
params:
  - path: String (required) - "Local filesystem path to an image file"
```

### 7. `update_plan` (new - no equivalent in our toolset)
```
name: "update_plan"
params:
  - explanation: String
  - plan: Array<{step: String, status: String}> (required) - "pending|in_progress|completed"
```

### 8. `web_search` (native OpenAI)
```
type: "web_search" (NOT a function tool)
Handled natively by the Codex/OpenAI API, not as a function call.
```

### 9. `shell` (alternative to shell_command)
```
name: "shell"
params:
  - command: Array<String> (required) - passed to execvp()
  - workdir: String
  - timeout_ms: Number
  - sandbox_permissions, justification, prefix_rule (approval params)
```

### 10. `exec_command` (unified exec - PTY support)
```
name: "exec_command"
params:
  - cmd: String (required)
  - workdir: String
  - shell: String
  - tty: Boolean
  - yield_time_ms: Number
  - max_output_tokens: Number
  - login: Boolean
  - sandbox_permissions, justification, prefix_rule
```

### 11. `write_stdin` (interactive session input)
```
name: "write_stdin"
params:
  - session_id: Number (required)
  - chars: String
  - yield_time_ms: Number
  - max_output_tokens: Number
```

### 12. `request_user_input`
```
name: "request_user_input"
params: purpose, items (input fields)
```

## Current State in Codelet

**File**: `codelet/providers/src/codex/mod.rs` (lines 277-289)
```rust
// Currently registers Claude-style tools:
.tool(ReadTool::new(session_id))
.tool(WriteTool::new(session_id))
.tool(EditTool::new(session_id))
.tool(BashTool::new(session_id))
.tool(GrepTool::new(session_id))
.tool(GlobTool::new(session_id))
.tool(LsTool::new(session_id))
.tool(AstGrepTool::new(session_id))
.tool(AstGrepRefactorTool::new(session_id))
.tool(WebSearchTool::new(session_id))
```

These use Claude-style schemas that the GPT model isn't trained on. GPT-5.1-codex expects the native tool names and schemas listed above.

## What We Need

Create Codex-specific facade wrappers (following the existing facade pattern) that:

1. **Map tool names**: `Bash` → `shell_command`, `Read` → `read_file`, `Ls` → `list_dir`, `Grep` → `grep_files`
2. **Map parameter schemas**: Adapt our internal params to match Codex expected schemas
3. **Map parameter names**: e.g., `file_path` stays the same for read_file, but `path` → `dir_path` for list_dir
4. **Handle `apply_patch`**: This is a freeform tool, fundamentally different from our `Edit`/`Write`. We probably keep `Edit`/`Write` for now but consider adding `apply_patch` support later.
5. **Add missing tools**: `update_plan` could be valuable for ACDD workflow

## Facade Pattern Reference

Existing facades in `codelet/tools/src/facade/`:
- `bash.rs` - Gemini `run_shell_command` facade
- `file_ops.rs` - Gemini `read_file`, `write_file`, `replace_in_file` facades  
- `ls.rs` - Gemini `list_directory` facade
- `search.rs` - Gemini `search_file_content`, `find_files` facades
- `zai.rs` - Z.AI `run_command`, `read_file`, `write_file`, etc. facades
- `fspec_facade.rs` - Already has `OpenAIFspecFacade`
- `web_search.rs` - Already has `ClaudeWebSearchFacade`

Each facade implements a trait (e.g., `BashToolFacade`, `FileToolFacade`, etc.) that maps provider-specific params to internal params.

## Recommended Approach

Create new Codex facade structs following the established pattern:

1. `CodexShellCommandFacade` - implements `BashToolFacade`, maps `shell_command` → `InternalBashParams`
2. `CodexReadFileFacade` - implements `FileToolFacade`, maps `read_file` → `InternalFileParams::Read`
3. `CodexListDirFacade` - implements `LsToolFacade`, maps `list_dir` → `InternalLsParams`
4. `CodexGrepFilesFacade` - implements `SearchToolFacade`, maps `grep_files` → `InternalSearchParams::Grep`
5. Register with `FacadeToolWrapper` in `CodexProvider::create_rig_agent()`
6. Add `CodexFspecFacade` already exists as `OpenAIFspecFacade` - reuse or alias

## Files to Create/Modify

### New files:
- `codelet/tools/src/facade/codex.rs` - All Codex-specific facades

### Modified files:
- `codelet/tools/src/facade/mod.rs` - Export codex module
- `codelet/providers/src/codex/mod.rs` - Replace direct tool registration with facade wrappers
- `codelet/tools/src/facade/registry.rs` - Register Codex facades (optional)
