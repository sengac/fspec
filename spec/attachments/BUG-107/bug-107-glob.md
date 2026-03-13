# BUG-107: Codex facade exposes non-native glob tool

## Problem

The Codex facade in `codelet/tools/src/facade/codex.rs` defined a `CodexGlobFacade` struct that exposed a `glob` tool. However, the Codex CLI native tool set (sourced from `codex-rs/core/src/tools/spec.rs`) does **not** define a `glob` tool.

## Verification (2026-03-13)

Fresh clone of https://github.com/openai/codex confirmed:
- `grep "glob" codex-rs/core/src/tools/spec.rs` → **no matches**
- No `create_glob_tool()` function exists anywhere in the codebase
- The `glob` crate in `Cargo.lock` is a Rust build dependency, not a tool definition
- The `codex-rs/file-search` crate is a fuzzy file-name matcher (like fzf using `nucleo`), not a glob tool — used internally by the rollout system, not exposed as a model tool

## Native Codex CLI Tool Inventory

From `codex-rs/core/src/tools/spec.rs` (verified 2026-03-13):

**Execution tools:**
- `shell_command` — shell script execution (String command)
- `shell` — raw exec via execvp (Array<String> command)
- `exec_command` — unified exec with PTY support
- `write_stdin` — send input to interactive sessions

**File tools:**
- `read_file` — read file with slice/indentation modes
- `list_dir` — directory listing with offset/limit/depth pagination
- `apply_patch` — freeform and JSON patch variants

**Search tools:**
- `grep_files` — content search with `include` glob filter and `limit`

**Other tools:**
- `view_image` — local image viewing
- `update_plan` — structured plan tracking
- `request_user_input` — user input prompts
- `request_permissions` — permission requests
- `spawn_agent`, `send_input`, `wait`, `close_agent`, `resume_agent` — multi-agent
- `js_repl`, `js_repl_reset`, `artifacts` — code execution
- `tool_search`, `tool_suggest` — tool discovery (MCP/plugins)

**`glob` does NOT appear in this list.**

## How Codex Does Glob-Like Functionality

The native Codex approach to file-pattern matching is via `grep_files` with the `include` parameter:

```rust
// From codex-rs/core/src/tools/handlers/grep_files.rs
if let Some(glob) = include {
    command.arg("--glob").arg(glob);
}
```

The `include` param is passed directly to `rg --glob`, which is ripgrep's glob filter. Example:
```json
{"pattern": ".", "include": "*.rs", "path": "/src"}
```

This finds all `.rs` files under `/src` — the exact functionality that a standalone `glob` tool would provide.

**IMPORTANT**: Our `grep_files` facade currently **silently drops** the `include` and `limit` params (see BUG-111). Until BUG-111 is fixed, the glob-equivalent functionality is broken.

## Fix Applied

1. ✅ Removed `CodexGlobFacade` struct and `SearchToolFacade` impl from `codelet/tools/src/facade/codex.rs`
2. ✅ Removed `CodexGlobFacade` export from `codelet/tools/src/facade/mod.rs`
3. ✅ Removed glob tool registration from `CodexProvider::create_rig_agent()` in `codelet/providers/src/codex/mod.rs`
4. ✅ Removed all glob-related tests (facade tests, schema tests, naming tests)
5. ✅ Updated feature file `spec/features/codex-native-tool-facades.feature` to remove glob scenario
6. ✅ Added negative test verifying glob is NOT in the Codex tool set

## Related Cards

- **BUG-111**: `grep_files` facade must map `include` and `limit` params (the glob-equivalent functionality)
- **BUG-108**: `shell_command` facade ignores `workdir` and `timeout_ms`
- **BUG-109**: `read_file` facade missing `mode` and `indentation` params
- **BUG-110**: `list_dir` facade missing `offset` and `limit` params
- **BUG-112**: Missing `view_image` tool
- **BUG-113**: Missing `update_plan` tool
- **BUG-114**: Missing `shell` and `exec_command` tools
- **BUG-115**: Missing `write_stdin` tool
- **BUG-116**: Missing `request_user_input` tool

## References

- Codex CLI repo: https://github.com/openai/codex
- Tool spec: `codex-rs/core/src/tools/spec.rs`
- grep_files handler: `codex-rs/core/src/tools/handlers/grep_files.rs`
- list_dir handler: `codex-rs/core/src/tools/handlers/list_dir.rs`
