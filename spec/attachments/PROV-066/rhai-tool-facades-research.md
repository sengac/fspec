# PROV-066: Custom Provider Rhai-Scriptable Tool Facades — Research Document

**Work Unit:** PROV-066 — Custom provider Rhai-scriptable tool facades
**Parent:** PROV-061 — Custom Provider System
**Depends On:** PROV-062 — Provider config loader and Rhai script compiler
**Date:** 2026-04-17

---

## Table of Contents

1. [Existing Tool Facade Architecture](#1-existing-tool-facade-architecture)
2. [maps_to Target Registry](#2-maps_to-target-registry)
3. [RhaiToolFacadeAdapter Design](#3-rhaitoolfacadeadapter-design)
4. [tool_style Presets](#4-tool_style-presets)
5. [rig::Tool Trait Implementation](#5-rigtool-trait-implementation)
6. [Pre-tool Hook Integration](#6-pre-tool-hook-integration)

---

## 1. Existing Tool Facade Architecture

### 1.1 Architecture Overview

The facade pattern decouples **LLM provider-specific tool schemas** from **base tool implementations**. Each provider (Claude, Gemini, OpenAI, Codex, Z.AI) expects different tool names, parameter schemas, and parameter naming conventions. The facade layer translates between them.

```
┌───────────────────────────────────────────────────────────────────────┐
│                        Provider Layer                                │
├──────────┬──────────┬──────────┬──────────┬──────────┬───────────────┤
│  Claude  │  Gemini  │  OpenAI  │   Codex  │   Z.AI   │   Rhai (new) │
│  Facade  │  Facade  │  Facade  │  Facade  │  Facade  │   Facade     │
├──────────┴──────────┴──────────┴──────────┴──────────┴───────────────┤
│                     Internal*Params Enums                            │
│            (provider-agnostic parameter types)                       │
├──────────────────────────────────────────────────────────────────────┤
│                  Facade Wrapper Layer (impl rig::Tool)               │
│        (dispatches Internal*Params → base tool .call())              │
├──────────────────────────────────────────────────────────────────────┤
│                    Base Tool Implementations                         │
│       ReadTool, WriteTool, EditTool, BashTool, GrepTool, etc.       │
└──────────────────────────────────────────────────────────────────────┘
```

**Source:** `codelet/tools/src/facade/mod.rs` (lines 1–48)

### 1.2 Facade Traits

There are **8 facade traits**, each corresponding to a category of tool operation. Every trait follows the same pattern: `provider()`, `tool_name()`, `definition()`, and `map_params()`.

**Source:** `codelet/tools/src/facade/traits.rs`

#### 1.2.1 ToolFacade (Web Search)

```rust
// codelet/tools/src/facade/traits.rs:103-115
pub trait ToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalWebSearchParams, ToolError>;
}
```

#### 1.2.2 FileToolFacade

```rust
// codelet/tools/src/facade/traits.rs:124-136
pub trait FileToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError>;
}
```

#### 1.2.3 BashToolFacade

```rust
// codelet/tools/src/facade/traits.rs:161-173
pub trait BashToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError>;
}
```

#### 1.2.4 SearchToolFacade

```rust
// codelet/tools/src/facade/traits.rs:225-237
pub trait SearchToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError>;
}
```

#### 1.2.5 LsToolFacade

```rust
// codelet/tools/src/facade/traits.rs:265-277
pub trait LsToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalLsParams, ToolError>;
}
```

#### 1.2.6 FspecToolFacade

```rust
// codelet/tools/src/facade/fspec_facade.rs:24-36
pub trait FspecToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError>;
}
```

#### 1.2.7 ExecToolFacade

```rust
// codelet/tools/src/facade/traits.rs:340-352
pub trait ExecToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalExecParams, ToolError>;
}
```

#### 1.2.8 HitlToolFacade

```rust
// codelet/tools/src/facade/traits.rs:380-392
pub trait HitlToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalHitlParams, ToolError>;
}
```

### 1.3 ToolDefinition Struct

All facades return the same `ToolDefinition` struct which is then converted to rig's `ToolDefinition`:

```rust
// codelet/tools/src/facade/traits.rs:10-18
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,  // JSON Schema
}
```

This maps 1:1 to rig's `ToolDefinition`:

```rust
// codelet/patches/rig-core/src/completion/request.rs:189-194
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
```


### 1.4 Internal Parameter Types (Internal*Params)

These enums are the **provider-agnostic intermediate representation** between facade-specific JSON and base tool execution. There are **9 Internal*Params types**.

#### InternalFileParams

```rust
// codelet/tools/src/facade/traits.rs:76-97
#[derive(Debug, Clone, PartialEq)]
pub enum InternalFileParams {
    Read {
        file_path: String,
        offset: Option<usize>,
        limit: Option<usize>,
        mode: Option<String>,           // "slice" (default) or "indentation"
        indentation: Option<InternalIndentationParams>,
    },
    Write { file_path: String, content: String },
    Edit {
        file_path: String,
        old_string: String,
        new_string: String,
    },
}
```

#### InternalIndentationParams

```rust
// codelet/tools/src/facade/traits.rs:60-72
#[derive(Debug, Clone, PartialEq)]
pub struct InternalIndentationParams {
    pub anchor_line: Option<usize>,
    pub max_levels: Option<usize>,
    pub include_siblings: Option<bool>,
    pub include_header: Option<bool>,
    pub max_lines: Option<usize>,
}
```

#### InternalBashParams

```rust
// codelet/tools/src/facade/traits.rs:143-155
#[derive(Debug, Clone, PartialEq)]
pub enum InternalBashParams {
    Execute {
        command: String,
        cwd: Option<String>,
        timeout_ms: Option<u64>,
    },
}
```

#### InternalSearchParams

```rust
// codelet/tools/src/facade/traits.rs:180-198
#[derive(Debug, Clone, PartialEq)]
pub enum InternalSearchParams {
    Grep {
        pattern: String,
        path: Option<String>,
        include: Option<String>,   // glob filter (e.g., "*.rs")
        limit: Option<usize>,
    },
    Glob {
        pattern: String,
        path: Option<String>,
    },
}
```

#### InternalLsParams

```rust
// codelet/tools/src/facade/traits.rs:244-258
#[derive(Debug, Clone, PartialEq)]
pub enum InternalLsParams {
    List {
        path: Option<String>,
        offset: Option<usize>,
        limit: Option<usize>,
        depth: Option<usize>,
    },
}
```

#### InternalWebSearchParams

```rust
// codelet/tools/src/facade/traits.rs:23-53
#[derive(Debug, Clone, PartialEq)]
pub enum InternalWebSearchParams {
    Search { query: String },
    OpenPage { url: String, headless: bool, pause: bool },
    FindInPage { url: String, pattern: String, headless: bool, pause: bool },
    CaptureScreenshot {
        url: String,
        output_path: Option<String>,
        full_page: bool,
        headless: bool,
        pause: bool,
    },
}
```

#### InternalExecParams

```rust
// codelet/tools/src/facade/traits.rs:288-332
#[derive(Debug, Clone, PartialEq)]
pub enum InternalExecParams {
    Run {
        command: Value,               // shell string or argv array
        workdir: Option<String>,
        tty: bool,
        yield_time_ms: Option<u64>,
        max_output_tokens: Option<u64>,
        timeout_secs: Option<u64>,
    },
    Write {
        session_id: String,
        input: String,
        yield_time_ms: Option<u64>,
        max_output_tokens: Option<u64>,
    },
    Poll {
        session_id: String,
        yield_time_ms: Option<u64>,
        max_output_tokens: Option<u64>,
    },
    List,
    Close { session_id: String },
}
```

#### InternalHitlParams

```rust
// codelet/tools/src/facade/traits.rs:365-372
#[derive(Debug, Clone, PartialEq)]
pub enum InternalHitlParams {
    Request {
        questions: Vec<HitlQuestion>,
    },
}
```

#### InternalFspecParams

```rust
// codelet/tools/src/facade/fspec_facade.rs:13-18
#[derive(Debug, Clone, PartialEq)]
pub struct InternalFspecParams {
    pub command: String,
    pub args: String,
    pub project_root: String,
}
```

#### InternalBridgeParams

```rust
// codelet/tools/src/facade/bridge_facade.rs:14-22
#[derive(Debug, Clone, PartialEq)]
pub enum InternalBridgeParams {
    Connect { url: String },
    Disconnect { url: String },
    List,
}
```

### 1.5 ZAI Facade Implementations (Reference Implementation)

The Z.AI facades in `codelet/tools/src/facade/zai.rs` are the cleanest reference for how facades work. Each facade is a zero-sized struct implementing one facade trait.

**Example: ZAIReadFileFacade** (lines 71-119)

```rust
pub struct ZAIReadFileFacade;

impl FileToolFacade for ZAIReadFileFacade {
    fn provider(&self) -> &'static str { "zai" }
    fn tool_name(&self) -> &'static str { "read_file" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read file contents...".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "..." },
                    "offset": { "type": "integer", "description": "..." },
                    "limit": { "type": "integer", "description": "..." }
                },
                "required": ["file_path"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = extract_required_string(&input, "file_path", "read_file")?;
        let offset = extract_optional_uint(&input, "offset");
        let limit = extract_optional_uint(&input, "limit");
        Ok(InternalFileParams::Read { file_path, offset, limit, mode: None, indentation: None })
    }
}
```

**Complete ZAI tool set** (lines 715-721):
| ZAI Tool Name | Facade Struct | Trait Implemented |
|---|---|---|
| `list_dir` | `ZAIListDirFacade` | `LsToolFacade` |
| `read_file` | `ZAIReadFileFacade` | `FileToolFacade` |
| `write_file` | `ZAIWriteFileFacade` | `FileToolFacade` |
| `edit_file` | `ZAIEditFileFacade` | `FileToolFacade` |
| `run_command` | `ZAIRunCommandFacade` | `BashToolFacade` |
| `grep_files` | `ZAIGrepFilesFacade` | `SearchToolFacade` |
| `find_files` | `ZAIFindFilesFacade` | `SearchToolFacade` |

### 1.6 Codex Facade Implementations (Most Complex)

The Codex facades in `codelet/tools/src/facade/codex.rs` are the most complex, implementing exec, shell, and HITL tools that don't exist in simpler providers.

**Complete Codex tool set:**
| Codex Tool Name | Facade Struct | Trait | maps_to |
|---|---|---|---|
| `shell_command` | `CodexShellCommandFacade` | `BashToolFacade` | BashTool |
| `read_file` | `CodexReadFileFacade` | `FileToolFacade` | ReadTool |
| `list_dir` | `CodexListDirFacade` | `LsToolFacade` | LsTool |
| `view_image` | `CodexViewImageFacade` | `FileToolFacade` | ReadTool |
| `grep_files` | `CodexGrepFilesFacade` | `SearchToolFacade` | GrepTool |
| `shell` | `CodexShellFacade` | `ExecToolFacade` | UnifiedExecTool |
| `exec_command` | `CodexExecCommandFacade` | `ExecToolFacade` | UnifiedExecTool |
| `write_stdin` | `CodexWriteStdinFacade` | `ExecToolFacade` | UnifiedExecTool |
| `request_user_input` | `CodexRequestUserInputFacade` | `HitlToolFacade` | execute_hitl |

**Key differences from ZAI:**
- `list_dir` uses `dir_path` parameter (not `path`) — line 272
- `read_file` supports `mode` and `indentation` sub-object — lines 184-212
- `shell_command` accepts `workdir`, `timeout_ms`, `login`, `sandbox_permissions` — lines 57-101
- `shell` passes command as argv array (not string) — lines 462-496
- `exec_command` supports PTY allocation (`tty` field) — lines 572-588

### 1.7 Gemini Facade Implementations

**Complete Gemini tool set:**
| Gemini Tool Name | Facade Struct | Source File | Trait |
|---|---|---|---|
| `read_file` | `GeminiReadFileFacade` | `file_ops.rs:14` | `FileToolFacade` |
| `write_file` | `GeminiWriteFileFacade` | `file_ops.rs:82` | `FileToolFacade` |
| `replace` | `GeminiReplaceFacade` | `file_ops.rs:140` | `FileToolFacade` |
| `run_shell_command` | `GeminiRunShellCommandFacade` | `bash.rs:14` | `BashToolFacade` |
| `list_directory` | `GeminiListDirectoryFacade` | `ls.rs:14` | `LsToolFacade` |
| `search_file_content` | `GeminiSearchFileContentFacade` | `search.rs:14` | `SearchToolFacade` |
| `glob` | `GeminiGlobFacade` | `search.rs:69` | `SearchToolFacade` |
| `google_web_search` | `GeminiGoogleWebSearchFacade` | `web_search.rs:190` | `ToolFacade` |
| `web_fetch` | `GeminiWebFetchFacade` | `web_search.rs:237` | `ToolFacade` |
| `capture_screenshot` | `GeminiWebScreenshotFacade` | `web_search.rs:293` | `ToolFacade` |
| `fspec_command` | `GeminiFspecFacade` | `fspec_facade.rs:89` | `FspecToolFacade` |

### 1.8 Parameter Extraction Helpers

`codelet/tools/src/facade/param_extract.rs` provides lenient extraction functions that tolerate LLM type errors:

```rust
// Accepts both JSON numbers and numeric strings
pub fn extract_optional_uint(input: &Value, field: &str) -> Option<usize>

// Required string — errors on missing/null/empty
pub fn extract_required_string(input: &Value, field: &str, tool: &'static str) -> Result<String, ToolError>

// Optional string — returns None for missing/null/empty
pub fn extract_optional_string(input: &Value, field: &str) -> Option<String>

// Boolean — accepts true/false, "true"/"false", "yes"/"no", 0/1
pub fn extract_optional_bool(input: &Value, field: &str) -> Option<bool>

// Raw Value → u64 coercion
pub fn value_as_u64_lenient(v: &Value) -> Option<u64>

// Raw Value → bool coercion
pub fn value_as_bool_lenient(v: &Value) -> Option<bool>
```

### 1.9 Wrapper Types

`codelet/tools/src/facade/wrapper.rs` contains **9 wrapper types**, each implementing `rig::tool::Tool`:

| Wrapper | const NAME | Facade Trait | Output Type | Base Tool(s) |
|---|---|---|---|---|
| `FacadeToolWrapper` | `"facade_wrapper"` | `ToolFacade` | `WebSearchResult` | `WebSearchTool` |
| `FileToolFacadeWrapper` | `"file_facade_wrapper"` | `FileToolFacade` | `FileOperationResult` | `ReadTool`, `WriteTool`, `EditTool` |
| `BashToolFacadeWrapper` | `"bash_facade_wrapper"` | `BashToolFacade` | `BashOperationResult` | `BashTool` |
| `SearchToolFacadeWrapper` | `"search_facade_wrapper"` | `SearchToolFacade` | `SearchOperationResult` | `GrepTool`, `GlobTool` |
| `LsToolFacadeWrapper` | `"ls_facade_wrapper"` | `LsToolFacade` | `LsOperationResult` | `LsTool` |
| `FspecToolFacadeWrapper` | `"fspec_facade_wrapper"` | `FspecToolFacade` | `String` | FspecHandler (TypeScript) |
| `BridgeToolFacadeWrapper` | `"bridge_facade_wrapper"` | `BridgeToolFacade` | `BridgeOperationResult` | BridgeHandler |
| `ExecToolFacadeWrapper` | `"exec_facade_wrapper"` | `ExecToolFacade` | `ExecOperationResult` | `UnifiedExecTool` |
| `HitlToolFacadeWrapper` | `"hitl_facade_wrapper"` | `HitlToolFacade` | `HitlOperationResult` | `execute_hitl()` |


---

## 2. maps_to Target Registry

### 2.1 Concept

The `maps_to` field in a Rhai tool definition specifies which **base tool** the custom tool routes to. When the LLM calls a Rhai-defined tool, the `RhaiToolFacadeAdapter` must:

1. Parse provider-specific JSON params from the LLM
2. Optionally call `map_tool_params()` in Rhai for custom transformation
3. Convert the result to an `Internal*Params` variant
4. Route to the correct wrapper's execution logic

### 2.2 Complete maps_to Target Table

Based on exhaustive analysis of all wrapper `call()` implementations, the following `maps_to` targets are supported:

| `maps_to` value | Internal Params Type | Wrapper | Base Tool(s) | Description |
|---|---|---|---|---|
| `"read_file"` | `InternalFileParams::Read` | `FileToolFacadeWrapper` | `ReadTool` | Read file content (text, images, PDFs) |
| `"write_file"` | `InternalFileParams::Write` | `FileToolFacadeWrapper` | `WriteTool` | Write/create file |
| `"edit_file"` | `InternalFileParams::Edit` | `FileToolFacadeWrapper` | `EditTool` | Search-and-replace in file |
| `"bash"` | `InternalBashParams::Execute` | `BashToolFacadeWrapper` | `BashTool` | Execute shell command |
| `"grep"` | `InternalSearchParams::Grep` | `SearchToolFacadeWrapper` | `GrepTool` | Search file contents by regex |
| `"glob"` | `InternalSearchParams::Glob` | `SearchToolFacadeWrapper` | `GlobTool` | Find files by glob pattern |
| `"ls"` | `InternalLsParams::List` | `LsToolFacadeWrapper` | `LsTool` | List directory contents |
| `"web_search"` | `InternalWebSearchParams::Search` | `FacadeToolWrapper` | `WebSearchTool` | Web search query |
| `"web_open"` | `InternalWebSearchParams::OpenPage` | `FacadeToolWrapper` | `WebSearchTool` | Open/fetch URL |
| `"web_find"` | `InternalWebSearchParams::FindInPage` | `FacadeToolWrapper` | `WebSearchTool` | Find pattern in web page |
| `"web_screenshot"` | `InternalWebSearchParams::CaptureScreenshot` | `FacadeToolWrapper` | `WebSearchTool` | Capture webpage screenshot |
| `"exec_run"` | `InternalExecParams::Run` | `ExecToolFacadeWrapper` | `UnifiedExecTool` | Run command (PTY capable) |
| `"exec_write"` | `InternalExecParams::Write` | `ExecToolFacadeWrapper` | `UnifiedExecTool` | Write to running session |
| `"exec_poll"` | `InternalExecParams::Poll` | `ExecToolFacadeWrapper` | `UnifiedExecTool` | Poll session output |
| `"exec_list"` | `InternalExecParams::List` | `ExecToolFacadeWrapper` | `UnifiedExecTool` | List active sessions |
| `"exec_close"` | `InternalExecParams::Close` | `ExecToolFacadeWrapper` | `UnifiedExecTool` | Close session |
| `"fspec"` | `InternalFspecParams` | `FspecToolFacadeWrapper` | FspecHandler | Execute fspec command |
| `"hitl"` | `InternalHitlParams::Request` | `HitlToolFacadeWrapper` | `execute_hitl()` | Request user input |
| `"bridge_connect"` | `InternalBridgeParams::Connect` | `BridgeToolFacadeWrapper` | BridgeHandler | Connect WebSocket |
| `"bridge_disconnect"` | `InternalBridgeParams::Disconnect` | `BridgeToolFacadeWrapper` | BridgeHandler | Disconnect WebSocket |
| `"bridge_list"` | `InternalBridgeParams::List` | `BridgeToolFacadeWrapper` | BridgeHandler | List connections |

### 2.3 How Wrapper call() Dispatches Internal Params to Base Tools

#### FileToolFacadeWrapper (lines 400-551)

The `call()` method matches on `InternalFileParams` variants:

```rust
// codelet/tools/src/facade/wrapper.rs:407-549
match internal_params {
    InternalFileParams::Read { file_path, offset, limit, .. } => {
        let resolved_path = validate_and_resolve_path(self.session_id, &file_path, "read")?;
        let read_args = ReadArgs { file_path: resolved_path, offset, limit, pdf_mode: None };
        self.read_tool.call(read_args).await  // → FileOperationResult
    }
    InternalFileParams::Write { file_path, content } => {
        let resolved_path = validate_and_resolve_path(self.session_id, &file_path, "write")?;
        // BLOCK-006: check_write_permission before writing
        let write_args = WriteArgs { file_path: resolved_path, content };
        self.write_tool.call(write_args).await
    }
    InternalFileParams::Edit { file_path, old_string, new_string } => {
        let resolved_path = validate_and_resolve_path(self.session_id, &file_path, "edit")?;
        // BLOCK-006: check_write_permission before editing
        let edit_args = EditArgs { file_path: resolved_path, old_string, new_string };
        self.edit_tool.call(edit_args).await
    }
}
```

#### BashToolFacadeWrapper (lines 1143-1182)

```rust
// codelet/tools/src/facade/wrapper.rs:1152-1181
match internal_params {
    InternalBashParams::Execute { command, cwd, .. } => {
        let bash_args = BashArgs { command: command.clone(), cwd };
        self.bash_tool.call(bash_args).await  // → BashOperationResult
    }
}
```

#### SearchToolFacadeWrapper (lines 1269-1381)

```rust
// codelet/tools/src/facade/wrapper.rs:1277-1381
match internal_params {
    InternalSearchParams::Grep { pattern, path, include, limit } => {
        let resolved_path = validate_and_resolve_path(...);
        let grep_args = GrepArgs { pattern, path: resolved_path, output_mode: None, glob: include, limit };
        self.grep_tool.call(grep_args).await
    }
    InternalSearchParams::Glob { pattern, path } => {
        let resolved_path = validate_and_resolve_path(...);
        let glob_args = GlobArgs { pattern, path: resolved_path, case_insensitive: None };
        self.glob_tool.call(glob_args).await
    }
}
```

#### ExecToolFacadeWrapper (lines 1850-1891)

```rust
// codelet/tools/src/facade/wrapper.rs:1857-1891
let json_args = internal_exec_params_to_json(internal_params);
self.exec_tool.call(UnifiedExecArgs(json_args)).await  // → ExecOperationResult
```

The helper `internal_exec_params_to_json()` (lines 1762-1825) converts each `InternalExecParams` variant to JSON:
- `Run` → `{"action": "run", "command": ..., "tty": ...}`
- `Write` → `{"action": "write", "session_id": ..., "input": ...}`
- `Poll` → `{"action": "poll", "session_id": ...}`
- `List` → `{"action": "list"}`
- `Close` → `{"action": "close", "session_id": ...}`

### 2.4 Required Internal*Params Fields Per maps_to

For the `default_to_internal()` function (auto-mapping without Rhai), these are the field names that must be extracted from the LLM's JSON params:

| `maps_to` | Required Fields | Optional Fields |
|---|---|---|
| `read_file` | `file_path` | `offset`, `limit`, `mode`, `indentation.*` |
| `write_file` | `file_path`, `content` | — |
| `edit_file` | `file_path`, `old_string`, `new_string` | — |
| `bash` | `command` | `cwd`, `timeout_ms` |
| `grep` | `pattern` | `path`, `include`, `limit` |
| `glob` | `pattern` | `path` |
| `ls` | — | `path`, `offset`, `limit`, `depth` |
| `web_search` | `query` | — |
| `web_open` | `url` | `headless`, `pause` |
| `web_find` | `url`, `pattern` | `headless`, `pause` |
| `web_screenshot` | `url` | `output_path`, `full_page`, `headless`, `pause` |
| `exec_run` | `command` | `workdir`, `tty`, `yield_time_ms`, `max_output_tokens`, `timeout_secs` |
| `exec_write` | `session_id`, `input` | `yield_time_ms`, `max_output_tokens` |
| `exec_poll` | `session_id` | `yield_time_ms`, `max_output_tokens` |
| `exec_list` | — | — |
| `exec_close` | `session_id` | — |
| `fspec` | `command` | `args`, `project_root` |
| `hitl` | `questions` | — |


---

## 3. RhaiToolFacadeAdapter Design

### 3.1 Overview

`RhaiToolFacadeAdapter` is a **single generic struct** that implements `rig::tool::Tool` and routes ALL Rhai-defined tools. Unlike the existing architecture where each tool has a dedicated facade struct, the adapter is parameterized at construction time with tool metadata parsed from the Rhai script's `define_tools()` return value.

### 3.2 RhaiToolDef — Parsed Tool Definition

When the Rhai script's `define_tools(config)` function is called, it returns an array of maps. Each map is parsed into a `RhaiToolDef`:

```rust
/// A tool definition parsed from a Rhai define_tools() return value.
#[derive(Debug, Clone)]
pub struct RhaiToolDef {
    /// Tool name as the LLM sees it (e.g., "read_file", "execute_code")
    pub name: String,
    /// Tool description for the LLM
    pub description: String,
    /// JSON Schema for the tool parameters
    pub parameters: serde_json::Value,
    /// Which base tool this routes to (e.g., "read_file", "bash", "grep")
    pub maps_to: String,
    /// Whether this tool is visible to the LLM (default: true)
    pub visible: bool,
}
```

**Rhai script example:**

```rhai
fn define_tools(config) {
    [
        #{
            name: "execute_code",
            description: "Run a shell command in the sandbox",
            parameters: #{
                type: "object",
                properties: #{
                    code: #{ type: "string", description: "The command to execute" }
                },
                required: ["code"]
            },
            maps_to: "bash"
        },
        #{
            name: "view_file",
            description: "Read a file from the filesystem",
            parameters: #{
                type: "object",
                properties: #{
                    path: #{ type: "string", description: "File path to read" },
                    start_line: #{ type: "integer", description: "Start line (optional)" }
                },
                required: ["path"]
            },
            maps_to: "read_file"
        }
    ]
}
```

### 3.3 Parsing define_tools() Return Value

The `define_tools()` return value is a Rhai `Array` (= `Vec<Dynamic>`). Each element is a Rhai `Map` (= `BTreeMap<Identifier, Dynamic>`). The conversion uses the existing `dynamic_to_json_value()` / `json_value_to_dynamic()` helpers from `codelet/providers/src/oauth/building_blocks.rs` (lines 215-271):

```rust
/// Parse define_tools() return value into Vec<RhaiToolDef>.
fn parse_tool_definitions(result: Dynamic) -> Result<Vec<RhaiToolDef>, ToolError> {
    let arr = result.try_cast::<rhai::Array>()
        .ok_or_else(|| ToolError::Validation {
            tool: "rhai_provider",
            message: "define_tools() must return an array of maps".to_string(),
        })?;

    let mut tools = Vec::new();
    for item in arr {
        let map = item.try_cast::<rhai::Map>()
            .ok_or_else(|| ToolError::Validation {
                tool: "rhai_provider",
                message: "Each tool definition must be a map".to_string(),
            })?;

        let name = map.get("name")
            .and_then(|v| v.clone().into_string().ok())
            .ok_or_else(|| ToolError::Validation {
                tool: "rhai_provider",
                message: "Tool definition missing 'name' field".to_string(),
            })?;

        let description = map.get("description")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();

        let maps_to = map.get("maps_to")
            .and_then(|v| v.clone().into_string().ok())
            .ok_or_else(|| ToolError::Validation {
                tool: "rhai_provider",
                message: format!("Tool '{name}' missing 'maps_to' field"),
            })?;

        // Convert Rhai Map → serde_json::Value for parameters schema
        let parameters = map.get("parameters")
            .map(|v| dynamic_to_json_value(v))
            .unwrap_or_else(|| json!({"type": "object"}));

        let visible = map.get("visible")
            .and_then(|v| v.as_bool().ok())
            .unwrap_or(true);

        tools.push(RhaiToolDef { name, description, parameters, maps_to, visible });
    }
    Ok(tools)
}
```

### 3.4 map_tool_params() — Custom Parameter Transformation

When a Rhai script defines `map_tool_params(config, tool_name, maps_to, params)`, the adapter calls it to transform LLM params before routing to the base tool. This is the Rhai equivalent of each facade's `map_params()` method.

**Rhai script example:**

```rhai
fn map_tool_params(config, tool_name, maps_to, params) {
    if tool_name == "execute_code" {
        // Transform "code" param to "command" for bash
        #{
            command: params.code
        }
    } else if tool_name == "view_file" {
        // Transform "path" → "file_path", "start_line" → "offset"
        let result = #{
            file_path: params.path
        };
        if params.contains("start_line") {
            result.offset = params.start_line;
        }
        result
    } else {
        // Pass through unchanged
        params
    }
}
```

**Execution flow:**

```rust
/// Call map_tool_params() in Rhai if it exists, otherwise pass params through.
fn call_map_tool_params(
    engine: &Engine,
    ast: &AST,
    config: Dynamic,
    tool_name: &str,
    maps_to: &str,
    params: Value,
) -> Result<Value, ToolError> {
    // Check if function exists in the AST
    let has_map_fn = ast.iter_functions()
        .any(|f| f.name == "map_tool_params");

    if !has_map_fn {
        return Ok(params);  // No custom mapping — pass through
    }

    let params_dynamic = json_value_to_dynamic(&params);
    let mut scope = Scope::new();

    let result: Dynamic = engine.call_fn(
        &mut scope,
        ast,
        "map_tool_params",
        (config, tool_name.to_string(), maps_to.to_string(), params_dynamic),
    ).map_err(|e| ToolError::Execution {
        tool: "rhai_provider",
        message: format!("map_tool_params() failed: {e}"),
    })?;

    Ok(dynamic_to_json_value(&result))
}
```

### 3.5 default_to_internal() — Auto-mapping Without Rhai

When no `map_tool_params()` function exists in the Rhai script, the adapter uses `default_to_internal()` to directly map JSON parameter names to `Internal*Params` fields. This uses the same `extract_*` helpers from `param_extract.rs`.

```rust
/// Build Internal*Params from JSON using field name matching.
/// This is the fallback when no map_tool_params() is defined in Rhai.
fn default_to_internal(maps_to: &str, params: &Value) -> Result<InternalParamsVariant, ToolError> {
    match maps_to {
        "read_file" => {
            let file_path = extract_required_string(params, "file_path", "rhai_tool")?;
            let offset = extract_optional_uint(params, "offset");
            let limit = extract_optional_uint(params, "limit");
            Ok(InternalParamsVariant::File(InternalFileParams::Read {
                file_path, offset, limit, mode: None, indentation: None,
            }))
        }
        "write_file" => {
            let file_path = extract_required_string(params, "file_path", "rhai_tool")?;
            let content = extract_required_string(params, "content", "rhai_tool")?;
            Ok(InternalParamsVariant::File(InternalFileParams::Write { file_path, content }))
        }
        "edit_file" => {
            let file_path = extract_required_string(params, "file_path", "rhai_tool")?;
            let old_string = extract_required_string(params, "old_string", "rhai_tool")?;
            let new_string = extract_required_string(params, "new_string", "rhai_tool")?;
            Ok(InternalParamsVariant::File(InternalFileParams::Edit {
                file_path, old_string, new_string,
            }))
        }
        "bash" => {
            let command = extract_required_string(params, "command", "rhai_tool")?;
            let cwd = extract_optional_string(params, "cwd");
            let timeout_ms = params.get("timeout_ms")
                .and_then(value_as_u64_lenient);
            Ok(InternalParamsVariant::Bash(InternalBashParams::Execute {
                command, cwd, timeout_ms,
            }))
        }
        "grep" => {
            let pattern = extract_required_string(params, "pattern", "rhai_tool")?;
            let path = extract_optional_string(params, "path");
            let include = extract_optional_string(params, "include");
            let limit = extract_optional_uint(params, "limit");
            Ok(InternalParamsVariant::Search(InternalSearchParams::Grep {
                pattern, path, include, limit,
            }))
        }
        "glob" => {
            let pattern = extract_required_string(params, "pattern", "rhai_tool")?;
            let path = extract_optional_string(params, "path");
            Ok(InternalParamsVariant::Search(InternalSearchParams::Glob { pattern, path }))
        }
        "ls" => {
            let path = extract_optional_string(params, "path");
            let offset = extract_optional_uint(params, "offset");
            let limit = extract_optional_uint(params, "limit");
            let depth = extract_optional_uint(params, "depth");
            Ok(InternalParamsVariant::Ls(InternalLsParams::List {
                path, offset, limit, depth,
            }))
        }
        // ... web_search, exec_*, fspec, hitl, bridge_* variants ...
        unknown => Err(ToolError::Validation {
            tool: "rhai_tool",
            message: format!("Unknown maps_to target: '{unknown}'"),
        }),
    }
}
```

### 3.6 execute_internal() — Dispatch to Base Tool

After `Internal*Params` is constructed (either via Rhai `map_tool_params()` + parsing, or via `default_to_internal()`), the adapter dispatches to the correct base tool. This reuses the exact same logic as the existing wrappers.

```rust
/// Unified enum for all internal param types
enum InternalParamsVariant {
    File(InternalFileParams),
    Bash(InternalBashParams),
    Search(InternalSearchParams),
    Ls(InternalLsParams),
    WebSearch(InternalWebSearchParams),
    Exec(InternalExecParams),
    Fspec(InternalFspecParams),
    Hitl(InternalHitlParams),
    Bridge(InternalBridgeParams),
}

/// Execute the base tool based on the internal params variant.
/// Returns a JSON string result suitable for LLM consumption.
async fn execute_internal(
    variant: InternalParamsVariant,
    session_id: Uuid,
) -> Result<String, ToolError> {
    match variant {
        InternalParamsVariant::File(params) => {
            // Reuse FileToolFacadeWrapper's dispatch logic
            execute_file_op(params, session_id).await
        }
        InternalParamsVariant::Bash(params) => {
            execute_bash_op(params, session_id).await
        }
        InternalParamsVariant::Search(params) => {
            execute_search_op(params, session_id).await
        }
        InternalParamsVariant::Ls(params) => {
            execute_ls_op(params, session_id).await
        }
        // ... etc for all variants
    }
}
```

### 3.7 Error Handling

The adapter handles errors at multiple levels:

1. **Rhai compilation errors** — caught at `ScriptLoader` level (PROV-062), fail fast
2. **define_tools() runtime errors** — caught during tool registration, prevents session start
3. **map_tool_params() runtime errors** — returned as `ToolError::Execution` to the LLM
4. **Unknown maps_to targets** — returned as `ToolError::Validation`
5. **Missing required params** — `extract_required_string()` returns `ToolError::Validation`
6. **Base tool execution errors** — propagated from the underlying tool

### 3.8 RhaiToolFacadeAdapter Struct

```rust
/// A single generic struct implementing rig::Tool that routes ALL Rhai-defined tools.
///
/// One instance is created per tool definition from define_tools().
/// All instances sharing the same Rhai script share the same Engine + AST (via Arc).
pub struct RhaiToolFacadeAdapter {
    /// The parsed tool definition from define_tools()
    tool_def: RhaiToolDef,
    /// Shared Rhai engine (sandboxed, with building block modules)
    engine: Arc<Engine>,
    /// Compiled Rhai script AST
    ast: Arc<AST>,
    /// Provider config as Rhai Dynamic (passed to map_tool_params)
    config: Dynamic,
    /// Whether map_tool_params() exists in the script
    has_map_fn: bool,
    /// Session ID for base tool construction and pre-tool hooks
    session_id: Uuid,
    /// Base tools — lazily shared across all adapter instances for this session
    base_tools: Arc<BaseToolSet>,
}

/// Shared set of base tools for a session.
/// Created once and shared across all RhaiToolFacadeAdapter instances.
struct BaseToolSet {
    read_tool: ReadTool,
    write_tool: WriteTool,
    edit_tool: EditTool,
    bash_tool: BashTool,
    grep_tool: GrepTool,
    glob_tool: GlobTool,
    ls_tool: LsTool,
    web_search_tool: WebSearchTool,
    exec_tool: UnifiedExecTool,
    session_id: Uuid,
}
```


---

## 4. tool_style Presets

### 4.1 Concept

The `tool_style` field in a provider config specifies a **preset** that auto-generates a standard tool set without requiring the Rhai script to define `define_tools()`. Each preset matches one of the existing provider facades. This allows custom providers to say "I want Claude-style tools" or "I want Codex-style tools" without writing any Rhai tool mapping code.

### 4.2 Claude Preset (`tool_style: "claude"`)

Derived from the Claude facades across `web_search.rs`, `file_ops.rs` (Gemini facades used as Claude defaults since Claude has no separate file facades — it uses the rig native tool schema), `bash.rs`, `search.rs`, `ls.rs`, `fspec_facade.rs`.

**Note:** Claude uses rig's native tool schema generation (via `schemars::schema_for!()`) for most tools. The facade-based tools are only for WebSearch and Fspec. For a `tool_style: "claude"` preset, we use Claude-like naming:

| Tool Name | maps_to | Parameter Schema Source |
|---|---|---|
| `Read` | `read_file` | `ReadArgs` schema (native rig) |
| `Write` | `write_file` | `WriteArgs` schema (native rig) |
| `Edit` | `edit_file` | `EditArgs` schema (native rig) |
| `Bash` | `bash` | `BashArgs` schema (native rig) |
| `Grep` | `grep` | `GrepArgs` schema (native rig) |
| `Glob` | `glob` | `GlobArgs` schema (native rig) |
| `Ls` | `ls` | `LsArgs` schema (native rig) |
| `WebSearch` | `web_search` | Flat action_type schema from `ClaudeWebSearchFacade` |
| `Fspec` | `fspec` | `FspecArgs` schema from `ClaudeFspecFacade` |
| `Bridge` | `bridge_*` | Nested action schema from `ClaudeBridgeFacade` |

### 4.3 OpenAI Preset (`tool_style: "openai"`)

OpenAI uses the same schemas as Claude for most tools (both use `schemars` generated schemas). Key difference: Fspec tool is named `fspec` instead of `Fspec`.

| Tool Name | maps_to | Notes |
|---|---|---|
| `Read` | `read_file` | Same as Claude |
| `Write` | `write_file` | Same as Claude |
| `Edit` | `edit_file` | Same as Claude |
| `Bash` | `bash` | Same as Claude |
| `Grep` | `grep` | Same as Claude |
| `Glob` | `glob` | Same as Claude |
| `Ls` | `ls` | Same as Claude |
| `fspec` | `fspec` | `OpenAIFspecFacade` — lowercase name |
| `bridge` | `bridge_*` | `OpenAIBridgeFacade` — same nested schema |

### 4.4 Gemini Preset (`tool_style: "gemini"`)

Gemini prefers flat schemas, snake_case names, no `oneOf`, and separate focused tools.

| Tool Name | maps_to | Facade Source |
|---|---|---|
| `read_file` | `read_file` | `GeminiReadFileFacade` (`file_ops.rs:14`) |
| `write_file` | `write_file` | `GeminiWriteFileFacade` (`file_ops.rs:82`) |
| `replace` | `edit_file` | `GeminiReplaceFacade` (`file_ops.rs:140`) |
| `run_shell_command` | `bash` | `GeminiRunShellCommandFacade` (`bash.rs:14`) |
| `list_directory` | `ls` | `GeminiListDirectoryFacade` (`ls.rs:14`) |
| `search_file_content` | `grep` | `GeminiSearchFileContentFacade` (`search.rs:14`) |
| `glob` | `glob` | `GeminiGlobFacade` (`search.rs:69`) |
| `google_web_search` | `web_search` | `GeminiGoogleWebSearchFacade` (`web_search.rs:190`) |
| `web_fetch` | `web_open` | `GeminiWebFetchFacade` (`web_search.rs:237`) |
| `capture_screenshot` | `web_screenshot` | `GeminiWebScreenshotFacade` (`web_search.rs:293`) |
| `fspec_command` | `fspec` | `GeminiFspecFacade` (`fspec_facade.rs:89`) |
| `bridge_connection` | `bridge_*` | `GeminiBridgeFacade` (flat action_type schema) |

**Key Gemini schema characteristics (from actual source):**
- No `additionalProperties: false` (unlike ZAI/Codex)
- `type: "number"` for integers (not `"integer"`)
- Separate tools for web operations (no unified action dispatch)
- `dir_path` parameter name for search/glob (not `path`)
- Edit tool called `replace` (not `edit_file`)

### 4.5 Codex Preset (`tool_style: "codex"`)

Codex has the most tools, including exec and HITL support.

| Tool Name | maps_to | Facade Source |
|---|---|---|
| `shell_command` | `bash` | `CodexShellCommandFacade` (`codex.rs:42`) |
| `read_file` | `read_file` | `CodexReadFileFacade` (`codex.rs:117`) |
| `list_dir` | `ls` | `CodexListDirFacade` (`codex.rs:229`) |
| `view_image` | `read_file` | `CodexViewImageFacade` (`codex.rs:296`) |
| `grep_files` | `grep` | `CodexGrepFilesFacade` (`codex.rs:357`) |
| `shell` | `exec_run` | `CodexShellFacade` (`codex.rs:424`) |
| `exec_command` | `exec_run` | `CodexExecCommandFacade` (`codex.rs:519`) |
| `write_stdin` | `exec_write`/`exec_poll` | `CodexWriteStdinFacade` (`codex.rs:609`) |
| `request_user_input` | `hitl` | `CodexRequestUserInputFacade` (`codex.rs:718`) |

**Key Codex schema characteristics (from actual source):**
- `additionalProperties: false` on all schemas
- Uses `dir_path` (not `path`) for `list_dir`
- `shell_command` accepts `workdir`, `timeout_ms`, `login`, `sandbox_permissions`, `justification`, `prefix_rule`
- `read_file` supports `mode: "indentation"` with nested `indentation` sub-object
- `shell` command is `type: "array"` (argv), not string
- `exec_command` uses `cmd` (not `command`) parameter name

### 4.6 ZAI Preset (`tool_style: "zai"`)

ZAI is the simplest preset — flat schemas with `additionalProperties: false`.

| Tool Name | maps_to | Facade Source |
|---|---|---|
| `list_dir` | `ls` | `ZAIListDirFacade` (`zai.rs:30`) |
| `read_file` | `read_file` | `ZAIReadFileFacade` (`zai.rs:71`) |
| `write_file` | `write_file` | `ZAIWriteFileFacade` (`zai.rs:127`) |
| `edit_file` | `edit_file` | `ZAIEditFileFacade` (`zai.rs:170`) |
| `run_command` | `bash` | `ZAIRunCommandFacade` (`zai.rs:228`) |
| `grep_files` | `grep` | `ZAIGrepFilesFacade` (`zai.rs:270`) |
| `find_files` | `glob` | `ZAIFindFilesFacade` (`zai.rs:314`) |
| `run_fspec` | `fspec` | `ZAIFspecFacade` (`fspec_facade.rs:208`) |

**Key ZAI schema characteristics (from actual source `zai.rs:740-771`):**
- ALL schemas have `additionalProperties: false`
- Path parameters have `"default": "."` in schema
- Uses `path` parameter (not `dir_path`)
- Snake_case tool names

### 4.7 Preset Implementation Design

```rust
/// Generate tool definitions for a tool_style preset.
fn preset_tool_definitions(style: &str) -> Vec<RhaiToolDef> {
    match style {
        "claude" => claude_preset_tools(),
        "openai" => openai_preset_tools(),
        "gemini" => gemini_preset_tools(),
        "codex" => codex_preset_tools(),
        "zai" => zai_preset_tools(),
        _ => vec![],  // Unknown preset — empty (script must define_tools)
    }
}

/// Extract tool definitions from existing facade structs.
fn zai_preset_tools() -> Vec<RhaiToolDef> {
    vec![
        facade_to_rhai_tool_def(&ZAIListDirFacade, "ls"),
        facade_to_rhai_tool_def(&ZAIReadFileFacade, "read_file"),
        facade_to_rhai_tool_def(&ZAIWriteFileFacade, "write_file"),
        facade_to_rhai_tool_def(&ZAIEditFileFacade, "edit_file"),
        facade_to_rhai_tool_def(&ZAIRunCommandFacade, "bash"),
        facade_to_rhai_tool_def(&ZAIGrepFilesFacade, "grep"),
        facade_to_rhai_tool_def(&ZAIFindFilesFacade, "glob"),
    ]
}

/// Helper to extract RhaiToolDef from any existing facade.
fn facade_to_rhai_tool_def<F: FileToolFacade>(facade: &F, maps_to: &str) -> RhaiToolDef {
    let def = facade.definition();
    RhaiToolDef {
        name: def.name,
        description: def.description,
        parameters: def.parameters,
        maps_to: maps_to.to_string(),
        visible: true,
    }
}
```

### 4.8 tool_style + define_tools() Interaction

The priority order is:

1. **If `define_tools()` exists in Rhai script:** its return value completely replaces the preset tools
2. **If `tool_style` is set and no `define_tools()`:** preset tools are used
3. **If neither is set:** error — no tools to register
4. **If both are set:** `define_tools()` wins (it can reference preset defaults via a helper)


---

## 5. rig::Tool Trait Implementation

### 5.1 The rig::Tool Trait

From `codelet/patches/rig-core/src/tool/mod.rs` (lines 106-136):

```rust
pub trait Tool: Sized + WasmCompatSend + WasmCompatSync {
    /// The name of the tool. This name should be unique.
    const NAME: &'static str;

    type Error: std::error::Error + WasmCompatSend + WasmCompatSync + 'static;
    type Args: for<'a> Deserialize<'a> + WasmCompatSend + WasmCompatSync;
    type Output: Serialize;

    /// A method returning the name of the tool (default: NAME.to_string()).
    fn name(&self) -> String { Self::NAME.to_string() }

    /// A method returning the tool definition.
    fn definition(&self, _prompt: String)
        -> impl Future<Output = ToolDefinition> + WasmCompatSend + WasmCompatSync;

    /// The tool execution method.
    fn call(&self, args: Self::Args)
        -> impl Future<Output = Result<Self::Output, Self::Error>> + WasmCompatSend;
}
```

### 5.2 How Existing Wrappers Implement Tool

All existing wrappers follow the same pattern. Here's the annotated `FileToolFacadeWrapper` as canonical example:

```rust
// codelet/tools/src/facade/wrapper.rs:377-551
impl Tool for FileToolFacadeWrapper {
    // Dummy const — overridden by name() method below
    const NAME: &'static str = "file_facade_wrapper";

    // All wrappers use ToolError (from codelet-tools)
    type Error = ToolError;

    // All wrappers accept raw JSON via FacadeArgs
    type Args = FacadeArgs;  // = newtype around serde_json::Value

    // Each wrapper has its own result type
    type Output = FileOperationResult;

    /// CRITICAL: Override to return the FACADE's dynamic name.
    /// This is how different facades get different tool names despite
    /// sharing the same wrapper struct.
    fn name(&self) -> String {
        self.facade.tool_name().to_string()  // e.g., "read_file", "replace"
    }

    /// Return facade-specific schema as rig's ToolDefinition.
    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        let facade_def = self.facade.definition();
        RigToolDefinition {
            name: facade_def.name,
            description: facade_def.description,
            parameters: facade_def.parameters,
        }
    }

    /// The main execution pipeline.
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 1. Pre-tool hook check (HOOK-013)
        check_pre_tool_hook(self.session_id, &self.name(), &args.0)?;

        // 2. Map provider JSON → Internal*Params via facade
        let internal_params = self.facade.map_params(args.0)?;

        // 3. Dispatch to base tool based on variant
        match internal_params {
            InternalFileParams::Read { .. } => { /* ReadTool */ }
            InternalFileParams::Write { .. } => { /* WriteTool */ }
            InternalFileParams::Edit { .. } => { /* EditTool */ }
        }
    }
}
```

### 5.3 FacadeArgs — The Universal Args Type

```rust
// codelet/tools/src/facade/wrapper.rs:36-37
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacadeArgs(pub Value);
```

This is a newtype around `serde_json::Value` that implements `Deserialize`. When rig's `ToolDyn` blanket implementation calls `serde_json::from_str(&args)`, it deserializes the LLM's raw JSON parameters into `FacadeArgs(Value)`.

### 5.4 RhaiToolFacadeAdapter Tool Implementation

```rust
impl Tool for RhaiToolFacadeAdapter {
    // Dummy const — overridden by name()
    const NAME: &'static str = "rhai_tool_adapter";

    type Error = ToolError;
    type Args = FacadeArgs;
    type Output = String;  // JSON string result

    fn name(&self) -> String {
        self.tool_def.name.clone()
    }

    async fn definition(&self, _prompt: String) -> RigToolDefinition {
        RigToolDefinition {
            name: self.tool_def.name.clone(),
            description: self.tool_def.description.clone(),
            parameters: self.tool_def.parameters.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 1. HOOK-013: Pre-tool hook check
        check_pre_tool_hook(self.session_id, &self.name(), &args.0)?;

        // 2. Parameter transformation
        let mapped_params = if self.has_map_fn {
            // Call Rhai map_tool_params() in a blocking task
            let engine = self.engine.clone();
            let ast = self.ast.clone();
            let config = self.config.clone();
            let tool_name = self.tool_def.name.clone();
            let maps_to = self.tool_def.maps_to.clone();
            let params = args.0.clone();

            tokio::task::spawn_blocking(move || {
                call_map_tool_params(&engine, &ast, config, &tool_name, &maps_to, params)
            })
            .await
            .map_err(|e| ToolError::Execution {
                tool: "rhai_provider",
                message: format!("spawn_blocking failed: {e}"),
            })??
        } else {
            args.0  // Pass through — let default_to_internal handle it
        };

        // 3. Build Internal*Params from mapped params
        let internal = default_to_internal(&self.tool_def.maps_to, &mapped_params)?;

        // 4. Execute base tool
        execute_internal(internal, self.session_id).await
    }
}
```

### 5.5 Key Design Decisions

**Q: Why `type Output = String` instead of a specific result type?**

Because each `maps_to` target has a different result type (`FileOperationResult`, `BashOperationResult`, etc.). The adapter serializes all results to JSON strings, matching how `ToolDyn` works (which calls `serde_json::to_string(&output)`).

**Q: Why `Arc<Engine>` and `Arc<AST>`?**

Rhai's `Engine` is `Sync` but not `Clone`. The AST is `Clone` but expensive to clone. Using `Arc` allows all `RhaiToolFacadeAdapter` instances for the same script to share the engine and AST without cloning.

**Q: Why `tokio::task::spawn_blocking` for Rhai calls?**

Rhai execution is synchronous. The existing `ScriptedOAuthProvider` (in `codelet/providers/src/oauth/script_provider.rs` lines 113-124) establishes this pattern:

```rust
// codelet/providers/src/oauth/script_provider.rs:113-124
pub async fn build_authorization_request(&self) -> Result<Map> {
    let engine = self.engine.clone();
    let ast = self.ast.clone();
    let config = self.config_map();

    tokio::task::spawn_blocking(move || -> Result<Map> {
        let mut scope = Scope::new();
        let result: Dynamic = engine
            .call_fn(&mut scope, &ast, "build_authorization_request", (config,))
            .map_err(|e| anyhow!("build_authorization_request failed: {e}"))?;
        result.try_cast::<Map>().ok_or_else(|| anyhow!("must return a Map"))
    })
    .await
    .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
}
```


---

## 6. Pre-tool Hook Integration

### 6.1 The check_pre_tool_hook() Function

Every wrapper calls `check_pre_tool_hook()` at the top of its `call()` method. This is defined in `codelet/tools/src/facade/wrapper.rs` (lines 570-581):

```rust
// codelet/tools/src/facade/wrapper.rs:570-581
fn check_pre_tool_hook(session_id: Uuid, tool_name: &str, args: &Value) -> Result<(), ToolError> {
    match pre_tool_hook_check(session_id, tool_name, args) {
        Ok(PreToolHookDecision::Allow | PreToolHookDecision::Continue) => Ok(()),
        Ok(PreToolHookDecision::Deny(reason)) | Err(reason) => {
            Err(ToolError::Blocked {
                tool: "pre_tool_use_hook",
                message: reason,
            })
        }
    }
}
```

### 6.2 PreToolHookDecision Enum

From `codelet/tools/src/pre_tool_hook.rs` (lines 18-26):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreToolHookDecision {
    Allow,           // Hook says: allow (skip permission prompts)
    Deny(String),    // Hook says: deny with reason
    Continue,        // Hook says: no opinion — proceed normally
}
```

### 6.3 Handler Registration

Handlers are registered per-session via a global `RwLock<HashMap<Uuid, PreToolHookHandler>>`:

```rust
// codelet/tools/src/pre_tool_hook.rs:32-36
pub type PreToolHookHandler =
    std::sync::Arc<dyn Fn(Uuid, &str, &Value) -> PreToolHookDecision + Send + Sync>;

static SESSION_HANDLERS: RwLock<Option<HashMap<Uuid, PreToolHookHandler>>> = RwLock::new(None);
```

### 6.4 How RhaiToolFacadeAdapter Must Integrate

The adapter MUST call `check_pre_tool_hook()` before any tool execution, passing:
- `self.session_id` — the session UUID
- `&self.name()` — the Rhai-defined tool name (NOT "rhai_tool_adapter")
- `&args.0` — the raw JSON parameters from the LLM

This is already shown in the implementation in Section 5.4. The critical point is that the **tool name passed to the hook is the Rhai-defined name** (e.g., "execute_code"), not the maps_to target or the const NAME.

### 6.5 Other Cross-Cutting Concerns

The `RhaiToolFacadeAdapter` must also integrate with:

1. **BLOCK-006: Block notifications** — When file writes or bash commands are blocked, `emit_block_notification()` should be called (from `wrapper.rs:691-694`)
2. **TOOL-014: Path validation** — `validate_and_resolve_path()` must be called for file/search/ls operations (from `wrapper.rs:720-729`)
3. **Stage permissions** — `check_write_permission()` must be called before file writes/edits (from `wrapper.rs:469`)
4. **Session isolation** — `get_effective_cwd()` / `get_isolation_context()` for worktree isolation

Since `execute_internal()` will reuse the same dispatch logic as the existing wrappers, these concerns are automatically handled if the shared functions are called.

---

## 7. Summary: Implementation Roadmap

### Phase 1: Core Types
1. Define `RhaiToolDef` struct
2. Define `InternalParamsVariant` enum
3. Implement `parse_tool_definitions()` (Rhai Dynamic → RhaiToolDef)

### Phase 2: Default Mapping
4. Implement `default_to_internal()` for all 21 maps_to targets
5. Implement `execute_internal()` reusing existing wrapper dispatch logic

### Phase 3: Rhai Integration
6. Implement `call_map_tool_params()` with `spawn_blocking`
7. Implement `RhaiToolFacadeAdapter` struct with `rig::Tool`

### Phase 4: Presets
8. Implement `preset_tool_definitions()` for all 5 presets
9. Extract tool definitions from existing facade structs

### Phase 5: Registration
10. Implement factory function to create `Vec<RhaiToolFacadeAdapter>` from a provider config
11. Integration with agent builder (how adapters are registered with rig)

---

## Appendix A: File Location Index

| File | Purpose | Key Types |
|---|---|---|
| `codelet/tools/src/facade/traits.rs` | All facade traits + Internal*Params | `ToolFacade`, `FileToolFacade`, `BashToolFacade`, `SearchToolFacade`, `LsToolFacade`, `ExecToolFacade`, `HitlToolFacade` |
| `codelet/tools/src/facade/wrapper.rs` | All wrapper impls (rig::Tool) | `FacadeToolWrapper`, `FileToolFacadeWrapper`, `BashToolFacadeWrapper`, `SearchToolFacadeWrapper`, `LsToolFacadeWrapper`, `FspecToolFacadeWrapper`, `BridgeToolFacadeWrapper`, `ExecToolFacadeWrapper`, `HitlToolFacadeWrapper` |
| `codelet/tools/src/facade/zai.rs` | ZAI facade impls (reference) | `ZAIReadFileFacade`, `ZAIWriteFileFacade`, etc. |
| `codelet/tools/src/facade/codex.rs` | Codex facade impls (complex) | `CodexShellCommandFacade`, `CodexExecCommandFacade`, etc. |
| `codelet/tools/src/facade/param_extract.rs` | Parameter extraction helpers | `extract_required_string`, `extract_optional_uint`, etc. |
| `codelet/tools/src/facade/fspec_facade.rs` | Fspec facades + InternalFspecParams | `ClaudeFspecFacade`, `GeminiFspecFacade`, etc. |
| `codelet/tools/src/facade/bridge_facade.rs` | Bridge facades + InternalBridgeParams | `ClaudeBridgeFacade`, `GeminiBridgeFacade`, etc. |
| `codelet/tools/src/facade/file_ops.rs` | Gemini file facades | `GeminiReadFileFacade`, `GeminiWriteFileFacade`, `GeminiReplaceFacade` |
| `codelet/tools/src/facade/bash.rs` | Gemini bash facade | `GeminiRunShellCommandFacade` |
| `codelet/tools/src/facade/ls.rs` | Gemini ls facade | `GeminiListDirectoryFacade` |
| `codelet/tools/src/facade/search.rs` | Gemini search facades | `GeminiSearchFileContentFacade`, `GeminiGlobFacade` |
| `codelet/tools/src/facade/web_search.rs` | Web search facades | `ClaudeWebSearchFacade`, `GeminiGoogleWebSearchFacade`, etc. |
| `codelet/tools/src/facade/registry.rs` | ProviderToolRegistry | Registration of web search facades |
| `codelet/tools/src/facade/fspec_registration.rs` | Fspec tool factories | `claude_fspec_tool()`, `fspec_tool_for_provider()`, etc. |
| `codelet/tools/src/facade/mod.rs` | Module root + re-exports | Architecture diagram |
| `codelet/tools/src/pre_tool_hook.rs` | Pre-tool hook mechanism | `PreToolHookDecision`, `pre_tool_hook_check()` |
| `codelet/providers/src/oauth/engine.rs` | Rhai engine factory | `build_sandboxed_engine()`, `RhaiModule` |
| `codelet/providers/src/oauth/building_blocks.rs` | Rhai building block modules | `json_value_to_dynamic()`, `dynamic_to_json_value()` |
| `codelet/providers/src/oauth/script_provider.rs` | ScriptedOAuthProvider (reference) | `ScriptedOAuthProvider`, `ScriptProviderConfig` |
| `codelet/patches/rig-core/src/tool/mod.rs` | rig Tool trait | `Tool`, `ToolDyn`, `ToolDefinition` |
| `codelet/patches/rig-core/src/completion/request.rs` | rig ToolDefinition | `ToolDefinition { name, description, parameters }` |

## Appendix B: Rhai API Reference

### Key Types (from `/tmp/rhai/src/lib.rs`)

```rust
pub type Map = std::collections::BTreeMap<Identifier, Dynamic>;  // line 304
pub type Array = Vec<Dynamic>;                                     // line 289
```

### Engine::call_fn (from `/tmp/rhai/src/api/call_fn.rs`)

```rust
// line 126
pub fn call_fn<T: Variant + Clone>(
    &self,
    scope: &mut Scope,
    ast: &AST,
    fn_name: impl AsRef<str>,
    args: impl FuncArgs,
) -> RhaiResultOf<T>
```

### Dynamic Conversions (from `codelet/providers/src/oauth/building_blocks.rs`)

```rust
fn json_value_to_dynamic(value: &serde_json::Value) -> Dynamic   // line 216
fn dynamic_to_json_value(value: &Dynamic) -> serde_json::Value   // line 244
```

### Sandboxed Engine (from `codelet/providers/src/oauth/engine.rs`)

```rust
pub fn build_sandboxed_engine(modules: Vec<RhaiModule>) -> Engine  // line 42
// Safety limits: 50,000 ops, 32 call levels, 1MB strings, 10K arrays/maps
```

### ScriptedOAuthProvider Pattern (from `codelet/providers/src/oauth/script_provider.rs`)

```rust
pub fn load(script_path: &Path, config: ScriptProviderConfig) -> Result<Self>  // line 53
// 1. build_default_engine()
// 2. engine.compile(&script_content)
// 3. Store engine (Arc<Engine>) + ast (AST) + config
```
