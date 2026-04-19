#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
//! Shared helpers for PROV-066 custom-provider tool facade tests.
//!
//! Included via `#[path = "custom_tool_facades_test_helpers.rs"] mod helpers;`.
//!
//! These helpers build ProviderConfig instances in-memory (no JSON on disk
//! required) and wrap ScriptLoader + `.rhai` file creation so each test
//! stays short.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::TempDir;

use codelet_providers::custom::{
    ApiStyle, AuthConfig, ModelDef, ProviderConfig, ScriptLoader, ToolStyle,
};

/// Write a `.rhai` script body to `dir/filename` and return the path.
pub fn write_script(dir: &Path, filename: &str, body: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, body).expect("write script");
    path
}

/// Build a minimal `ProviderConfig` with a single model alias `"smart"`
/// pointing at a freshly-written script on disk. The returned TempDir
/// must be retained by the caller for the lifetime of the test.
///
/// Callers can override `tool_style` after construction by mutating the
/// returned config.
pub fn facade_config_with_script(
    name: &str,
    script_body: &str,
    tool_style: ToolStyle,
) -> (TempDir, ProviderConfig) {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_script(tmp.path(), "p.rhai", script_body);

    let mut models: HashMap<String, ModelDef> = HashMap::new();
    models.insert(
        "smart".to_string(),
        ModelDef {
            id: "model-smart-v2".to_string(),
            context_window: 128_000,
            max_output_tokens: 4096,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
        },
    );

    let cfg = ProviderConfig {
        name: name.to_string(),
        display_name: "My LLM".to_string(),
        base_url: "https://api.example.com".to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: AuthConfig::Bearer {
            env_var: "MY_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style,
        api_style: ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    (tmp, cfg)
}

/// Shared helper — build a default-engine ScriptLoader wrapped in `Arc`.
pub fn make_loader() -> Arc<ScriptLoader> {
    Arc::new(ScriptLoader::with_default_engine())
}

/// A minimal script body whose only purpose is to parse successfully.
/// Does NOT define `define_tools` or `map_tool_params`.
pub const NO_TOOL_FNS_SCRIPT: &str = r#"
fn unused() { 1 }
"#;

/// A script body defining `define_tools` that returns a single `read_file`
/// tool mapped to `file:read`.
pub const DEFINE_TOOLS_READ_FILE: &str = r#"
fn define_tools(config) {
    [
        #{
            name: "read_file",
            description: "Read a file",
            parameters: #{
                "type": "object",
                "properties": #{
                    "file_path": #{ "type": "string" }
                },
                "required": ["file_path"]
            },
            maps_to: "file:read"
        }
    ]
}
"#;

/// A script body defining `define_tools` returning two entries: a
/// `read_file` tool and a `bash` tool. Used to verify partial tool lists.
pub const DEFINE_TOOLS_TWO_ENTRIES: &str = r#"
fn define_tools(config) {
    [
        #{
            name: "read_file",
            description: "Read a file",
            parameters: #{ "type": "object" },
            maps_to: "file:read"
        },
        #{
            name: "run_bash",
            description: "Execute a bash command",
            parameters: #{ "type": "object" },
            maps_to: "bash"
        }
    ]
}
"#;

/// A script body whose `define_tools` unconditionally throws a runtime
/// error. Used to verify preset fallback semantics.
pub const DEFINE_TOOLS_RUNTIME_ERROR: &str = r#"
fn define_tools(config) {
    throw "define_tools boom";
}
"#;

/// A script body whose `define_tools` returns a tool with an unknown
/// maps_to identifier. Used to verify strict maps_to validation.
pub const DEFINE_TOOLS_UNKNOWN_MAPS_TO: &str = r#"
fn define_tools(config) {
    [
        #{
            name: "weird",
            description: "unknown",
            parameters: #{ "type": "object" },
            maps_to: "mystery:foo"
        }
    ]
}
"#;

/// A script body that defines `define_tools` returning one `read_file`
/// tool AND defines `map_tool_params` that renames `filepath` → `file_path`
/// for `file:read` maps_to targets.
pub const DEFINE_TOOLS_AND_RENAMING_MAP: &str = r#"
fn define_tools(config) {
    [
        #{
            name: "read_file",
            description: "Read a file",
            parameters: #{ "type": "object" },
            maps_to: "file:read"
        }
    ]
}
fn map_tool_params(config, tool_name, maps_to, params) {
    if maps_to == "file:read" {
        let out = #{};
        if params.contains("filepath") {
            out.file_path = params.filepath;
        }
        out
    } else {
        params
    }
}
"#;

/// A script body whose `map_tool_params` returns Rhai unit for every
/// invocation. Tests use this to assert default field-by-field mapping.
pub const DEFINE_TOOLS_AND_UNIT_MAP: &str = r#"
fn define_tools(config) {
    [
        #{
            name: "read_file",
            description: "Read a file",
            parameters: #{ "type": "object" },
            maps_to: "file:read"
        }
    ]
}
fn map_tool_params(config, tool_name, maps_to, params) {
    ()
}
"#;
