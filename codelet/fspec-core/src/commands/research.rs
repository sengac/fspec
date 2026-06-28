//! `research` — Rust port of `src/commands/research.ts` (RPC-286, LIST-only).
//!
//! Feature: spec/features/research-rust-port.feature
//!
//! ## Scope (per supervisor decision — Option 1, LIST-only)
//!
//! Only the **list / status** behaviour is ported. The EXECUTE path (a
//! research tool is selected via `--tool`) in the TypeScript original spawns
//! child processes (`child_process.spawn`), performs network I/O (perplexity /
//! jira / confluence / stakeholder), and dynamically imports JS plugins — all
//! genuinely async / un-portable under the single-poll `poll_sync_future`
//! dispatcher. That path is DEFERRED, the same class as `reverse`'s
//! dispatcher-only path.
//!
//! What IS ported:
//!   * LIST mode (no `tool`): enumerate the five bundled registry tools
//!     (`ast`, `perplexity`, `jira`, `confluence`, `stakeholder`) with a
//!     resolved `configured` status, using the same config-resolution
//!     precedence as the TS `resolveConfig` (ENV → user `~/.fspec/
//!     fspec-config.json` → project `spec/fspec-config.json` → defaults).
//!     BLOCKING `std::fs` + static tables only.
//!   * Pre-execution validation: a selected tool that is not in the bundled
//!     registry is rejected with the same 3-line message the TS
//!     `getResearchTool` throws (`Research tool not found: <name>` + the
//!     `Available bundled tools: …` list + the custom-tool hint), surfaced as
//!     `FspecCoreError::InvalidArgs{command:"research", reason:…}` BEFORE any
//!     work — mirroring the TS `src/research-tools/registry.ts:88-95` guard.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single `run` — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## Environment assumption
//!
//! Config resolution reads `~/.fspec/fspec-config.json` and research env vars
//! (PERPLEXITY_API_KEY, JIRA_URL, …). The dispatcher-contract tests assume a
//! clean environment (no research env vars, no `research` block in the user
//! config). Documented in the test headers; flagged for CI.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;

/// A single research tool in the bundled registry. The pair `(name,
/// required-fields)` mirrors the TS `TOOL_REGISTRY`
/// (`src/commands/research-tool-list.ts:29-56`); a tool is *configured* iff
/// every required field resolves to a non-empty value.
struct RegistryTool {
    name: &'static str,
    description: &'static str,
    /// Config keys that must all resolve non-empty for the tool to be
    /// reported as configured (`[]` ⇒ always configured, e.g. `ast`).
    required: &'static [&'static str],
}

/// The bundled research-tool registry, in TS `TOOL_REGISTRY` declaration
/// order (`perplexity`, `jira`, `confluence`, `stakeholder`, `ast`). The list
/// scenarios assert membership (and a sorted set), so order here only affects
/// the rendered text ordering — kept matching the TS object insertion order.
const REGISTRY: &[RegistryTool] = &[
    RegistryTool {
        name: "perplexity",
        description: "Perplexity AI research tool for web search and AI-powered answers",
        required: &["apiKey"],
    },
    RegistryTool {
        name: "jira",
        description: "Jira integration for querying issues and project data",
        required: &["url", "token"],
    },
    RegistryTool {
        name: "confluence",
        description: "Confluence integration for searching documentation and wiki pages",
        required: &["url", "token"],
    },
    RegistryTool {
        name: "stakeholder",
        description: "Stakeholder communication tool for Teams/Slack/Discord",
        required: &["teamsWebhook"],
    },
    RegistryTool {
        name: "ast",
        description: "AST code analysis tool for pattern detection and deep code analysis",
        required: &[],
    },
];

/// Per-tool environment-variable mappings, mirroring the TS `ENV_VAR_MAPPINGS`
/// table (`src/utils/config-resolution.ts:51-68`). Used both as the highest-
/// precedence config layer and to render setup guidance for unconfigured
/// tools.
fn env_mappings(tool: &str) -> &'static [(&'static str, &'static str)] {
    match tool {
        "perplexity" => &[
            ("apiKey", "PERPLEXITY_API_KEY"),
            ("model", "PERPLEXITY_MODEL"),
        ],
        "jira" => &[("url", "JIRA_URL"), ("token", "JIRA_TOKEN")],
        "confluence" => &[("url", "CONFLUENCE_URL"), ("token", "CONFLUENCE_TOKEN")],
        "stakeholder" => &[
            ("teamsWebhook", "TEAMS_WEBHOOK_URL"),
            ("slackWebhook", "SLACK_WEBHOOK_URL"),
        ],
        _ => &[],
    }
}

/// CLI arguments accepted by `research`. Today only `tool` is meaningful at
/// the LIST-only dispatcher surface: when present and unknown it triggers the
/// not-found guard; when present and known the EXECUTE path is out of scope
/// (see module docs); when absent the tool listing is produced. `all`,
/// `query`, `workUnit`, `attach` are accepted (parity with the TS
/// `ResearchOptions`) but unused in LIST mode — declared so unknown-field
/// strictness never rejects a forwarded flag.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ResearchArgs {
    /// Research tool to execute. Omit to list available tools.
    tool: Option<String>,
    /// Show all tools (configured + unconfigured). Reserved; LIST mode always
    /// enumerates the full registry regardless of this flag (the dispatcher
    /// tests assert all five tools are present), matching the CLI `--help`
    /// fixture which lists every tool.
    #[allow(dead_code)]
    all: Option<bool>,
    #[allow(dead_code)]
    query: Option<String>,
    #[allow(dead_code)]
    work_unit: Option<String>,
    #[allow(dead_code)]
    attach: Option<bool>,
    /// Test-only override for the user config path (mirrors the TS
    /// `userConfigPath` option). When `None`, `~/.fspec/fspec-config.json` is
    /// used.
    #[allow(dead_code)]
    user_config_path: Option<String>,
}

/// One tool's resolved listing entry. Serialised into the dispatcher data
/// envelope's `tools` array; the dispatcher-contract tests read `name` and
/// `configured`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolStatus {
    name: String,
    description: String,
    configured: bool,
    /// `"✓"` when configured, `"✗"` otherwise — parity with the TS
    /// `statusIndicator`.
    status_indicator: String,
    /// `fspec research --tool=<name> <args>` — the per-tool usage line the
    /// CLI listing prints.
    usage: String,
    /// First-line setup hint for unconfigured tools (`export ENVVAR="..."`),
    /// mirroring the TS `configGuidance`. `None` for configured tools (parity
    /// with the TS `if (tool.configGuidance)` guard in the CLI listing).
    #[serde(skip_serializing_if = "Option::is_none")]
    config_guidance: Option<String>,
}

/// Dispatcher / CLI entry point. The caller passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so the
/// same binary can serve multiple sessions / working directories safely.
///
/// Returns a JSON envelope `{ "tools": [ ... ], "executed": false,
/// "discoveryMethod": "registry" }` for the LIST path. The unknown-tool guard
/// returns `Err(InvalidArgs)` so the dispatcher surfaces an `error` substring
/// and the CLI bridge maps it to exit 1 + stderr.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ResearchArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "research",
            reason: format!("failed to parse args: {e}"),
        })?;

    // EXECUTE-path guard: a selected tool that is not in the bundled registry
    // is rejected with the same 3-line message the TS `getResearchTool` throws
    // for an unknown tool (`src/research-tools/registry.ts:90-94`). The CLI
    // bridge prints the message verbatim to stderr and exits 1.
    if let Some(tool) = args.tool.as_deref() {
        if !REGISTRY.iter().any(|t| t.name == tool) {
            // Bundled tool names in the TS `BUNDLED_TOOLS` Map insertion order
            // (`ast, perplexity, jira, confluence, stakeholder`), which differs
            // from this module's REGISTRY render order.
            let bundled = "ast, perplexity, jira, confluence, stakeholder";
            return Err(FspecCoreError::InvalidArgs {
                command: "research",
                reason: format!(
                    "Research tool not found: {tool}\n\n\
                     Available bundled tools: {bundled}\n\
                     To use custom tools: create spec/research-tools/{tool}.ts \
                     and run 'fspec build-tool {tool}'"
                ),
            });
        }
        // Known tool → EXECUTE path is deferred (network/child-process/NAPI).
        return Err(FspecCoreError::InvalidArgs {
            command: "research",
            reason: format!(
                "Research tool execution is not yet ported to the Rust binary \
                 (tool '{tool}'); use the listing mode (omit --tool) for now"
            ),
        });
    }

    // LIST mode: enumerate the full bundled registry with resolved status.
    let user_config = args.user_config_path.as_deref();
    let tools: Vec<ToolStatus> = REGISTRY
        .iter()
        .map(|t| resolve_status(t, project_root, user_config))
        .collect();

    let envelope = json!({
        "tools": tools,
        "executed": false,
        "discoveryMethod": "registry",
    });

    serde_json::to_string(&envelope).map_err(|e| FspecCoreError::InvalidArgs {
        command: "research",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Resolve the configured status for a single registry tool by merging the
/// config layers (defaults → project → user → ENV) and checking that every
/// required field is present and non-empty.
fn resolve_status(
    tool: &RegistryTool,
    project_root: &Path,
    user_config: Option<&str>,
) -> ToolStatus {
    let config = resolve_config(tool.name, project_root, user_config);
    let configured = tool.required.iter().all(|field| {
        config
            .get(*field)
            .and_then(Value::as_str)
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    });
    // Setup guidance for unconfigured tools: `export ENVVAR="..."` for each
    // REQUIRED field that has an env mapping, joined by '\n' (parity with the
    // TS `envVarExamples.join('\n')`). Configured tools get `None`.
    let config_guidance = if configured {
        None
    } else {
        let examples: Vec<String> = env_mappings(tool.name)
            .iter()
            .filter(|pair| tool.required.contains(&pair.0))
            .map(|pair| format!("export {}=\"...\"", pair.1))
            .collect();
        if examples.is_empty() {
            None
        } else {
            Some(examples.join("\n"))
        }
    };
    ToolStatus {
        name: tool.name.to_string(),
        description: tool.description.to_string(),
        configured,
        status_indicator: if configured { "✓" } else { "✗" }.to_string(),
        usage: format!("fspec research --tool={} <args>", tool.name),
        config_guidance,
    }
}

/// Merge the configuration layers for `tool_name`, lowest → highest priority:
///   1. Defaults (only `perplexity.model = "sonar"`).
///   2. Project config `<project_root>/spec/fspec-config.json` → `.research[tool]`.
///   3. User config `~/.fspec/fspec-config.json` → `.research[tool]`
///      (overridable for tests via `user_config_path`).
///   4. Environment variables via the per-tool ENV mappings.
///
/// Mirrors the TS `resolveConfig` (`src/utils/config-resolution.ts:74-144`).
/// All reads are BLOCKING `std::fs`; malformed JSON is silently ignored
/// (parity with the TS `try { … } catch {}`).
fn resolve_config(tool_name: &str, project_root: &Path, user_config: Option<&str>) -> Value {
    let mut config = serde_json::Map::new();

    // Layer 4: defaults (lowest priority).
    if tool_name == "perplexity" {
        config.insert("model".to_string(), json!("sonar"));
    }

    // Layer 3: project config.
    let project_config_path = project_root.join("spec").join("fspec-config.json");
    merge_research_block(&mut config, &project_config_path, tool_name);

    // Layer 2: user config (higher priority).
    let user_config_path = match user_config {
        Some(p) => std::path::PathBuf::from(p),
        None => fspec_user_dir().join("fspec-config.json"),
    };
    merge_research_block(&mut config, &user_config_path, tool_name);

    // Layer 1: environment variables (highest priority).
    for (config_key, env_var) in env_mappings(tool_name) {
        if let Ok(val) = std::env::var(env_var) {
            config.insert((*config_key).to_string(), json!(val));
        }
    }

    Value::Object(config)
}

/// Read `path` as JSON and, if it contains `.research[tool_name]`, merge that
/// object's keys into `config` (later layers overwrite earlier ones).
/// Missing file / unreadable / invalid JSON are silently ignored.
fn merge_research_block(config: &mut serde_json::Map<String, Value>, path: &Path, tool_name: &str) {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed: Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(block) = parsed
        .get("research")
        .and_then(|r| r.get(tool_name))
        .and_then(Value::as_object)
    {
        for (k, v) in block {
            config.insert(k.clone(), v.clone());
        }
    }
}

/// Resolve `~/.fspec` (parity with the TS `getFspecUserDir`
/// = `join(homedir(), '.fspec')`). Falls back to a relative `.fspec` if the
/// home directory cannot be resolved — in that case the (almost certainly
/// absent) file simply doesn't merge, preserving defaults/ENV behaviour.
fn fspec_user_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .map(std::path::PathBuf::from)
        });
    match home {
        Some(h) => h.join(".fspec"),
        None => std::path::PathBuf::from(".fspec"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    /// Parse the `run` envelope and return its `tools` array.
    fn tools_of(data: &str) -> Vec<Value> {
        let v: Value = serde_json::from_str(data).unwrap();
        v["tools"].as_array().cloned().unwrap_or_default()
    }

    fn is_configured(tools: &[Value], name: &str) -> bool {
        tools
            .iter()
            .find(|t| t["name"].as_str() == Some(name))
            .and_then(|t| t["configured"].as_bool())
            .unwrap_or(false)
    }

    fn write_project_config(root: &Path, config: &Value) {
        let spec = root.join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::write(
            spec.join("fspec-config.json"),
            serde_json::to_string_pretty(config).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn list_mode_enumerates_all_five_tools() {
        let tmp = TempDir::new().unwrap();
        let out = futures_poll(run("{}", tmp.path()));
        let tools = tools_of(&out);
        let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec!["ast", "confluence", "jira", "perplexity", "stakeholder"]
        );
    }

    #[test]
    fn ast_is_configured_by_default_perplexity_is_not() {
        let tmp = TempDir::new().unwrap();
        let out = futures_poll(run("{}", tmp.path()));
        let tools = tools_of(&out);
        assert!(is_configured(&tools, "ast"));
        assert!(!is_configured(&tools, "perplexity"));
    }

    #[test]
    fn project_config_marks_perplexity_configured() {
        let tmp = TempDir::new().unwrap();
        write_project_config(
            tmp.path(),
            &json!({ "research": { "perplexity": { "apiKey": "pplx-test" } } }),
        );
        let out = futures_poll(run("{}", tmp.path()));
        let tools = tools_of(&out);
        assert!(is_configured(&tools, "perplexity"));
    }

    #[test]
    fn stakeholder_configured_when_webhook_present() {
        let tmp = TempDir::new().unwrap();
        write_project_config(
            tmp.path(),
            &json!({ "research": { "stakeholder": { "teamsWebhook": "https://example.test/hook" } } }),
        );
        let out = futures_poll(run("{}", tmp.path()));
        let tools = tools_of(&out);
        assert!(is_configured(&tools, "stakeholder"));
    }

    #[test]
    fn list_mode_creates_no_files() {
        let tmp = TempDir::new().unwrap();
        let _ = futures_poll(run("{}", tmp.path()));
        assert!(!tmp.path().join("spec/fspec-config.json").exists());
    }

    #[test]
    fn unknown_tool_is_rejected_with_not_found() {
        let tmp = TempDir::new().unwrap();
        let err = futures_poll_err(run(r#"{"tool":"does-not-exist"}"#, tmp.path()));
        let msg = err.to_string();
        assert!(
            msg.contains("Research tool not found: does-not-exist"),
            "msg={msg}"
        );
        // Parity with the TS 3-line message: bundled-tools list + custom-tool hint.
        assert!(
            msg.contains("Available bundled tools: ast, perplexity, jira, confluence, stakeholder"),
            "msg={msg}"
        );
        assert!(
            msg.contains("create spec/research-tools/does-not-exist.ts and run 'fspec build-tool does-not-exist'"),
            "msg={msg}"
        );
    }

    // Minimal single-poll driver mirroring the dispatcher's poll_sync_future:
    // these run functions never .await on a real resource, so one poll
    // resolves them.
    fn futures_poll<F: std::future::Future<Output = Result<String, FspecCoreError>>>(
        fut: F,
    ) -> String {
        drive(fut).expect("research run should succeed")
    }

    fn futures_poll_err<F: std::future::Future<Output = Result<String, FspecCoreError>>>(
        fut: F,
    ) -> FspecCoreError {
        drive(fut).expect_err("research run should fail")
    }

    fn drive<T, F: std::future::Future<Output = Result<T, FspecCoreError>>>(
        fut: F,
    ) -> Result<T, FspecCoreError> {
        use std::pin::pin;
        use std::task::{Context, Poll, Waker};
        let mut fut = pin!(fut);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("future returned Pending under single-poll driver"),
        }
    }
}
