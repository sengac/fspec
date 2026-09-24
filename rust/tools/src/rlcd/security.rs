//! RLCD-004 — semantic security layer over the regex blocklist.
//!
//! Feature: spec/features/rlcd-blocklist-security.feature
//!
//! The RLCD engine is a semantic SECOND-STAGE over the regex blocklist for
//! Bash and file operations. INVARIANT (call sites in `blocklist::middleware`
//! and the four file tools): the regex blocklist is ALWAYS evaluated first:
//!
//! - a regex Block rejects on its own reason and never consults RLCD;
//! - an explicit regex Allow rule is never staged. RLCD only stages ops the
//!   regex pass resolved to 'no rule matched' (checkMode `all`) or 'prompt +
//!   user allowed' (checkMode `all` | `prompt-only`).
//!
//! One noul question per op (`qid = "security"`): p >= block -> hard reject
//! (`BlockedError`, rule id `rlcd-security`); p >= prompt -> Triple pause
//! (AllowOnce / AllowSession `rlcd-check` / Deny -> "RLCD security: user
//! denied access"). Bash and file ops score on different model scales, so
//! each profile carries its own (block, prompt) thresholds (contract v2,
//! calibrated 2026-09-24 on the live 'typed-decisions' backend, 390-item
//! battery): bash 0.65/0.45, file 0.35/0.28 (config-overridable). The state
//! LEADS with the agent-harness context ("no human watching"), then the
//! machine-readable marker (tool= / command= / path= / operation=).
//! FAIL-OPEN (HITL 2026-09-24): unreachable engine, classify error,
//! disabled config or unknown checkMode -> skip (the regex blocklist +
//! stage permissions stay fully in force). Latency guard: per-session
//! counter `maxChecksPerSession` (0 = unlimited), one-time warn above cap.
//! Sync entry (Bash) bridges via `block_in_place` + `Handle::block_on`
//! (multi-thread guard, fail-open); the async entry serves the file tools.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use tracing::warn;
use uuid::Uuid;

use crate::blocklist::{allow_for_session, is_session_allowed, BlockedError};
use crate::rlcd::client::{HttpRlcdClient, RlcdClient, RlcdQuestion};
use crate::rlcd::config::load_rlcd_config;
use crate::rlcd::health::health_check;
use crate::rlcd::security_config::SecurityThresholds;
use crate::tool_pause::{pause_for_user, PauseKind, PauseRequest, PauseResponse};

/// The rule id surfaced in `BlockedError` for stage decisions.
pub const RLCD_SECURITY_RULE_ID: &str = "rlcd-security";
/// The single session-allowance pseudo-pattern for the stage.
pub const RLCD_CHECK_ALLOWANCE: &str = "rlcd-check";

/// One-time-warn marker keys (cap skip / unreachable) per session.
const WARN_CAP: &str = "cap";
const WARN_UNREACHABLE: &str = "unreachable";

/// Direct /health probe budget for the stage (no auto-spawn on the hot
/// path — a down server degrades fast instead of entering the 60s spawn
/// load poll).
const STAGE_HEALTH_BUDGET: Duration = Duration::from_secs(2);
/// Per-classify transport budget for the stage.
const STAGE_REQUEST_BUDGET: Duration = Duration::from_secs(5);

/// Per-session RLCD security-check counter (latency guard).
static RLCD_CHECK_COUNTER: LazyLock<RwLock<HashMap<Uuid, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Sessions already warned about a stage skip (one-time per session + reason).
static RLCD_SECURITY_WARNED: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// One-time warn (per session + reason).
fn warn_once(session_id: Uuid, reason: &str, msg: String) {
    let mut guard = RLCD_SECURITY_WARNED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let key = format!("{session_id}:{reason}");
    if guard.insert(key) {
        warn!("{msg}");
    }
}

/// Whether the per-session counter has exceeded the cap (0 = unlimited).
fn over_check_cap(session_id: Uuid, cap: u32) -> bool {
    if cap == 0 {
        return false;
    }
    let mut guard = RLCD_CHECK_COUNTER
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let count = guard.entry(session_id).or_insert(0);
    *count += 1;
    *count > cap
}

/// Fail-open warn for an engine/classify failure (one-time per session).
fn warn_unreachable_once(session_id: Uuid, detail: &str) {
    warn_once(
        session_id,
        WARN_UNREACHABLE,
        format!("{detail} — stage skipped (fail-open; regex blocklist stays in force)"),
    );
}

/// The async stage: health probe -> noul classify -> threshold mapping.
/// Every failure mode fails open (Ok(()) + at most one warn); the profile
/// (`thresholds`) is picked by the caller (bash vs file, contract v2).
pub async fn run_rlcd_security_check(
    session_id: Uuid,
    state: &str,
    instructions: &str,
    tool_name: &str,
    thresholds: &SecurityThresholds,
) -> Result<(), BlockedError> {
    let config = load_rlcd_config();
    if !config.enabled || !matches!(config.security.check_mode.as_str(), "all" | "prompt-only") {
        return Ok(()); // disabled config or unknown checkMode: fail-open
    }
    let security = &config.security;
    if is_session_allowed(RLCD_CHECK_ALLOWANCE) {
        return Ok(());
    }
    if over_check_cap(session_id, security.max_checks_per_session) {
        warn_once(
            session_id,
            WARN_CAP,
            format!(
                "rlcd security: per-session check cap ({}) reached for {session_id} — stage skipped",
                security.max_checks_per_session
            ),
        );
        return Ok(());
    }

    // Direct /health probe (model name echoed — never hardcoded). A down or
    // unreachable engine degrades: skip, never block.
    let model = match health_check(&config.url, STAGE_HEALTH_BUDGET).await {
        Ok(model) => model,
        Err(e) => {
            warn_unreachable_once(session_id, &format!("rlcd security: engine unreachable ({e})"));
            return Ok(());
        }
    };

    let question = RlcdQuestion {
        qid: "security".to_string(),
        r#type: "noul".to_string(),
        instructions: Some(instructions.to_string()),
        criteria: None,
    };
    let response = match HttpRlcdClient
        .classify(&config.url, &model, state, std::slice::from_ref(&question), STAGE_REQUEST_BUDGET)
        .await
    {
        Ok(response) => response,
        Err(e) => {
            warn_unreachable_once(session_id, &format!("rlcd security: classify failed ({e})"));
            return Ok(());
        }
    };

    // p = noul P(true) — missing/non-numeric is 0.0 (never fails open the
    // WRONG way: 0.0 can only ever ALLOW below the thresholds).
    let p = response
        .answers
        .get("security")
        .and_then(|a| a.get("noul"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);

    if p >= thresholds.block {
        return Err(BlockedError {
            reason: format!(
                "RLCD security: P(risk)={p} >= blockThreshold {:.2} — the engine judged this operation risky",
                thresholds.block
            ),
            guidance: None,
            rule_id: RLCD_SECURITY_RULE_ID.to_string(),
        });
    }
    if p >= thresholds.prompt {
        let response = pause_for_user(
            session_id,
            PauseRequest {
                kind: PauseKind::Triple,
                tool_name: tool_name.to_string(),
                message: format!(
                    "RLCD security: P(risk)={p} >= promptThreshold {:.2} — the engine suspects this operation",
                    thresholds.prompt
                ),
                details: Some(state.to_string()),
            },
        );
        return match response {
            PauseResponse::AllowSession => {
                allow_for_session(RLCD_CHECK_ALLOWANCE);
                Ok(())
            }
            // AllowOnce and Resumed (no pause handler registered) execute
            // once; Approved (legacy non-triple response) lets it through —
            // the same convention as the regex prompt branch.
            PauseResponse::AllowOnce | PauseResponse::Resumed | PauseResponse::Approved => Ok(()),
            _ => Err(BlockedError {
                reason: "RLCD security: user denied access".to_string(),
                guidance: None,
                rule_id: RLCD_SECURITY_RULE_ID.to_string(),
            }),
        };
    }
    Ok(())
}

/// The SYNC entry point for the regex-blocklist middleware (Bash).
///
/// Bridges into the async stage via `block_in_place` + `Handle::block_on`,
/// guarded by a multi-thread-runtime check — absent one, the stage skips
/// (fail-open; the regex layer is unaffected).
pub fn run_rlcd_security_check_sync(
    session_id: Uuid,
    state: &str,
    instructions: &str,
    tool_name: &str,
    thresholds: &SecurityThresholds,
) -> Result<(), BlockedError> {
    let handle = match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            handle
        }
        _ => return Ok(()), // no multi-thread reactor: skip (fail-open)
    };
    let state = state.to_string();
    let instructions = instructions.to_string();
    let tool_name = tool_name.to_string();
    tokio::task::block_in_place(|| {
        handle.block_on(run_rlcd_security_check(
            session_id, &state, &instructions, &tool_name, thresholds,
        ))
    })
}

/// The async file-operation entry point (Read/Write/Edit/ApplyPatch):
/// the regex pass runs FIRST (a Block rejects, a Prompt pauses + resolves,
/// an explicit Allow passes); the RLCD stage then mirrors the Bash
/// contract: never an explicit-Allow rule; `all` stages no-rule +
/// prompt-allow ops; `prompt-only` stages prompt-allow ops only.
/// `operation` is read/write/edit/patch; `tool_name` the rig tool name.
pub async fn check_file_path_semantic(
    file_path: &str,
    session_id: Uuid,
    tool_name: &str,
    operation: &str,
) -> Result<(), BlockedError> {
    // (1) Regex layer first — deterministic rules are the hard-fact layer.
    crate::blocklist::check_file_path(file_path, session_id)?;

    // (2) Classify the regex outcome (mirrors check_file_path's internal
    // match): explicit Allow rule / prompt-rule allowance / no rule.
    use crate::blocklist::{load_blocklist_config, middleware_project_root, BlocklistMatcher};
    let project_root = middleware_project_root();
    let config = load_blocklist_config(project_root.as_deref());
    let result = if config.rules.is_empty() {
        crate::blocklist::CheckResult::allowed()
    } else {
        BlocklistMatcher::new(config).check_command(file_path)
    };
    let explicit_allow =
        result.matched_action == Some(crate::blocklist::BlocklistAction::Allow);
    let prompt_allowed =
        result.matched_action == Some(crate::blocklist::BlocklistAction::Prompt);

    let rlcd_config = load_rlcd_config();
    let stage = match rlcd_config.security.check_mode.as_str() {
        "all" => !explicit_allow,
        "prompt-only" => prompt_allowed,
        _ => false, // unknown checkMode: stage disabled
    };
    if !stage {
        return Ok(());
    }

    // (3) The stage itself. Context-LEADING state (calibrated 2026-09-24):
    // the agent-harness sentence first, the machine-readable marker after.
    let user = context_user();
    let cwd = crate::facade::get_effective_cwd(session_id)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let ctx = format!(
        "An AI coding agent is about to perform this file operation automatically \
         on the user's development machine (user: {user}, working directory: {cwd}), \
         with the user's permissions and no human watching. "
    );
    let state = format!("{ctx}tool={tool_name}; path={file_path}; operation={operation}");
    let thresholds = SecurityThresholds::file(&rlcd_config.security);
    run_rlcd_security_check(
        session_id,
        &state,
        "Would this file access destroy data irreversibly, damage the system or user \
         accounts, or compromise security (e.g. steal or modify secrets)? Answer true \
         only for genuinely harmful access.",
        tool_name,
        &thresholds,
    )
    .await
}

/// The current user name for the agent-harness context sentence
/// (`USER` env var, else the generic "the user" — never fails).
#[must_use]
pub fn context_user() -> String {
    std::env::var("USER").unwrap_or_else(|_| "the user".to_string())
}
