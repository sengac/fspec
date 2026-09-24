//! RLCD-003 — semantic gate for fspec workflow commands (agent-mode).
//!
//! Feature: spec/features/rlcd-semantic-gate-for-fspec-workflow-commands-agent-mode.feature
//!
//! Gates configured workflow commands at the `FspecToolFacadeWrapper`
//! boundary (agent-mode only — the CLI has no session context and stays
//! ungated). ONE choice question per gated command ({proceed, hold,
//! ask_user}); proceed => execute, hold => reject with "RLCD gate: ...",
//! ask_user => Triple pause (AllowOnce / AllowSession / Deny). Session
//! allowance `rlcd-gate:<command>` suppresses re-prompts. FAIL-OPEN (HITL
//! decision 2026-09-24): unreachable engine or `rlcd.enabled=false` =>
//! execute anyway + warn log; the gate never hard-blocks and never mutates
//! fspec state itself.
//!
//! The pure decision core (answer → outcome) lives in [`gate_map`]; the
//! state builder and allowance helpers are here.

use std::time::Duration;

use serde_json::{json, Value};
use tracing::warn;
use uuid::Uuid;

use crate::blocklist::{allow_for_session, clear_session_allowances, is_session_allowed};
use crate::rlcd::client::{HttpRlcdClient, RlcdClient, RlcdQuestion};
use crate::rlcd::config::load_rlcd_config;
use crate::rlcd::gate_map::{map_gate_answer, GateOutcome};
use crate::rlcd::service::RlcdSupervisor;
use crate::tool_pause::{pause_for_user, PauseKind, PauseRequest, PauseResponse};

/// Bounded wait for the engine to be ready before gating a command.
const GATE_ENSURE_BUDGET: Duration = Duration::from_secs(5);
/// Per-classify-request transport budget.
const GATE_REQUEST_BUDGET: Duration = Duration::from_secs(10);
/// Session-allowance key prefix (blocklist session-allowance API).
const GATE_ALLOWANCE_KEY: &str = "rlcd-gate:";

// ============================================================================
// state builder (best-effort; never fails)
// ============================================================================

/// The raw, always-available state (command + args).
fn raw_state(command: &str, args_json: &str) -> String {
    format!("command={command}; args={args_json}")
}

/// Build the RLCD state string for a gated command.
///
/// `update-work-unit-status` gets the work unit's title/epic from
/// `<project_root>/spec/work-units.json` (best-effort — a missing/unreadable
/// file or unknown unit degrades to the raw `command=...; args=...` state);
/// every other command uses the raw state.
#[must_use]
pub fn build_gate_state(command: &str, args_json: &str, project_root: &str) -> String {
    if command == "update-work-unit-status" {
        let Ok(parsed) = serde_json::from_str::<Value>(args_json) else {
            return raw_state(command, args_json);
        };
        let work_unit_id = parsed
            .get("workUnitId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let status = parsed
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some((title, epic)) = work_unit_meta(project_root, &work_unit_id) {
            return format!(
                "command=update-work-unit-status; workUnit={work_unit_id}; \
                 transition=to {status}; title={title}; epic={epic}; args={args_json}"
            );
        }
        return format!(
            "command=update-work-unit-status; workUnit={work_unit_id}; \
             transition=to {status}; args={args_json}"
        );
    }
    raw_state(command, args_json)
}

/// Best-effort `(title, epic)` lookup in `spec/work-units.json`.
fn work_unit_meta(project_root: &str, work_unit_id: &str) -> Option<(String, String)> {
    let path = std::path::Path::new(project_root).join("spec/work-units.json");
    let content = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&content).ok()?;
    let unit = value.get("workUnits")?.get(work_unit_id)?;
    let title = unit
        .get("title")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())?
        .to_string();
    let epic = unit
        .get("epic")
        .and_then(Value::as_str)
        .filter(|e| !e.is_empty())
        .unwrap_or_default()
        .to_string();
    Some((title, epic))
}

// ============================================================================
// session-allowance helpers (blocklist API, keyed 'rlcd-gate:<command>')
// ============================================================================

/// Whether the session already allows the gate for `command` (AllowSession).
#[must_use]
pub fn is_session_gate_allowed(command: &str) -> bool {
    is_session_allowed(&format!("{GATE_ALLOWANCE_KEY}{command}"))
}

/// Record the gate allowance for `command` (user chose AllowSession).
pub fn allow_session_gate(command: &str) {
    allow_for_session(&format!("{GATE_ALLOWANCE_KEY}{command}"));
}

/// Clear every gate allowance (TUI restart path — the blocklist
/// `clear_session_allowances` already covers this; kept for the
/// contract/tests that clear only the gate keys).
pub fn clear_session_gate_allowances() {
    clear_session_allowances();
}

// ============================================================================
// gate entry
// ============================================================================

/// The gate's verdict for one gated command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateVerdict {
    /// Execute via the normal fspec handler path.
    Execute,
    /// Reject BEFORE execution (the agent sees the reason and can react).
    Rejected { reason: String },
}

/// The gate entry point (called from `FspecToolFacadeWrapper::call` — the
/// agent-mode boundary). Never panics; every failure mode fails open.
pub async fn run_rlcd_gate(
    session_id: Uuid,
    command: &str,
    args_json: &str,
    project_root: &str,
) -> GateVerdict {
    let config = load_rlcd_config();
    let gate_config = config.gate.clone();

    // Ungated command / disabled engine: execute without any RLCD work.
    if !config.enabled || !gate_config.commands.iter().any(|c| c == command) {
        return GateVerdict::Execute;
    }
    // Session allowance (AllowSession): execute without asking.
    if is_session_gate_allowed(command) {
        return GateVerdict::Execute;
    }

    let supervisor = RlcdSupervisor::new(config);
    let configured_url = supervisor.config().url.clone();
    let model = match supervisor.ensure_ready(GATE_ENSURE_BUDGET).await {
        Ok(model) => model,
        Err(e) => {
            warn!(
                command,
                error = %e,
                "rlcd gate: engine unreachable — failing open (executing the command)"
            );
            return GateVerdict::Execute;
        }
    };

    let state = build_gate_state(command, args_json, project_root);
    let question = RlcdQuestion {
        qid: "gate".to_string(),
        r#type: "choice".to_string(),
        instructions: Some("Given this ACDD workflow command, should it proceed?".to_string()),
        criteria: Some(json!({
            "proceed": "Proceed with the command",
            "hold": "Hold - bad timing or risky",
            "ask_user": "Ask the user"
        })),
    };
    let client = HttpRlcdClient;
    let base_url = supervisor.effective_base_url().unwrap_or(configured_url);
    let response = match client
        .classify(
            &base_url,
            &model,
            &state,
            std::slice::from_ref(&question),
            GATE_REQUEST_BUDGET,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(
                command,
                error = %e,
                "rlcd gate: classify failed — failing open (executing the command)"
            );
            return GateVerdict::Execute;
        }
    };

    let probs = response
        .answers
        .get("gate")
        .and_then(|a| a.get("probabilities"))
        .cloned()
        .unwrap_or(Value::Null);
    match map_gate_answer(&probs, &gate_config) {
        GateOutcome::Proceed => GateVerdict::Execute,
        GateOutcome::Hold { reason } => GateVerdict::Rejected {
            reason: format!("RLCD gate: {reason}"),
        },
        GateOutcome::AskUser { reason } => gate_pause(session_id, command, args_json, reason),
    }
}

/// Outcome (c): Triple pause; AllowOnce/AllowSession execute, Deny rejects.
fn gate_pause(
    session_id: Uuid,
    command: &str,
    args_json: &str,
    reason: String,
) -> GateVerdict {
    let response = pause_for_user(
        session_id,
        PauseRequest {
            kind: PauseKind::Triple,
            tool_name: "fspec".to_string(),
            message: format!("RLCD gate: {reason}"),
            details: Some(format!("command={command}; args={args_json}")),
        },
    );
    match response {
        PauseResponse::AllowSession => {
            allow_session_gate(command);
            GateVerdict::Execute
        }
        // AllowOnce and Resumed (no pause handler registered) execute once.
        PauseResponse::AllowOnce | PauseResponse::Resumed => GateVerdict::Execute,
        _ => GateVerdict::Rejected {
            reason: "RLCD gate: user denied".to_string(),
        },
    }
}
