//! Runtime dispatch for custom-provider `maps_to` targets (PROV-069).
//!
//! Extends the `default_to_internal_file` pattern with one
//! `default_to_internal_<category>` helper per remaining `maps_to`
//! category and a top-level [`default_to_internal`] router that converts
//! a `maps_to` identifier plus a raw `serde_json::Value` into a
//! type-safe [`DispatchedToolParams`] enum. All mapping errors surface
//! as [`CustomProviderError::RhaiRuntimeError`].

use serde::Deserialize;
use serde_json::Value;

use codelet_tools::facade::{
    InternalBashParams, InternalBridgeParams, InternalExecParams, InternalFileParams,
    InternalFspecParams, InternalHitlParams, InternalLsParams, InternalSearchParams,
    InternalWebSearchParams,
};
use codelet_tools::request_user_input::HitlQuestion;

use super::error::CustomProviderError;
use super::tool_facade::default_to_internal_file;
use super::tool_resolve::KNOWN_MAPS_TO;

/// Result of dispatching a custom provider tool-call's `maps_to` and
/// params through [`default_to_internal`]. Each variant wraps the
/// internal parameter type already exposed by `codelet_tools::facade`.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchedToolParams {
    /// Mapped from `file:read` / `file:write` / `file:edit`.
    File(InternalFileParams),
    /// Mapped from `bash`.
    Bash(InternalBashParams),
    /// Mapped from `search:grep` or `search:glob`.
    Search(InternalSearchParams),
    /// Mapped from `ls`.
    Ls(InternalLsParams),
    /// Mapped from `web_search:search`.
    WebSearch(InternalWebSearchParams),
    /// Mapped from `fspec`.
    Fspec(InternalFspecParams),
    /// Mapped from `bridge`.
    Bridge(InternalBridgeParams),
    /// Mapped from `exec:run`.
    Exec(InternalExecParams),
    /// Mapped from `hitl`.
    Hitl(InternalHitlParams),
}

/// Convert a `maps_to` identifier + Rhai-supplied params JSON into a
/// [`DispatchedToolParams`] variant. Unknown `maps_to` identifiers return
/// a [`CustomProviderError::RhaiRuntimeError`] whose message names the
/// offending value and lists every valid identifier from
/// [`KNOWN_MAPS_TO`].
pub fn default_to_internal(
    maps_to: &str,
    params: &Value,
) -> Result<DispatchedToolParams, CustomProviderError> {
    use DispatchedToolParams as D;
    match maps_to {
        "file:read" | "file:write" | "file:edit" => {
            Ok(D::File(default_to_internal_file(maps_to, params)?))
        }
        "bash" => Ok(D::Bash(bash(params)?)),
        "search:grep" => Ok(D::Search(search_grep(params)?)),
        "search:glob" => Ok(D::Search(search_glob(params)?)),
        "ls" => Ok(D::Ls(ls(params)?)),
        "web_search:search" => Ok(D::WebSearch(web_search(params)?)),
        "fspec" => Ok(D::Fspec(fspec(params)?)),
        "bridge" => Ok(D::Bridge(bridge(params)?)),
        "exec:run" => Ok(D::Exec(exec_run(params)?)),
        "hitl" => Ok(D::Hitl(hitl(params)?)),
        other => Err(unknown(other)),
    }
}

fn unknown(offender: &str) -> CustomProviderError {
    let valid = KNOWN_MAPS_TO.join(", ");
    CustomProviderError::RhaiRuntimeError(format!(
        "unknown maps_to '{offender}'; valid identifiers: {valid}"
    ))
}

fn fail(category: &str, source: impl std::fmt::Display) -> CustomProviderError {
    CustomProviderError::RhaiRuntimeError(format!(
        "default {category} mapping failed: {source}"
    ))
}

fn parse<T: for<'de> Deserialize<'de>>(
    category: &str,
    params: &Value,
) -> Result<T, CustomProviderError> {
    serde_json::from_value(params.clone()).map_err(|e| fail(category, e))
}

fn bash(params: &Value) -> Result<InternalBashParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        command: String,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
    }
    let s: Shape = parse("bash", params)?;
    Ok(InternalBashParams::Execute {
        command: s.command,
        cwd: s.cwd,
        timeout_ms: s.timeout_ms,
    })
}

fn search_grep(params: &Value) -> Result<InternalSearchParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        pattern: String,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        include: Option<String>,
        #[serde(default)]
        limit: Option<usize>,
    }
    let s: Shape = parse("search:grep", params)?;
    Ok(InternalSearchParams::Grep {
        pattern: s.pattern,
        path: s.path,
        include: s.include,
        limit: s.limit,
    })
}

fn search_glob(params: &Value) -> Result<InternalSearchParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        pattern: String,
        #[serde(default)]
        path: Option<String>,
    }
    let s: Shape = parse("search:glob", params)?;
    Ok(InternalSearchParams::Glob {
        pattern: s.pattern,
        path: s.path,
    })
}

fn ls(params: &Value) -> Result<InternalLsParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        offset: Option<usize>,
        #[serde(default)]
        limit: Option<usize>,
        #[serde(default)]
        depth: Option<usize>,
    }
    let s: Shape = parse("ls", params)?;
    Ok(InternalLsParams::List {
        path: s.path,
        offset: s.offset,
        limit: s.limit,
        depth: s.depth,
    })
}

fn web_search(params: &Value) -> Result<InternalWebSearchParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        query: String,
    }
    let s: Shape = parse("web_search:search", params)?;
    Ok(InternalWebSearchParams::Search { query: s.query })
}

fn default_args() -> String {
    "{}".to_string()
}

fn default_project_root() -> String {
    ".".to_string()
}

fn fspec(params: &Value) -> Result<InternalFspecParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        command: String,
        #[serde(default = "default_args")]
        args: String,
        #[serde(default = "default_project_root")]
        project_root: String,
    }
    let s: Shape = parse("fspec", params)?;
    Ok(InternalFspecParams {
        command: s.command,
        args: s.args,
        project_root: s.project_root,
    })
}

fn bridge(params: &Value) -> Result<InternalBridgeParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Action {
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        url: Option<String>,
    }
    #[derive(Deserialize)]
    struct Shape {
        action: Action,
    }
    let s: Shape = parse("bridge", params)?;
    match s.action.kind.as_str() {
        "connect" => {
            let url = s
                .action
                .url
                .ok_or_else(|| fail("bridge", "connect action missing 'url'"))?;
            Ok(InternalBridgeParams::Connect { url })
        }
        "disconnect" => {
            let url = s
                .action
                .url
                .ok_or_else(|| fail("bridge", "disconnect action missing 'url'"))?;
            Ok(InternalBridgeParams::Disconnect { url })
        }
        "list" => Ok(InternalBridgeParams::List),
        other => Err(fail("bridge", format!("unknown action.type '{other}'"))),
    }
}

fn exec_run(params: &Value) -> Result<InternalExecParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        command: Value,
        #[serde(default)]
        workdir: Option<String>,
        #[serde(default)]
        tty: bool,
        #[serde(default)]
        yield_time_ms: Option<u64>,
        #[serde(default)]
        max_output_tokens: Option<u64>,
        #[serde(default)]
        timeout_secs: Option<u64>,
    }
    let s: Shape = parse("exec:run", params)?;
    Ok(InternalExecParams::Run {
        command: s.command,
        workdir: s.workdir,
        tty: s.tty,
        yield_time_ms: s.yield_time_ms,
        max_output_tokens: s.max_output_tokens,
        timeout_secs: s.timeout_secs,
    })
}

fn hitl(params: &Value) -> Result<InternalHitlParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        questions: Vec<HitlQuestion>,
    }
    let s: Shape = parse("hitl", params)?;
    Ok(InternalHitlParams::Request { questions: s.questions })
}
