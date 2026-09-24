//! RLCD-003 — RLCD semantic gate for fspec workflow commands (agent-mode).
//!
//! Feature: spec/features/rlcd-semantic-gate-for-fspec-workflow-commands-agent-mode.feature
//!
//! Covers the workflow-gate contract (see the RLCD-003 architecture notes +
//! spec/attachments/RLCD-003/ast-research-rlcd-workflow-gate.md): the
//! FspecToolFacadeWrapper boundary gates configured commands with ONE choice
//! question ({proceed, hold, ask_user}) against the RLCD engine; proceed =>
//! execute via the normal handler, hold => reject with an "RLCD gate:" error
//! (handler NOT called), ask_user => Triple pause (AllowOnce / AllowSession /
//! Deny); session allowance 'rlcd-gate:<command>' suppresses re-prompts;
//! fail-open (unreachable engine or rlcd.enabled=false) executes anyway; the
//! gate list + thresholds are config-driven; the gate state is built
//! best-effort from the command/args + spec/work-units.json (title/epic when
//! available, raw state otherwise, never failing).
//!
//! ACDD red phase: at test-writing time `codelet_tools::rlcd::gate`
//! (run_rlcd_gate, map_gate_answer, build_gate_state, RlcdGateConfig, ...)
//! does not exist yet; this suite fails to compile/link until the
//! implementing phase provides the contracted API.
//!
//! Test seams (per the AST research attachment):
//! * handler stub via `set_fspec_handler_for_session` (records FspecRequest),
//! * Triple pause via `set_pause_handler` (captures PauseRequest),
//! * session allowances via the blocklist API (`allow_for_session` etc.),
//! * user config via `FSPEC_USER_DIR` temp dir (#[serial] — env mutation),
//! * mock RLCD server via the included rlcd_mock.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "rlcd_mock.rs"]
mod rlcd_mock;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use codelet_tools::facade::{FspecToolFacade, InternalFspecParams, ToolDefinition};
use codelet_tools::facade::wrapper::{FacadeArgs, FspecToolFacadeWrapper};
use codelet_tools::rlcd::{
    build_gate_state, clear_session_gate_allowances, is_session_gate_allowed, load_rlcd_config,
    map_gate_answer, GateOutcome, RlcdGateConfig,
};
use codelet_tools::blocklist::{
    allow_for_session, clear_session_allowances, is_session_allowed,
};
use codelet_tools::fspec_handler::{
    set_fspec_handler_for_session, FspecHandler, FspecRequest, FspecResult,
};
use codelet_tools::{
    set_pause_handler, PauseKind, PauseRequest, PauseResponse,
};
use rig::tool::Tool;
use rlcd_mock::MockRlcd;
use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;
use uuid::Uuid;

// ============================================================================
// test facade + handler/pause stubs (AST research §2, §5)
// ============================================================================

/// Minimal `FspecToolFacade` whose `map_params` returns fixed internal params
/// (the wrapper's pre-gate path only needs command/args/project_root).
struct TestFspecFacade {
    command: String,
    args_json: String,
    project_root: String,
}

impl FspecToolFacade for TestFspecFacade {
    fn provider(&self) -> &'static str {
        "test"
    }

    fn tool_name(&self) -> &'static str {
        "TestFspec"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "TestFspec".to_string(),
            description: "test facade".to_string(),
            parameters: json!({"type": "object"}),
        }
    }

    fn map_params(&self, _input: serde_json::Value) -> Result<InternalFspecParams, codelet_tools::ToolError> {
        Ok(InternalFspecParams {
            command: self.command.clone(),
            args: self.args_json.clone(),
            project_root: self.project_root.clone(),
        })
    }
}

/// A handler stub that records every FspecRequest it receives and returns a
/// fixed success result — proves whether (and with what request) the normal
/// fspec execution path ran.
fn handler_stub(requests: &Arc<std::sync::Mutex<Vec<FspecRequest>>>) -> FspecHandler {
    let shared = Arc::clone(requests);
    Arc::new(move |req: FspecRequest| {
        shared.lock().expect("handler lock poisoned").push(req);
        FspecResult {
            success: true,
            data: "HANDLER-OK".to_string(),
            error: None,
            system_reminder: None,
        }
    })
}

/// A pause handler that records the PauseRequest and returns the canned
/// response (the call counter is separate, so scenarios can pause twice).
fn pause_stub(
    requests: &Arc<std::sync::Mutex<Vec<PauseRequest>>>,
    calls: &Arc<AtomicUsize>,
    response: PauseResponse,
) -> codelet_tools::PauseHandler {
    let shared = Arc::clone(requests);
    let counter = Arc::clone(calls);
    Arc::new(move |req: PauseRequest| {
        shared.lock().expect("pause lock poisoned").push(req);
        counter.fetch_add(1, Ordering::SeqCst);
        response
    })
}

// ============================================================================
// config + env helpers (user config via FSPEC_USER_DIR temp dir)
// ============================================================================

/// A temporary fspec user dir with `fspec-config.json` whose `rlcd` section
/// points at `url` (plus an optional nested `gate` object, e.g.
/// `{"commands": [...], "holdThreshold": 0.95}`). Restores the environment
/// on drop (callers run under `#[serial]`).
fn config_dir_with_rlcd_url(url: &str, extra_gate: Option<serde_json::Value>) -> (TempDir, String) {
    let tmp = TempDir::new().expect("temp user dir");
    let mut section = json!({ "url": url });
    if let Some(gate) = extra_gate {
        if let Some(s) = section.as_object_mut() {
            s.insert("gate".to_string(), gate);
        }
    }
    let body = json!({ "rlcd": section }).to_string();
    std::fs::write(tmp.path().join("fspec-config.json"), body).expect("write config");
    std::env::set_var("FSPEC_USER_DIR", tmp.path());
    (tmp, url.to_string())
}

/// An empty temp user dir (no config file) — resolves to RLCD defaults.
fn empty_config_dir() -> TempDir {
    let tmp = TempDir::new().expect("temp user dir");
    std::env::set_var("FSPEC_USER_DIR", tmp.path());
    tmp
}

/// The args JSON for `update-work-unit-status` moving `unit` to `status`.
fn status_args(unit: &str, status: &str) -> String {
    json!({ "workUnitId": unit, "status": status }).to_string()
}

/// The mock /health model used across scenarios (proves model echo too).
const GATE_MODEL: &str = "gate-model-x";

/// The captured classifier request (exactly one expected per gate ask).
fn captured_gate_request(mock: &MockRlcd) -> serde_json::Value {
    let bodies = mock.classifier_bodies();
    assert_eq!(bodies.len(), 1, "exactly one classifier request expected");
    serde_json::from_str(&bodies[0]).expect("classifier request body is JSON")
}

/// Assert a gate pause happened: the LAST recorded pause is a Triple pause,
/// tool_name "fspec", details carrying the gated command. (Call-count checks
/// go through the pause_calls counter — a scenario may pause more than once.)
fn assert_gate_pause(requests: &std::sync::Mutex<Vec<PauseRequest>>, command: &str) {
    let guard = requests.lock().expect("pause lock poisoned");
    assert!(!guard.is_empty(), "at least one gate pause expected");
    let req = guard.last().expect("pause list non-empty");
    assert_eq!(req.kind, PauseKind::Triple, "gate pause must be Triple");
    assert_eq!(req.tool_name, "fspec", "gate pause must name the fspec tool");
    assert!(
        req.details
            .as_deref()
            .is_some_and(|d| d.contains(command)),
        "gate pause details must carry the gated command {command:?}: {:?}",
        req.details
    );
}

/// Build the wrapper around the test facade for `command` (project_root "."),
/// with the recording handler registered for a fresh session.
fn wrapper_with_handler(
    command: &str,
    args_json: &str,
    requests: &Arc<std::sync::Mutex<Vec<FspecRequest>>>,
) -> (FspecToolFacadeWrapper, Uuid) {
    let session = Uuid::new_v4();
    set_fspec_handler_for_session(session, Some(handler_stub(requests)));
    let facade = TestFspecFacade {
        command: command.to_string(),
        args_json: args_json.to_string(),
        project_root: ".".to_string(),
    };
    (
        FspecToolFacadeWrapper::new(Arc::new(facade), session),
        session,
    )
}

// ============================================================================
// OUTCOME (a): PROCEED
// ============================================================================

/// Scenario: A confident proceed answer executes the command normally
#[serial]
#[tokio::test]
async fn scenario_proceed_executes_normally() {
    // @step Given a gated command "update-work-unit-status" for work unit "AUTH-001" from "implementing" to "validating"
    let args = status_args("AUTH-001", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));

    // @step And a reachable RLCD engine answering the gate question with P(proceed)=0.90, P(hold)=0.05, P(ask_user)=0.05
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"proceed","confidence":0.9,"probabilities":{"proceed":0.9,"hold":0.05,"ask_user":0.05}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);
    let (wrapper, session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    // @step When the agent issues the command
    let out = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("proceed must execute, not error");

    // @step Then the command executes through the normal fspec handler path
    assert!(out.contains("HANDLER-OK"), "handler result must pass through: {out}");
    {
        let guard = requests.lock().expect("handler lock poisoned");
        assert_eq!(guard.len(), 1, "handler must be called exactly once");
        assert_eq!(guard[0].command, "update-work-unit-status");
        assert_eq!(guard[0].args_json, args);
    }

    // @step And the agent receives the unchanged success result
    assert!(out.starts_with("HANDLER-OK"), "success result must be unchanged: {out}");

    // @step And no pause prompt was shown to the user
    assert!(!codelet_tools::has_pause_handler(session), "sanity: no handler registered");
    assert_eq!(mock.classifier_hits(), 1, "exactly one gate question");
    // The model must be echoed from /health (never hardcoded)
    let req = captured_gate_request(&mock);
    assert_eq!(
        req.get("model").and_then(serde_json::Value::as_str),
        Some(GATE_MODEL),
        "gate request must echo the /health-reported model"
    );
    mock.stop().await;
}

// ============================================================================
// OUTCOME (b): HOLD
// ============================================================================

/// Scenario: A hold answer rejects the command before execution
#[serial]
#[tokio::test]
async fn scenario_hold_rejects_before_execution() {
    // @step Given a gated command "update-work-unit-status" for work unit "AUTH-002" from "implementing" to "done"
    let args = status_args("AUTH-002", "done");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));

    // @step And a reachable RLCD engine answering the gate question with P(hold)=0.80, P(proceed)=0.05, P(ask_user)=0.15
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"hold","confidence":0.8,"probabilities":{"proceed":0.05,"hold":0.8,"ask_user":0.15}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);
    let (wrapper, _session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    // @step When the agent issues the command
    let result = wrapper.call(FacadeArgs(json!({}))).await;

    // @step Then the command is rejected with an error starting "RLCD gate:"
    let err = result
        .expect_err("a hold answer must reject, not execute")
        .to_string();
    assert!(
        err.starts_with("RLCD gate:"),
        "reject error must start with 'RLCD gate:': {err}"
    );

    // @step And the fspec handler is NOT called (the work unit's status is unchanged)
    {
        let guard = requests.lock().expect("handler lock poisoned");
        assert!(guard.is_empty(), "hold must reject BEFORE the handler runs");
    }

    // @step And no pause prompt was shown to the user
    assert_eq!(mock.classifier_hits(), 1, "exactly one gate question");
    mock.stop().await;
}

// ============================================================================
// OUTCOME (c): ASK_USER (Triple pause)
// ============================================================================

/// Scenario: An ask_user answer with a Deny response rejects the command
#[serial]
#[tokio::test]
async fn scenario_ask_user_deny_rejects() {
    // @step Given a gated command "update-work-unit-status" for work unit "AUTH-003" from "testing" to "implementing"
    let args = status_args("AUTH-003", "implementing");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));

    // @step And a reachable RLCD engine answering the gate question with P(ask_user)=0.55, P(hold)=0.20, P(proceed)=0.25
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"ask_user","confidence":0.55,"probabilities":{"proceed":0.25,"hold":0.2,"ask_user":0.55}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);
    let (wrapper, session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    let pause_requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pause_calls = Arc::new(AtomicUsize::new(0));
    set_pause_handler(
        session,
        Some(pause_stub(&pause_requests, &pause_calls, PauseResponse::Denied)),
    );

    // @step When the agent issues the command
    let result = wrapper.call(FacadeArgs(json!({}))).await;

    // @step Then a Triple pause prompt appears showing the command and the engine's rationale
    assert_gate_pause(&pause_requests, "update-work-unit-status");

    // @step And the user chooses Deny
    assert_eq!(pause_calls.load(Ordering::SeqCst), 1);

    // @step Then the command is rejected with an error containing "RLCD gate: user denied"
    let err = result
        .expect_err("Deny must reject, not execute")
        .to_string();
    assert!(
        err.starts_with("RLCD gate:") && err.contains("user denied"),
        "deny rejection must read 'RLCD gate: user denied': {err}"
    );

    // @step And the fspec handler is NOT called
    {
        let guard = requests.lock().expect("handler lock poisoned");
        assert!(guard.is_empty(), "Deny must not reach the handler");
    }
    mock.stop().await;
}

/// Scenario: An ask_user answer with AllowOnce executes once and prompts again
#[serial]
#[tokio::test]
async fn scenario_ask_user_allow_once_executes_and_prompts_again() {
    let args = status_args("AUTH-004", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));

    // @step Given a reachable RLCD engine answering the gate question with P(ask_user)=0.50, P(hold)=0.10, P(proceed)=0.40
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"ask_user","confidence":0.5,"probabilities":{"proceed":0.4,"hold":0.1,"ask_user":0.5}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);

    // @step When the agent issues the gated command "update-work-unit-status" and the user chooses Allow Once
    let (wrapper, session) = wrapper_with_handler("update-work-unit-status", &args, &requests);
    let pause_requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pause_calls = Arc::new(AtomicUsize::new(0));
    set_pause_handler(
        session,
        Some(pause_stub(&pause_requests, &pause_calls, PauseResponse::AllowOnce)),
    );
    let out = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("AllowOnce must execute");

    // @step Then the command executes through the normal fspec handler path
    assert!(out.contains("HANDLER-OK"));
    assert_eq!(
        requests.lock().expect("handler lock poisoned").len(),
        1
    );
    assert_eq!(pause_calls.load(Ordering::SeqCst), 1);

    // @step When the agent issues the same gated command again
    set_pause_handler(
        session,
        Some(pause_stub(&pause_requests, &pause_calls, PauseResponse::Denied)),
    );
    let second = wrapper.call(FacadeArgs(json!({}))).await;

    // @step Then a Triple pause prompt appears again
    assert_eq!(
        pause_calls.load(Ordering::SeqCst),
        2,
        "AllowOnce is one-shot: the next gated command must prompt again"
    );
    assert_gate_pause(&pause_requests, "update-work-unit-status");
    assert!(
        second.is_err(),
        "second command (Denied) must reject: {second:?}"
    );
    assert_eq!(
        requests.lock().expect("handler lock poisoned").len(),
        1,
        "only the first (AllowOnce) execution may reach the handler"
    );
    mock.stop().await;
}

/// Scenario: An ask_user answer with AllowSession suppresses later prompts
#[serial]
#[tokio::test]
async fn scenario_ask_user_allow_session_suppresses_later_prompts() {
    let args = status_args("AUTH-005", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    clear_session_allowances();

    // @step Given a reachable RLCD engine answering the gate question with P(ask_user)=0.50, P(hold)=0.10, P(proceed)=0.40
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"ask_user","confidence":0.5,"probabilities":{"proceed":0.4,"hold":0.1,"ask_user":0.5}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);
    let (wrapper, session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    let pause_requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pause_calls = Arc::new(AtomicUsize::new(0));
    set_pause_handler(
        session,
        Some(pause_stub(&pause_requests, &pause_calls, PauseResponse::AllowSession)),
    );

    // @step When the agent issues the gated command "update-work-unit-status" and the user chooses Allow Session
    let out = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("AllowSession must execute");
    assert!(out.contains("HANDLER-OK"));
    assert!(
        is_session_allowed("rlcd-gate:update-work-unit-status"),
        "AllowSession must record the gate allowance"
    );

    // @step Then the command executes through the normal fspec handler path
    assert_eq!(requests.lock().expect("handler lock poisoned").len(), 1);

    // @step When the agent issues the same gated command again
    let second = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("allowance must let the second command execute");

    // @step Then the command executes without another prompt and without any RLCD call
    assert!(second.contains("HANDLER-OK"));
    assert_eq!(
        pause_calls.load(Ordering::SeqCst),
        1,
        "the allowance must suppress the second prompt"
    );
    assert_eq!(
        mock.classifier_hits(),
        1,
        "the allowance must skip the second RLCD call"
    );
    clear_session_allowances();
    mock.stop().await;
}

// ============================================================================
// FAIL-OPEN (rule 7)
// ============================================================================

/// Scenario: An unreachable engine fails open and executes the command
#[serial]
#[tokio::test]
async fn scenario_unreachable_fails_open_and_executes() {
    // @step Given no reachable RLCD engine anywhere
    // (an unroutable private IP: connect-only, never spawned; the bounded
    //  ensure budget makes the fail-open degrade quickly)
    let tmp = empty_config_dir();
    let _ = tmp;
    let (_tmp, _url) = config_dir_with_rlcd_url("http://10.255.255.1:9999", None);
    let args = status_args("AUTH-006", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (wrapper, _session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    let started = std::time::Instant::now();

    // @step When the agent issues the gated command "update-work-unit-status"
    let out = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("fail-open must execute, not block or error");

    // @step Then the command executes through the normal fspec handler path
    assert!(out.contains("HANDLER-OK"));
    assert_eq!(requests.lock().expect("handler lock poisoned").len(), 1);

    // @step And the agent receives the unchanged success result
    assert!(out.starts_with("HANDLER-OK"));
    assert!(
        started.elapsed() < Duration::from_secs(9),
        "fail-open must stay within a bounded wait (no 60s model-load poll)"
    );
}

// ============================================================================
// GATE LIST (rule 2)
// ============================================================================

/// Scenario: A command not in the gate list passes through without RLCD involvement
#[serial]
#[tokio::test]
async fn scenario_ungated_command_passes_through() {
    // @step Given a reachable RLCD engine
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (wrapper, _session) = wrapper_with_handler(
        "list-work-units",
        r#"{"status":"backlog"}"#,
        &requests,
    );

    // @step When the agent issues the command "list-work-units"
    let out = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("ungated command must execute");

    // @step Then the command executes through the normal fspec handler path
    assert!(out.contains("HANDLER-OK"));
    assert_eq!(requests.lock().expect("handler lock poisoned").len(), 1);

    // @step And no RLCD gate question was asked
    assert_eq!(
        mock.classifier_hits(),
        0,
        "ungated commands must not touch the RLCD engine"
    );
    mock.stop().await;
}

/// Scenario: A command added to the gate list via config is also gated
#[serial]
#[tokio::test]
async fn scenario_config_added_command_is_gated() {
    // @step Given the user config lists rlcd.gate.commands as ["update-work-unit-status", "delete-work-unit"]
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"hold","confidence":0.9,"probabilities":{"proceed":0.05,"hold":0.9,"ask_user":0.05}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let gate = json!({ "commands": ["update-work-unit-status", "delete-work-unit"] });
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), Some(gate));

    // The config must round-trip through load_rlcd_config()
    let cfg = load_rlcd_config();
    assert!(
        cfg.gate.commands.iter().any(|c| c == "delete-work-unit"),
        "the configured gate list must be honored by load_rlcd_config()"
    );

    // @step And a reachable RLCD engine answering the gate question with P(hold)=0.90, P(proceed)=0.05, P(ask_user)=0.05
    // (answer already set above)
    let args = r#"{"workUnitId":"AUTH-009"}"#;
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (wrapper, _session) = wrapper_with_handler("delete-work-unit", args, &requests);

    // @step When the agent issues the gated command "delete-work-unit"
    let result = wrapper.call(FacadeArgs(json!({}))).await;

    // @step Then the command is rejected with an error starting "RLCD gate:"
    let err = result
        .expect_err("a configured gate command must be reviewed")
        .to_string();
    assert!(err.starts_with("RLCD gate:"), "got: {err}");

    // @step And the fspec handler is NOT called
    assert!(requests.lock().expect("handler lock poisoned").is_empty());
    mock.stop().await;
}

// ============================================================================
// STATE CONSTRUCTION (rule 3)
// ============================================================================

/// Scenario: The gate state includes the work unit's title and epic when available
#[serial]
#[tokio::test]
async fn scenario_state_includes_title_and_epic() {
    // @step Given a project whose spec/work-units.json defines work unit "AUTH-001" with title "User Login" and epic "authentication"
    let project = TempDir::new().expect("project dir");
    std::fs::create_dir_all(project.path().join("spec")).expect("create spec dir");
    std::fs::write(
        project.path().join("spec/work-units.json"),
        json!({
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001",
                    "title": "User Login",
                    "epic": "authentication",
                    "status": "implementing"
                }
            }
        })
        .to_string(),
    )
    .expect("write work-units.json");

    // @step And a reachable RLCD engine
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"proceed","confidence":0.9,"probabilities":{"proceed":0.9,"hold":0.05,"ask_user":0.05}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);

    // @step When the agent issues the gated command "update-work-unit-status" for work unit "AUTH-001"
    let args = status_args("AUTH-001", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (_wrapper, _session) = wrapper_with_handler(
        "update-work-unit-status",
        &args,
        &requests,
    );
    // point the test facade at the temp project (fresh wrapper with the same handler)
    let session = Uuid::new_v4();
    codelet_tools::set_fspec_handler_for_session(session, Some(handler_stub(&requests)));
    let facade = TestFspecFacade {
        command: "update-work-unit-status".to_string(),
        args_json: args.clone(),
        project_root: project.path().to_string_lossy().to_string(),
    };
    let wrapper2 = FspecToolFacadeWrapper::new(Arc::new(facade), session);
    let out = wrapper2
        .call(FacadeArgs(json!({})))
        .await
        .expect("state-rich gate must still execute on proceed");
    assert!(out.contains("HANDLER-OK"));

    // @step Then the RLCD request state mentions "AUTH-001", the status transition, "User Login" and "authentication"
    let req = captured_gate_request(&mock);
    let state = req
        .get("state")
        .and_then(serde_json::Value::as_str)
        .expect("state must be a plain string");
    for needle in ["AUTH-001", "validating", "User Login", "authentication"] {
        assert!(
            state.contains(needle),
            "gate state must mention {needle:?}: {state}"
        );
    }
    mock.stop().await;
}

/// Scenario: A missing work-units.json degrades to a raw command state without failing
#[serial]
#[tokio::test]
async fn scenario_missing_work_units_degrades_to_raw_state() {
    // @step Given a project without a spec/work-units.json file
    let project = TempDir::new().expect("project dir");

    // @step And a reachable RLCD engine
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"proceed","confidence":0.9,"probabilities":{"proceed":0.9,"hold":0.05,"ask_user":0.05}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);

    // @step When the agent issues the gated command "update-work-unit-status" for work unit "GHOST-999"
    let args = status_args("GHOST-999", "backlog");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let session = Uuid::new_v4();
    codelet_tools::set_fspec_handler_for_session(session, Some(handler_stub(&requests)));
    let facade = TestFspecFacade {
        command: "update-work-unit-status".to_string(),
        args_json: args.clone(),
        project_root: project.path().to_string_lossy().to_string(),
    };
    let wrapper = FspecToolFacadeWrapper::new(Arc::new(facade), session);
    let out = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("missing context must never fail the gate");
    assert!(out.contains("HANDLER-OK"));

    // @step Then the gate proceeds with a state naming the command and its arguments (no title, no epic)
    let req = captured_gate_request(&mock);
    let state = req
        .get("state")
        .and_then(serde_json::Value::as_str)
        .expect("state must be a plain string");
    assert!(
        state.contains("update-work-unit-status"),
        "raw state must name the command: {state}"
    );
    assert!(
        state.contains("GHOST-999"),
        "raw state must carry the args: {state}"
    );

    // @step And the command still goes through the RLCD review
    assert_eq!(mock.classifier_hits(), 1);
    mock.stop().await;
}

// ============================================================================
// DISABLED CONFIG (rule 7)
// ============================================================================

/// Scenario: A disabled RLCD config bypasses the gate entirely
#[serial]
#[tokio::test]
async fn scenario_disabled_config_bypasses_gate() {
    // @step Given the user config has rlcd.enabled = false
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    let tmp = TempDir::new().expect("temp user dir");
    std::fs::write(
        tmp.path().join("fspec-config.json"),
        json!({ "rlcd": { "enabled": false, "url": mock.base_url() } }).to_string(),
    )
    .expect("write config");
    std::env::set_var("FSPEC_USER_DIR", tmp.path());

    // @step And a reachable RLCD engine
    // (mock ready above — but it must never be consulted)
    let args = status_args("AUTH-011", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (wrapper, _session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    // @step When the agent issues the gated command "update-work-unit-status"
    let out = wrapper
        .call(FacadeArgs(json!({})))
        .await
        .expect("disabled RLCD must never block");

    // @step Then the command executes through the normal fspec handler path
    assert!(out.contains("HANDLER-OK"));
    assert_eq!(requests.lock().expect("handler lock poisoned").len(), 1);

    // @step And no RLCD gate question was asked
    assert_eq!(
        mock.classifier_hits(),
        0,
        "disabled RLCD must not touch the engine"
    );
    mock.stop().await;
}

// ============================================================================
// DECISION MAPPING (rule 5; thresholds)
// ============================================================================

/// Scenario: A low-confidence proceed answer asks the user instead of proceeding
#[serial]
#[tokio::test]
async fn scenario_thin_margin_proceed_asks_user() {
    // @step Given a reachable RLCD engine answering the gate question with P(proceed)=0.50, P(ask_user)=0.38, P(hold)=0.12
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"proceed","confidence":0.5,"probabilities":{"proceed":0.5,"hold":0.12,"ask_user":0.38}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);
    let args = status_args("AUTH-012", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (wrapper, session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    let pause_requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pause_calls = Arc::new(AtomicUsize::new(0));
    set_pause_handler(
        session,
        Some(pause_stub(&pause_requests, &pause_calls, PauseResponse::Denied)),
    );

    // @step When the agent issues the gated command "update-work-unit-status"
    let result = wrapper.call(FacadeArgs(json!({}))).await;

    // @step Then a Triple pause prompt appears (the proceed margin 0.12 is below proceedMargin 0.15)
    assert_gate_pause(&pause_requests, "update-work-unit-status");
    assert_eq!(pause_calls.load(Ordering::SeqCst), 1);

    // @step And the fspec handler is NOT called until the user answers
    assert!(
        requests.lock().expect("handler lock poisoned").is_empty(),
        "thin-margin proceed must never auto-execute"
    );
    assert!(result.is_err(), "Deny must reject the pending command");
    mock.stop().await;
}

/// Scenario: A hold probability at or above holdThreshold rejects the command
#[serial]
#[tokio::test]
async fn scenario_hold_threshold_with_other_argmax_rejects() {
    // @step Given a reachable RLCD engine answering the gate question with P(ask_user)=0.80, P(hold)=0.70, P(proceed)=0.50
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"ask_user","confidence":0.8,"probabilities":{"proceed":0.5,"hold":0.7,"ask_user":0.8}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), None);
    let args = status_args("AUTH-013", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (wrapper, _session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    // @step When the agent issues the gated command "update-work-unit-status"
    let result = wrapper.call(FacadeArgs(json!({}))).await;

    // @step Then the command is rejected with an error starting "RLCD gate:" (P(hold)=0.70 meets the default holdThreshold of 0.65)
    let err = result
        .expect_err("P(hold) >= holdThreshold must reject regardless of argmax")
        .to_string();
    assert!(err.starts_with("RLCD gate:"), "got: {err}");

    // @step And the fspec handler is NOT called
    assert!(requests.lock().expect("handler lock poisoned").is_empty());
    mock.stop().await;
}

/// Scenario: Configured thresholds change the gate's decision
#[serial]
#[tokio::test]
async fn scenario_configured_threshold_changes_decision() {
    // @step Given the user config has rlcd.gate.holdThreshold = 0.95
    let mock = MockRlcd::ready(GATE_MODEL).start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"ask_user","confidence":0.8,"probabilities":{"proceed":0.5,"hold":0.7,"ask_user":0.8}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let gate = json!({ "holdThreshold": 0.95 });
    let (_tmp, _url) = config_dir_with_rlcd_url(&mock.base_url(), Some(gate));
    let args = status_args("AUTH-014", "validating");
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (wrapper, session) = wrapper_with_handler("update-work-unit-status", &args, &requests);

    let pause_requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pause_calls = Arc::new(AtomicUsize::new(0));
    set_pause_handler(
        session,
        Some(pause_stub(&pause_requests, &pause_calls, PauseResponse::Denied)),
    );

    // @step And a reachable RLCD engine answering the gate question with P(ask_user)=0.80, P(hold)=0.70, P(proceed)=0.50
    // (answer already set above)

    // @step When the agent issues the gated command "update-work-unit-status"
    let result = wrapper.call(FacadeArgs(json!({}))).await;

    // @step Then a Triple pause prompt appears (P(hold)=0.70 is below the configured holdThreshold of 0.95 and argmax is ask_user)
    assert_gate_pause(&pause_requests, "update-work-unit-status");
    assert_eq!(pause_calls.load(Ordering::SeqCst), 1);

    // @step And the fspec handler is NOT called until the user answers
    assert!(
        requests.lock().expect("handler lock poisoned").is_empty(),
        "below the raised threshold the same answer must ask, not reject"
    );
    assert!(result.is_err(), "Deny must reject the pending command");
    mock.stop().await;
}

// ============================================================================
// PURE DECISION CORE + STATE BUILDER (contract seams, unit-level)
// ============================================================================

/// Unit: map_gate_answer priority + margin semantics (contract note §2).
#[test]
fn unit_map_gate_answer_priorities() {
    let cfg = RlcdGateConfig::default();
    // hold threshold trips even with a different argmax
    assert!(
        matches!(
            map_gate_answer(&json!({"proceed": 0.5, "hold": 0.7, "ask_user": 0.8}), &cfg),
            GateOutcome::Hold { .. }
        ),
        "P(hold) >= holdThreshold must yield Hold"
    );
    // argmax proceed with a comfortable margin
    assert!(
        matches!(
            map_gate_answer(&json!({"proceed": 0.9, "hold": 0.05, "ask_user": 0.05}), &cfg),
            GateOutcome::Proceed
        ),
        "comfortable proceed margin must yield Proceed"
    );
    // thin margin (0.12 <= 0.15) is conservative: AskUser
    assert!(
        matches!(
            map_gate_answer(&json!({"proceed": 0.5, "hold": 0.12, "ask_user": 0.38}), &cfg),
            GateOutcome::AskUser { .. }
        ),
        "thin proceed margin must degrade to AskUser"
    );
    // argmax ask_user
    assert!(
        matches!(
            map_gate_answer(&json!({"proceed": 0.25, "hold": 0.2, "ask_user": 0.55}), &cfg),
            GateOutcome::AskUser { .. }
        ),
        "argmax ask_user must yield AskUser"
    );
    // missing probabilities are treated as 0.0 => never auto-proceed
    assert!(
        !matches!(
            map_gate_answer(&json!({}), &cfg),
            GateOutcome::Proceed
        ),
        "an empty answer must never auto-proceed"
    );
}

/// Unit: build_gate_state degrades to a raw command/args string (contract §3).
#[test]
fn unit_build_gate_state_missing_context() {
    let project = TempDir::new().expect("project dir");
    let state = build_gate_state(
        "update-work-unit-status",
        r#"{"workUnitId":"X-1","status":"done"}"#,
        &project.path().to_string_lossy(),
    );
    assert!(
        state.contains("update-work-unit-status"),
        "raw state must name the command: {state}"
    );
    assert!(state.contains("X-1"), "raw state must carry the args: {state}");
    // never panics/fails on a missing spec/work-units.json
    let _ = Path::new(&state);
}

/// Unit: session gate-allowance helpers (contract §4; mirror of the blocklist
/// session-allowance API keyed by 'rlcd-gate:<command>').
#[test]
#[serial]
fn unit_session_gate_allowance_helpers() {
    clear_session_gate_allowances();
    assert!(!is_session_gate_allowed("delete-work-unit"));
    clear_session_allowances();
    allow_for_session("rlcd-gate:delete-work-unit");
    assert!(is_session_gate_allowed("delete-work-unit"));
    clear_session_gate_allowances();
    assert!(!is_session_gate_allowed("delete-work-unit"));
}
