//! Feature: spec/features/agent-manager-await-idle.feature
//!
//! AMGR-015 await_idle types tests, moved out of tests.rs (COV-075: one test file per feature).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use super::handler::execute_agent_manager_async;
use super::handler::*;
use super::types::*;
use super::types::{AwaitOutcome, AwaitSessionResult, SessionIdParam};
use serial_test::serial;
use uuid::Uuid;

// ============================================================
// Types: deserialization and serialization
// ============================================================

#[test]
fn test_await_idle_single_session_deserializes() {
    let json = r#"{"action": "await_idle", "session_id": "abc-123"}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::AwaitIdle {
            session_id,
            timeout,
        } => {
            assert_eq!(session_id.into_vec(), vec!["abc-123"]);
            assert!(timeout.is_none());
        }
        _ => panic!("Expected AwaitIdle"),
    }
}

#[test]
fn test_await_idle_multiple_sessions_deserializes() {
    let json = r#"{"action": "await_idle", "session_id": ["abc-123", "def-456", "ghi-789"]}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::AwaitIdle {
            session_id,
            timeout,
        } => {
            let ids = session_id.into_vec();
            assert_eq!(ids, vec!["abc-123", "def-456", "ghi-789"]);
            assert!(timeout.is_none());
        }
        _ => panic!("Expected AwaitIdle"),
    }
}

#[test]
fn test_await_idle_with_timeout_deserializes() {
    let json = r#"{"action": "await_idle", "session_id": "abc-123", "timeout": 120}"#;
    let args: AgentManagerArgs = serde_json::from_str(json).unwrap();
    match args.action {
        AgentManagerAction::AwaitIdle {
            session_id,
            timeout,
        } => {
            assert_eq!(session_id.into_vec(), vec!["abc-123"]);
            assert_eq!(timeout, Some(120));
        }
        _ => panic!("Expected AwaitIdle"),
    }
}

#[test]
fn test_await_result_serialization() {
    let result = AgentManagerResult::AwaitResult {
        results: vec![
            AwaitSessionResult {
                session_id: "abc".to_string(),
                status: AwaitOutcome::Idle,
                request: None,
            },
            AwaitSessionResult {
                session_id: "def".to_string(),
                status: AwaitOutcome::TimedOut,
                request: None,
            },
            AwaitSessionResult {
                session_id: "ghi".to_string(),
                status: AwaitOutcome::Destroyed,
                request: None,
            },
        ],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("abc") && json.contains("\"idle\""));
    assert!(json.contains("def") && json.contains("\"timed_out\""));
    assert!(json.contains("ghi") && json.contains("\"destroyed\""));
}

#[test]
fn test_await_outcome_serialization() {
    assert_eq!(
        serde_json::to_string(&AwaitOutcome::Idle).unwrap(),
        "\"idle\""
    );
    assert_eq!(
        serde_json::to_string(&AwaitOutcome::TimedOut).unwrap(),
        "\"timed_out\""
    );
    assert_eq!(
        serde_json::to_string(&AwaitOutcome::Destroyed).unwrap(),
        "\"destroyed\""
    );
    assert_eq!(
        serde_json::to_string(&AwaitOutcome::Interrupted).unwrap(),
        "\"interrupted\""
    );
}

#[test]
fn test_session_id_param_into_vec() {
    assert_eq!(
        SessionIdParam::Single("a".to_string()).into_vec(),
        vec!["a"]
    );
    assert_eq!(
        SessionIdParam::Multiple(vec!["a".to_string(), "b".to_string()]).into_vec(),
        vec!["a", "b"]
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_async_handler_not_configured_returns_error() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(Uuid::new_v4().to_string()),
            timeout: Some(10),
        },
    )
    .await;
    match result {
        AgentManagerResult::Error { error, code, .. } => {
            assert!(error);
            assert_eq!(code, "internal_error");
        }
        other => panic!("Expected internal_error, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

#[tokio::test]
async fn test_tool_definition_includes_await_idle() {
    use super::super::AgentManagerTool;
    use rig::tool::Tool;
    let tool = AgentManagerTool::new(Uuid::new_v4());
    let def = tool.definition(String::new()).await;
    assert!(def.description.contains("await_idle"));
    let actions: Vec<String> = serde_json::from_value(
        def.parameters
            .get("properties")
            .unwrap()
            .get("action")
            .unwrap()
            .get("enum")
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert!(actions.contains(&"await_idle".to_string()));
    assert!(def
        .parameters
        .get("properties")
        .unwrap()
        .get("timeout")
        .is_some());
}
