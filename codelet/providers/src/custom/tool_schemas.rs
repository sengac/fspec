//! JSON-schema fragments for built-in Rhai custom-provider tool presets.
//!
//! Extracted from [`super::tool_presets`] so that file stays within the
//! 300-line project limit. Each schema here mirrors the corresponding
//! [`codelet_tools`] args shape so the `default_to_internal` dispatch
//! can `serde_json::from_value` the LLM's arguments straight through.

use serde_json::json;

pub(crate) fn file_read_schema() -> serde_json::Value {
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

pub(crate) fn file_write_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "file_path": { "type": "string" },
            "content": { "type": "string" }
        },
        "required": ["file_path", "content"]
    })
}

pub(crate) fn file_edit_schema() -> serde_json::Value {
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

pub(crate) fn bash_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": "string" }
        },
        "required": ["command"]
    })
}

pub(crate) fn grep_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": { "type": "string" },
            "path": { "type": "string" }
        },
        "required": ["pattern"]
    })
}

pub(crate) fn glob_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": { "type": "string" },
            "path": { "type": "string" }
        },
        "required": ["pattern"]
    })
}

pub(crate) fn ls_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" }
        }
    })
}

pub(crate) fn web_search_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" }
        },
        "required": ["query"]
    })
}

/// AST-based code search schema (PROV-097). Mirrors
/// [`codelet_tools::astgrep::AstGrepArgs`] exactly so the Rhai
/// dispatch's `search:ast_grep` handler can deserialise the LLM's
/// arguments straight through.
pub(crate) fn ast_grep_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": {
                "type": "string",
                "description": "The AST pattern to search for (must be valid syntax for the target language). Use $NAME for single-node wildcards, $$$ARGS for multi-node wildcards."
            },
            "language": {
                "type": "string",
                "description": "Programming language to search. Supported: rust, typescript, tsx, javascript, python, go, java, c, cpp, csharp, ruby, kotlin, swift, scala, php, bash, html, css, json, yaml, lua, elixir, haskell, dart, solidity, nix, hcl."
            },
            "path": {
                "type": "string",
                "description": "Directory or file to search in (optional, defaults to current directory)"
            }
        },
        "required": ["pattern", "language"]
    })
}
