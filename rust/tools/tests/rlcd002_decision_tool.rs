//! RLCD-002 — Decision() first-class rig tool for typed RLCD decisions
//! (choice/score/noul).
//!
//! Feature: spec/features/decision-first-class-rig-tool-for-typed-rlcd-decisions-choice-score-noul.feature
//!
//! Covers the Decision tool contract (see the RLCD-002 architecture notes):
//! typed question rendering (choice => argmax + confidence + probabilities,
//! score => score + legend, noul => P(true)), batch questions in one request,
//! argument validation (state non-empty, 1..=100 questions, choice/score
//! criteria 2..=50 — all BEFORE any network I/O), error mapping (422 =>
//! Validation with the server detail verbatim, 429 => Execution "queue busy",
//! 500/timeout => Execution, unreachable => fail-open structured error with a
//! hint), the model echo rule (request.model always comes from /health, never
//! hardcoded), and provider registration (all 7 create_rig_agent sites; the
//! DeepSearch sub-agent list stays unchanged).
//!
//! ACDD red phase: at test-writing time `codelet_tools::rlcd::decision`
//! (DecisionTool / DecisionArgs) does not exist yet; this suite fails to
//! compile/link until the implementing phase provides the contracted API.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "rlcd_mock.rs"]
mod rlcd_mock;

use std::time::Duration;

use codelet_tools::rlcd::{DecisionArgs, DecisionTool};
use codelet_tools::ToolError;
use rlcd_mock::MockRlcd;
use rig::tool::Tool;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// helpers
// ============================================================================

/// A valid choice question (the shape used by most "happy path" scenarios).
fn choice_question() -> serde_json::Value {
    json!({
        "q1": {
            "type": "choice",
            "criteria": {
                "proceed": "Go ahead with the decision",
                "hold": "Wait and gather more information",
                "ask_user": "Ask the user to decide"
            }
        }
    })
}

/// Start a ready mock and build DecisionArgs pointing at it.
async fn ready_tool_and_args(url_override: bool) -> (MockRlcd, DecisionArgs) {
    let mock = MockRlcd::ready("typed-decisions").start().await;
    let args = DecisionArgs {
        state: "customer requests refund of order 123".to_string(),
        questions: choice_question(),
        model: None,
        url: if url_override {
            Some(mock.base_url())
        } else {
            None
        },
    };
    (mock, args)
}

/// Assert `result` is a ToolError whose Display contains every `needle`
/// (case-insensitive). Consumes the result — call it exactly once per result.
fn assert_error_contains(result: Result<String, ToolError>, needles: &[&str]) {
    let err = result.unwrap_err().to_string();
    let lower = err.to_lowercase();
    for needle in needles {
        assert!(
            lower.contains(&needle.to_lowercase()),
            "error must mention {needle:?}, got: {err}"
        );
    }
}

// ============================================================================
// QUESTION TYPES (rule 5; examples 1, 2, 4)
// ============================================================================

/// Scenario: A choice question returns the argmax criterion with confidence
/// and full probabilities
#[tokio::test]
async fn scenario_choice_returns_argmax_confidence_probabilities() {
    // @step Given a mock RLCD server whose /health reports model "typed-decisions"
    let (mock, args) = ready_tool_and_args(true).await;
    mock.set_answers(
        r#"{"model":"typed-decisions","answers":{"q1":{"type":"choice","choice":"proceed","confidence":0.8,"probabilities":{"proceed":0.8,"hold":0.15,"ask_user":0.05}}},"usage":{"input_tokens":100,"output_tokens":0}}"#,
    );

    // @step And the state "customer requests refund of order 123"
    assert_eq!(args.state, "customer requests refund of order 123");

    // @step When the Decision tool is called with choice question "should we refund?" and criteria {proceed, hold, ask_user}
    let tool = DecisionTool::new(Uuid::new_v4());
    let out = tool.call(args).await.expect("choice call must succeed");

    // @step Then the response reports the argmax criterion "proceed"
    assert!(
        out.contains("proceed"),
        "choice output must name the argmax criterion 'proceed': {out}"
    );
    // @step And the response reports the confidence for "proceed"
    assert!(
        out.contains("0.8"),
        "choice output must report the confidence 0.8: {out}"
    );
    // @step And the response reports the full probability distribution for all three criteria
    for crit in ["hold", "ask_user"] {
        assert!(
            out.contains(crit),
            "choice output must include criterion {crit}: {out}"
        );
    }
    for p in ["0.15", "0.05"] {
        assert!(out.contains(p), "choice output must include probability {p}: {out}");
    }
    mock.stop().await;
}

/// Scenario: A score question returns per-criterion scores with a legend
#[tokio::test]
async fn scenario_score_returns_scores_with_legend() {
    // @step Given a mock RLCD server whose /health reports model "typed-decisions"
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    args.questions = json!({
        "q1": {
            "type": "score",
            "criteria": ["Routine", "Important", "Critical"]
        }
    });
    mock.set_answers(
        r#"{"model":"typed-decisions","answers":{"q1":{"type":"score","score":1.75,"confidence":0.8,"probabilities":{"0":0.05,"1":0.15,"2":0.8},"legend":{"0":"Routine","1":"Important","2":"Critical"}}},"usage":{"input_tokens":120,"output_tokens":0}}"#,
    );

    // @step When the Decision tool is called with a score question whose criteria are an array in rubric order
    let tool = DecisionTool::new(Uuid::new_v4());
    let out = tool.call(args).await.expect("score call must succeed");

    // @step Then the response reports a score for each criterion
    assert!(
        out.contains("1.75"),
        "score output must report the score 1.75: {out}"
    );
    // @step And the response includes a legend mapping each qid to its label
    for label in ["Routine", "Important", "Critical"] {
        assert!(
            out.contains(label),
            "score output must include the legend label {label}: {out}"
        );
    }
    mock.stop().await;
}

/// Scenario: A noul question returns P(true) as a single probability
#[tokio::test]
async fn scenario_noul_returns_p_true() {
    // @step Given a mock RLCD server whose /health reports model "typed-decisions"
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    args.questions = json!({ "q1": { "type": "noul" } });
    mock.set_answers(
        r#"{"model":"typed-decisions","answers":{"q1":{"type":"noul","noul":0.9}},"usage":{"input_tokens":40,"output_tokens":0}}"#,
    );

    // @step When the Decision tool is called with a noul question without criteria
    let tool = DecisionTool::new(Uuid::new_v4());
    let out = tool.call(args).await.expect("noul call must succeed");

    // @step Then the response reports P(true) as a single probability
    assert!(
        out.to_lowercase().contains("p(true)"),
        "noul output must report P(true): {out}"
    );
    assert!(
        out.contains("0.9"),
        "noul output must report the probability 0.9: {out}"
    );
    mock.stop().await;
}

// ============================================================================
// BATCH QUESTIONS (rule 1; example 5)
// ============================================================================

/// Scenario: One request carrying both a choice and a score question returns
/// both results
#[tokio::test]
async fn scenario_batch_choice_and_score_in_one_request() {
    // @step Given a mock RLCD server whose /health reports model "typed-decisions"
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    // @step And the state "customer requests refund of order 123"
    assert_eq!(args.state, "customer requests refund of order 123");
    args.questions = json!({
        "q_choice": {
            "type": "choice",
            "criteria": {
                "proceed": "Go ahead",
                "hold": "Wait"
            }
        },
        "q_score": {
            "type": "score",
            "criteria": ["Low", "High"]
        }
    });
    mock.set_answers(
        r#"{"model":"typed-decisions","answers":{"q_choice":{"type":"choice","choice":"proceed","confidence":0.7,"probabilities":{"proceed":0.7,"hold":0.3}},"q_score":{"type":"score","score":1.0,"confidence":0.7,"probabilities":{"0":0.3,"1":0.7},"legend":{"0":"Low","1":"High"}}},"usage":{"input_tokens":200,"output_tokens":0}}"#,
    );

    // @step When the Decision tool is called with one choice question and one score question in a single request
    let tool = DecisionTool::new(Uuid::new_v4());
    let out = tool.call(args).await.expect("batch call must succeed");

    // @step Then the response renders the choice result with argmax and probabilities
    assert!(
        out.contains("proceed"),
        "batch output must render the choice argmax 'proceed': {out}"
    );
    // @step And the response renders the score result with scores and legend
    assert!(
        out.contains("1.0"),
        "batch output must render the score 1.0: {out}"
    );
    assert!(
        out.contains("High"),
        "batch output must render the score legend label 'High': {out}"
    );
    // a single classifier request carried both questions
    assert_eq!(
        mock.classifier_hits(),
        1,
        "both questions must go over the wire in ONE request"
    );
    mock.stop().await;
}

// ============================================================================
// ARG VALIDATION (rules 4, 5; examples 6, 7, 8)
// ============================================================================

/// Scenario: A call without state is rejected before any network call
#[tokio::test]
async fn scenario_empty_state_rejected_before_network() {
    // @step Given a mock RLCD server that records every request it receives
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    args.state = "   ".to_string(); // whitespace-only counts as empty

    // @step When the Decision tool is called with an empty state and one valid question
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns a Validation error naming the state field
    assert!(
        matches!(result, Err(ToolError::Validation { .. })),
        "expected Validation, got {result:?}"
    );
    assert_error_contains(result, &["state"]);
    // @step And the mock server received zero requests
    assert_eq!(
        mock.classifier_hits(),
        0,
        "validation must short-circuit before any network I/O"
    );
    mock.stop().await;
}

/// Scenario: A call with zero questions is rejected before any network call
#[tokio::test]
async fn scenario_zero_questions_rejected_before_network() {
    // @step Given a mock RLCD server that records every request it receives
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    args.questions = json!({}); // empty question map

    // @step When the Decision tool is called with a valid state and no questions
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns a Validation error naming the questions field
    assert!(
        matches!(result, Err(ToolError::Validation { .. })),
        "expected Validation, got {result:?}"
    );
    assert_error_contains(result, &["questions"]);
    // @step And the mock server received zero requests
    assert_eq!(
        mock.classifier_hits(),
        0,
        "validation must short-circuit before any network I/O"
    );
    mock.stop().await;
}

/// Scenario: A call with more than 100 questions is rejected before any
/// network call
#[tokio::test]
async fn scenario_too_many_questions_rejected_before_network() {
    // @step Given a mock RLCD server that records every request it receives
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    let mut many = serde_json::Map::new();
    for i in 0..101 {
        many.insert(format!("q{i}"), json!({ "type": "noul" }));
    }
    args.questions = serde_json::Value::Object(many);

    // @step When the Decision tool is called with a valid state and 101 questions
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns a Validation error explaining the 1 to 100 questions bound
    assert!(
        matches!(result, Err(ToolError::Validation { .. })),
        "expected Validation, got {result:?}"
    );
    assert_error_contains(result, &["100", "questions"]);
    // @step And the mock server received zero requests
    assert_eq!(
        mock.classifier_hits(),
        0,
        "validation must short-circuit before any network I/O"
    );
    mock.stop().await;
}

/// Scenario: A choice question with a single criterion is rejected with the
/// bound explained
#[tokio::test]
async fn scenario_choice_single_criterion_rejected() {
    // @step Given a mock RLCD server that records every request it receives
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    args.questions =
        json!({ "q1": { "type": "choice", "criteria": { "only": "only one" } } });

    // @step When the Decision tool is called with a choice question whose criteria map has exactly one entry
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns a Validation error explaining the 2 to 50 criteria bound
    assert!(
        matches!(result, Err(ToolError::Validation { .. })),
        "expected Validation, got {result:?}"
    );
    assert_error_contains(result, &["criteria", "50"]);
    // @step And the mock server received zero requests
    assert_eq!(
        mock.classifier_hits(),
        0,
        "validation must short-circuit before any network I/O"
    );
    mock.stop().await;
}

/// Scenario: A score question with a single-entry criteria array is rejected
#[tokio::test]
async fn scenario_score_single_criterion_rejected() {
    // @step Given a mock RLCD server that records every request it receives
    let (mock, _base) = ready_tool_and_args(true).await;
    let mut args = _base;
    args.questions = json!({ "q1": { "type": "score", "criteria": ["only one"] } });

    // @step When the Decision tool is called with a score question whose criteria array has exactly one entry
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns a Validation error explaining the 2 to 50 criteria bound
    assert!(
        matches!(result, Err(ToolError::Validation { .. })),
        "expected Validation, got {result:?}"
    );
    assert_error_contains(result, &["criteria", "50"]);
    // @step And the mock server received zero requests
    assert_eq!(
        mock.classifier_hits(),
        0,
        "validation must short-circuit before any network I/O"
    );
    mock.stop().await;
}

// ============================================================================
// ERROR MAPPING (rule 3; examples 3, 9, 10, 13)
// ============================================================================

/// Scenario: A 422 response surfaces the first detail verbatim
#[tokio::test]
async fn scenario_422_surfaces_detail_verbatim() {
    // @step Given a mock RLCD server that answers /v1/classifier with 422 and detail "Loaded model is 'typed-decisions'"
    let (mock, _base) = ready_tool_and_args(true).await;
    let args = _base;
    mock.set_status(
        422,
        r#"{"error":{"message":"Loaded model is 'typed-decisions'","type":"invalid_request_error","code":422,"details":[{"param":"model","message":"Loaded model is 'typed-decisions'","type":"model_mismatch"}]}}"#,
    );

    // @step When the Decision tool is called with a valid choice question
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns a Validation error containing the detail "Loaded model is 'typed-decisions'"
    assert!(
        matches!(result, Err(ToolError::Validation { .. })),
        "422 must map to Validation, got {result:?}"
    );
    assert_error_contains(result, &["Loaded model is 'typed-decisions'"]);
    mock.stop().await;
}

/// Scenario: A 429 response with Retry-After reports the busy wait
#[tokio::test]
async fn scenario_429_reports_busy_wait() {
    // @step Given a mock RLCD server that answers /v1/classifier with 429 and Retry-After "5"
    let (mock, _base) = ready_tool_and_args(true).await;
    let args = _base;
    mock.set_status(429, r#"{"detail":"Scoring queue is full"}"#);
    mock.set_retry_after("5");

    // @step When the Decision tool is called with a valid choice question
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns an Execution error matching "RLCD queue busy, retry in 5s"
    assert!(
        matches!(result, Err(ToolError::Execution { .. })),
        "429 must map to Execution, got {result:?}"
    );
    assert_error_contains(result, &["retry in 5s", "queue busy"]);
    mock.stop().await;
}

/// Scenario: A 500 response maps to an Execution error
#[tokio::test]
async fn scenario_500_maps_to_execution() {
    // @step Given a mock RLCD server that answers /v1/classifier with 500
    let (mock, _base) = ready_tool_and_args(true).await;
    let args = _base;
    mock.set_status(500, r#"{"error":"internal model failure"}"#);

    // @step When the Decision tool is called with a valid choice question
    let tool = DecisionTool::new(Uuid::new_v4());
    let result = tool.call(args).await;

    // @step Then the tool returns an Execution error with the server failure detail
    assert!(
        matches!(result, Err(ToolError::Execution { .. })),
        "500 must map to Execution, got {result:?}"
    );
    assert_error_contains(result, &["500"]);
    mock.stop().await;
}

/// Scenario: A timed-out request maps to an Execution error
#[tokio::test]
async fn scenario_timeout_maps_to_execution() {
    // @step Given a mock RLCD server that delays /v1/classifier past the request budget
    let (mock, _base) = ready_tool_and_args(true).await;
    let args = _base;
    mock.set_delay_ms(1500); // far longer than the test's short request budget

    // @step When the Decision tool is called with a valid choice question
    // (a short request budget so the test does not wait on the 60s default)
    let tool =
        DecisionTool::with_budgets(Uuid::new_v4(), Duration::from_secs(5), Duration::from_millis(300));
    let started = std::time::Instant::now();
    let result = tool.call(args).await;
    let elapsed = started.elapsed();

    // @step Then the tool returns an Execution error with the timeout detail
    assert!(
        matches!(result, Err(ToolError::Execution { .. })),
        "timeout must map to Execution, got {result:?}"
    );
    assert_error_contains(result, &["timed out"]);
    assert!(
        elapsed < Duration::from_secs(3),
        "the request must be aborted at the budget, not wait for the full delay: {elapsed:?}"
    );
    mock.stop().await;
}

// ============================================================================
// FAIL-OPEN (rule 3; example 7)
// ============================================================================

/// Scenario: An unreachable RLCD returns a fail-open structured error with a
/// hint
#[tokio::test]
async fn scenario_unreachable_fail_open_with_hint() {
    // @step Given no RLCD server is reachable at the configured url
    // (a private, unroutable IP: connect-only, never spawned, health check
    //  fails at the transport layer)
    let unreachable_url = "http://10.255.255.1:9999";
    let args = DecisionArgs {
        state: "some state".to_string(),
        questions: choice_question(),
        model: None,
        url: Some(unreachable_url.to_string()),
    };

    // @step When the Decision tool is called with a valid choice question
    // (a short ensure budget so the fail-open degradation is bounded and the
    //  test does not wait on the 10s default)
    let tool =
        DecisionTool::with_budgets(Uuid::new_v4(), Duration::from_millis(1500), Duration::from_millis(500));
    let started = std::time::Instant::now();
    let result = tool.call(args).await;
    let elapsed = started.elapsed();

    // @step Then the tool returns a structured error identifying RLCD as unreachable
    assert!(result.is_err(), "unreachable RLCD must return an error, not a result");
    let err = result.unwrap_err().to_string();
    assert!(
        err.to_lowercase().contains("rlcd unreachable"),
        "the structured error must identify RLCD as unreachable: {err}"
    );
    // @step And the error includes a hint to start the backend or set rlcd.url
    assert!(
        err.to_lowercase().contains("rlcd.url"),
        "the fail-open error must include the rlcd.url hint: {err}"
    );
    // @step And the caller is not hard-blocked
    assert!(
        elapsed < Duration::from_secs(8),
        "fail-open must degrade within a bounded wait, took {elapsed:?}"
    );
}

// ============================================================================
// MODEL ECHO (rule 2; the model name is never hardcoded)
// ============================================================================

/// Scenario: The request echoes the server-reported model name
#[tokio::test]
async fn scenario_request_echoes_server_reported_model() {
    // @step Given a mock RLCD server whose /health reports model "custom-variant"
    let mock = MockRlcd::ready("custom-variant").start().await;
    mock.set_answers(
        r#"{"model":"custom-variant","answers":{"q1":{"type":"choice","choice":"proceed","confidence":0.5,"probabilities":{"proceed":0.5,"hold":0.5}}},"usage":{"input_tokens":10,"output_tokens":0}}"#,
    );
    let args = DecisionArgs {
        state: "some state".to_string(),
        questions: choice_question(),
        model: None, // default: must come from /health, not hardcoded
        url: Some(mock.base_url()),
    };

    // @step When the Decision tool is called with a valid choice question
    let tool = DecisionTool::new(Uuid::new_v4());
    let _out = tool.call(args).await.expect("call must succeed");

    // @step Then the classifier request body carries model "custom-variant"
    let bodies = mock.classifier_bodies();
    assert_eq!(bodies.len(), 1, "exactly one classifier request expected");
    let req: serde_json::Value = serde_json::from_str(&bodies[0]).expect("request body is JSON");
    assert_eq!(
        req.get("model").and_then(serde_json::Value::as_str),
        Some("custom-variant"),
        "request.model must echo the /health-reported model, not a hardcoded constant"
    );
    // @step And the request never hardcodes a model name different from the /health report
    assert_ne!(
        req.get("model").and_then(serde_json::Value::as_str),
        Some("typed-decisions"),
        "the model must not be a default constant when the server reports a different one"
    );
    mock.stop().await;
}

// ============================================================================
// INTEGRATION (rule 2; example 11)
// ============================================================================

/// Provider source files that must register the Decision tool (all 7
/// create_rig_agent sites). Paths are relative to the codelet-tools manifest
/// dir (rust/tools), so `../providers/src/...`.
const PROVIDER_FILES: &[&str] = &[
    "../providers/src/claude.rs",
    "../providers/src/openai.rs",
    "../providers/src/gemini.rs",
    "../providers/src/codex/mod.rs",
    "../providers/src/zai.rs",
    "../providers/src/custom/custom_provider.rs",
    "../providers/src/copilot/rig_agent.rs",
];

/// Scenario: The Decision tool is registered in every provider rig agent
#[test]
fn scenario_registered_in_every_provider() {
    // @step Given the RLCD supervisor is available
    // (verified by the other scenarios exercising DecisionTool end-to-end)

    // @step When each provider's rig agent is created
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let root = std::path::Path::new(&manifest);
    let mut missing = Vec::new();
    for rel in PROVIDER_FILES {
        let path = root.join(rel);
        let Ok(src) = std::fs::read_to_string(&path) else {
            panic!("provider source {rel} not found at {path:?}");
        };
        if !src.contains("DecisionTool::new(session_id)") {
            missing.push(rel);
        }
    }

    // @step Then the tool list includes the Decision tool alongside DeepSearch and Schedule
    assert!(
        missing.is_empty(),
        "DecisionTool must be registered in every provider rig agent; missing in: {missing:?}"
    );
    for rel in PROVIDER_FILES {
        let path = root.join(rel);
        let src = std::fs::read_to_string(&path).expect("provider source readable");
        assert!(
            src.contains("DeepSearchTool::new(session_id)"),
            "{rel} must still register DeepSearchTool (Decision goes alongside it)"
        );
        assert!(
            src.contains("ScheduleTool::new(session_id)"),
            "{rel} must still register ScheduleTool (Decision goes alongside it)"
        );
    }

    // @step And the DeepSearch sub-agent tool lists are unchanged
    use codelet_tools::deep_search::{SUB_AGENT_TOOL_COUNT, SUB_AGENT_TOOL_NAMES};
    assert_eq!(
        SUB_AGENT_TOOL_COUNT, 7,
        "the DeepSearch sub-agent must keep exactly 7 tools"
    );
    assert!(
        !SUB_AGENT_TOOL_NAMES.contains(&"Decision"),
        "Decision must NOT be added to the DeepSearch sub-agent tool list"
    );
}

// ============================================================================
// definition() schema shape (the tool's hand-written JSON schema contract)
// ============================================================================

/// The tool exposes a hand-written JSON schema naming its parameters.
#[tokio::test]
async fn definition_schema_names_parameters() {
    let tool = DecisionTool::new(Uuid::new_v4());
    let def = tool.definition(String::new()).await;
    assert_eq!(def.name, "Decision");
    assert!(!def.description.is_empty());
    let props = def.parameters.get("properties").expect("schema has properties");
    for key in ["state", "questions", "model", "url"] {
        assert!(
            props.get(key).is_some(),
            "schema must document the {key} parameter"
        );
    }
}
