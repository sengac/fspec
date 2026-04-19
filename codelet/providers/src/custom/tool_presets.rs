//! Preset tool lists keyed by [`ToolStyle`] (PROV-066).
//!
//! When a custom provider script omits `define_tools`, or that function
//! returns an error, the resolver falls back to one of these presets
//! selected by `ProviderConfig.tool_style`. Each preset enumerates the
//! full baseline tool surface fspec exposes to agents.

use serde_json::json;

use super::config::ToolStyle;
use super::tool_facade::RhaiToolDef;

/// Build a default `RhaiToolDef` from `(name, description, maps_to,
/// parameters)`. Centralised so the per-style constructors stay terse.
fn td(
    name: &str,
    description: &str,
    maps_to: &str,
    parameters: serde_json::Value,
) -> RhaiToolDef {
    RhaiToolDef {
        name: name.to_string(),
        description: description.to_string(),
        parameters,
        maps_to: maps_to.to_string(),
    }
}

fn file_read_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "file_path": { "type": "string" },
            "offset": { "type": "integer" },
            "limit": { "type": "integer" }
        },
        "required": ["file_path"]
    })
}

fn file_write_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "file_path": { "type": "string" },
            "content": { "type": "string" }
        },
        "required": ["file_path", "content"]
    })
}

fn file_edit_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "file_path": { "type": "string" },
            "old_string": { "type": "string" },
            "new_string": { "type": "string" }
        },
        "required": ["file_path", "old_string", "new_string"]
    })
}

fn bash_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": "string" }
        },
        "required": ["command"]
    })
}

fn grep_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": { "type": "string" },
            "path": { "type": "string" }
        },
        "required": ["pattern"]
    })
}

fn glob_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": { "type": "string" },
            "path": { "type": "string" }
        },
        "required": ["pattern"]
    })
}

fn ls_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" }
        }
    })
}

fn web_search_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" }
        },
        "required": ["query"]
    })
}

/// Claude-native preset: PascalCase tool names.
fn claude_preset() -> Vec<RhaiToolDef> {
    vec![
        td("Read", "Read a file", "file:read", file_read_schema()),
        td("Write", "Write a file", "file:write", file_write_schema()),
        td("Edit", "Edit a file", "file:edit", file_edit_schema()),
        td("Bash", "Run a bash command", "bash", bash_schema()),
        td("Grep", "Search file contents", "search:grep", grep_schema()),
        td("Glob", "Find files by glob", "search:glob", glob_schema()),
        td("LS", "List a directory", "ls", ls_schema()),
        td(
            "WebSearch",
            "Search the web",
            "web_search:search",
            web_search_schema(),
        ),
    ]
}

/// OpenAI-native preset: snake_case tool names.
fn openai_preset() -> Vec<RhaiToolDef> {
    vec![
        td("read_file", "Read a file", "file:read", file_read_schema()),
        td(
            "write_file",
            "Write a file",
            "file:write",
            file_write_schema(),
        ),
        td("edit_file", "Edit a file", "file:edit", file_edit_schema()),
        td("run_bash", "Run a bash command", "bash", bash_schema()),
        td(
            "grep_search",
            "Search file contents",
            "search:grep",
            grep_schema(),
        ),
        td(
            "glob_search",
            "Find files by glob",
            "search:glob",
            glob_schema(),
        ),
        td("list_dir", "List a directory", "ls", ls_schema()),
        td(
            "web_search",
            "Search the web",
            "web_search:search",
            web_search_schema(),
        ),
    ]
}

/// Gemini-native preset: lowerCamelCase.
fn gemini_preset() -> Vec<RhaiToolDef> {
    vec![
        td(
            "readFile",
            "Read a file",
            "file:read",
            file_read_schema(),
        ),
        td(
            "writeFile",
            "Write a file",
            "file:write",
            file_write_schema(),
        ),
        td("replace", "Edit a file", "file:edit", file_edit_schema()),
        td(
            "runShellCommand",
            "Run a bash command",
            "bash",
            bash_schema(),
        ),
        td(
            "searchFileContent",
            "Search file contents",
            "search:grep",
            grep_schema(),
        ),
        td(
            "globSearch",
            "Find files by glob",
            "search:glob",
            glob_schema(),
        ),
        td(
            "listDirectory",
            "List a directory",
            "ls",
            ls_schema(),
        ),
        td(
            "googleSearch",
            "Search the web",
            "web_search:search",
            web_search_schema(),
        ),
    ]
}

/// Codex-native preset.
fn codex_preset() -> Vec<RhaiToolDef> {
    vec![
        td(
            "read_file",
            "Read a file",
            "file:read",
            file_read_schema(),
        ),
        td(
            "write_file",
            "Write a file",
            "file:write",
            file_write_schema(),
        ),
        td(
            "edit_file",
            "Edit a file",
            "file:edit",
            file_edit_schema(),
        ),
        td(
            "shell",
            "Run a shell command",
            "bash",
            bash_schema(),
        ),
        td(
            "grep_files",
            "Search file contents",
            "search:grep",
            grep_schema(),
        ),
        td(
            "find_files",
            "Find files by glob",
            "search:glob",
            glob_schema(),
        ),
        td("list_dir", "List a directory", "ls", ls_schema()),
        td(
            "web_search",
            "Search the web",
            "web_search:search",
            web_search_schema(),
        ),
    ]
}

/// Return the preset tool list for a given [`ToolStyle`].
///
/// `ToolStyle::Anthropic` is treated as an alias for the Claude preset
/// because `"anthropic"` was the legacy configuration spelling before
/// the dedicated `"claude"` variant existed.
pub fn preset_tools(style: ToolStyle) -> Vec<RhaiToolDef> {
    match style {
        ToolStyle::Claude | ToolStyle::Anthropic => claude_preset(),
        ToolStyle::Openai => openai_preset(),
        ToolStyle::Gemini => gemini_preset(),
        ToolStyle::Codex => codex_preset(),
    }
}
