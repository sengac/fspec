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

use super::error::CustomProviderError;
use super::tool_dispatch_extras::{bridge, exec_run, fspec, hitl};
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
    /// Mapped from `search:ast_grep`.
    ///
    /// Not part of `InternalSearchParams` because the AST-grep tool is
    /// structural (pattern + language + optional path) rather than a
    /// plain text search, and adding it to that enum would ripple
    /// through every built-in provider's search facade. Instead we
    /// surface it as its own variant and forward directly to
    /// `codelet_tools::AstGrepTool` in the executor.
    AstGrep(AstGrepParams),
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

/// Dispatched params for the AST-grep tool (structural code search).
///
/// Mirrors [`codelet_tools::astgrep::AstGrepArgs`] — kept here as a
/// separate struct so the custom-provider dispatch owns its own
/// serde-less typed shape.
#[derive(Debug, Clone, PartialEq)]
pub struct AstGrepParams {
    /// Structural pattern (e.g. `fn $NAME($$$ARGS) { $$$BODY }`).
    pub pattern: String,
    /// Target language (e.g. `rust`, `typescript`, `python`).
    pub language: String,
    /// Optional directory / file to scope the search to.
    pub path: Option<String>,
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
        "search:ast_grep" => Ok(D::AstGrep(ast_grep(params)?)),
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
    CustomProviderError::RhaiRuntimeError(format!("default {category} mapping failed: {source}"))
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

fn ast_grep(params: &Value) -> Result<AstGrepParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        pattern: String,
        language: String,
        #[serde(default)]
        path: Option<String>,
    }
    let s: Shape = parse("search:ast_grep", params)?;
    Ok(AstGrepParams {
        pattern: s.pattern,
        language: s.language,
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
