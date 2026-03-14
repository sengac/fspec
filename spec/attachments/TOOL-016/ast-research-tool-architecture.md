# AST Research: Unified Exec Tool Architecture

## Existing Tool Implementations (impl Tool for X)

Found 13 Tool implementations in codelet/tools/src/:
- `FspecTool` (fspec.rs:64)
- `BridgeTool` (bridge.rs:233)
- `DeepSearchTool` (deep_search/mod.rs:249)
- `InjectSummaryTool` (inject_summary.rs:126)
- `FacadeToolWrapper` (facade/wrapper.rs:61) — web search facades
- `FileToolFacadeWrapper` (facade/wrapper.rs:252) — file ops facades
- `FspecToolFacadeWrapper` (facade/wrapper.rs:856) — fspec facades
- `BashToolFacadeWrapper` (facade/wrapper.rs:968) — bash facades
- `SearchToolFacadeWrapper` (facade/wrapper.rs:1091) — grep/glob facades
- `LsToolFacadeWrapper` (facade/wrapper.rs:1287) — ls facades
- `BridgeToolFacadeWrapper` (facade/wrapper.rs:1462) — bridge facades
- `SessionSearchTool` (session_search/mod.rs:61)
- `WebSearchTool` (web_search.rs:340)

## Existing Facade Traits (trait XFacade: Send + Sync)

Found 11 facade traits:
- `ToolFacade` (traits.rs:103) — web search
- `FileToolFacade` (traits.rs:124) — file read/write/edit
- `BashToolFacade` (traits.rs:161) — bash/shell commands
- `FspecToolFacade` (traits.rs:204) — fspec operations
- `SearchToolFacade` (traits.rs:225) — grep/glob
- `LsToolFacade` (traits.rs:265) — directory listing
- `BridgeToolFacade` (bridge_facade.rs:28) — bridge operations
- `SystemPromptFacade` (system_prompt.rs:215)
- `HistoryPreparationFacade` (gemini_history.rs:52)
- `TurnCompletionFacade` (gemini_history.rs:206)
- `FspecToolFacade` (fspec_facade.rs:24)

## Existing Internal Param Enums

- `InternalWebSearchParams` (traits.rs:23) — Search, OpenPage, FindInPage, CaptureScreenshot
- `InternalFileParams` (traits.rs:77) — Read, Write, Edit
- `InternalBashParams` (traits.rs:144) — Execute { command, cwd, timeout_ms }
- `InternalSearchParams` (traits.rs:181) — Grep, Glob
- `InternalLsParams` (traits.rs:245) — List { path, offset, limit, depth }
- `InternalBridgeParams` (bridge_facade.rs:15) — Connect, Disconnect, List

## Pattern: New ExecToolFacade will follow

New trait `ExecToolFacade` needs:
- `InternalExecParams` enum with Run, Write, Poll, List, Close variants
- `ExecToolFacadeWrapper` delegating to `UnifiedExecTool`
- ProcessStore as global static (no existing equivalent — BashTool is stateless)

## BashTool Analysis (bash.rs)

- `BashTool` struct: only field is `session_id: uuid::Uuid` for worktree isolation
- `BashArgs`: `command: String`, `cwd: Option<String>`
- One-shot execution: spawn → read stdout/stderr → wait → return
- Uses `ProcessGroupKiller` for Unix cleanup
- Blocklist check via `check_bash_command()`
- Streaming via `call_with_streaming()` with `StreamCallback`
- No session management, no ProcessStore

## Codex Facade Analysis (facade/codex.rs)

Existing Codex facades:
- `CodexShellCommandFacade` → `BashToolFacade` (shell_command)
- `CodexReadFileFacade` → `FileToolFacade` (read_file)
- `CodexListDirFacade` → `LsToolFacade` (list_dir)
- `CodexGrepFilesFacade` → `SearchToolFacade` (grep_files)
- `CodexViewImageFacade` → `FileToolFacade` (view_image)

Missing (to be added by BUG-114/BUG-115, depends on TOOL-016):
- `CodexShellFacade` → `ExecToolFacade` (shell — argv execvp)
- `CodexExecCommandFacade` → `ExecToolFacade` (exec_command — PTY)
- `CodexWriteStdinFacade` → `ExecToolFacade` (write_stdin — session input)

## ToolError Enum (error.rs)

Variants available:
- `Validation { tool, message }`
- `Execution { tool, message }`
- `Blocked { tool, message }`
- Plus others for IO, serialization, etc.

## Key Insight: No ProcessStore Exists Yet

The codebase has no process/session store. BashTool is completely stateless.
UnifiedExecTool will be the first tool to maintain cross-call state via ProcessStore.
