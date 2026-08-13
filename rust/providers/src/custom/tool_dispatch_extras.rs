//! Dispatch helpers for the less-commonly-used `maps_to` categories
//! (`fspec`, `bridge`, `exec:run`, `hitl`).
//!
//! Extracted from [`super::tool_dispatch`] so that file stays within the
//! 300-line project limit. Each helper deserialises the raw Rhai params
//! JSON into the canonical `Internal*Params` shape, returning a
//! [`CustomProviderError::RhaiRuntimeError`] on failure.

use serde::Deserialize;
use serde_json::Value;

use codelet_tools::facade::{
    InternalBridgeParams, InternalExecParams, InternalFspecParams, InternalHitlParams,
};
use codelet_tools::request_user_input::HitlQuestion;

use super::error::CustomProviderError;

fn fail(category: &str, source: impl std::fmt::Display) -> CustomProviderError {
    CustomProviderError::RhaiRuntimeError(format!("default {category} mapping failed: {source}"))
}

fn parse<T: for<'de> Deserialize<'de>>(
    category: &str,
    params: &Value,
) -> Result<T, CustomProviderError> {
    serde_json::from_value(params.clone()).map_err(|e| fail(category, e))
}

fn default_args() -> String {
    "{}".to_string()
}

fn default_project_root() -> String {
    ".".to_string()
}

pub(super) fn fspec(params: &Value) -> Result<InternalFspecParams, CustomProviderError> {
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

pub(super) fn bridge(params: &Value) -> Result<InternalBridgeParams, CustomProviderError> {
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

pub(super) fn exec_run(params: &Value) -> Result<InternalExecParams, CustomProviderError> {
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

pub(super) fn hitl(params: &Value) -> Result<InternalHitlParams, CustomProviderError> {
    #[derive(Deserialize)]
    struct Shape {
        questions: Vec<HitlQuestion>,
    }
    let s: Shape = parse("hitl", params)?;
    Ok(InternalHitlParams::Request {
        questions: s.questions,
    })
}
