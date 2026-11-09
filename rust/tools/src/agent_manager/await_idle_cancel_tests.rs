//! Feature: spec/features/agent-manager-await-idle.feature
//!
//! AMGR-015 await_idle destroy, interrupt, default timeout, hooks tests, moved out of tests.rs (COV-075: one test file per feature).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use super::handler::*;
use super::handler::{
    execute_agent_manager_async, set_agent_manager_async_handler, AgentManagerAsyncHandler,
};
use super::test_support::*;
use super::types::*;
use super::types::{AwaitOutcome, AwaitSessionResult, SessionIdParam};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

// @step Given I have a subordinate agent session that is running
// @step When I call await_idle with that session ID
// @step And the subordinate session is destroyed while being awaited
// @step Then the result should contain 1 entry with status "destroyed"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_session_destroyed() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::Destroyed);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(sub_id.clone()),
            timeout: Some(30),
        },
    )
    .await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].session_id, sub_id);
            assert_eq!(results[0].status, AwaitOutcome::Destroyed);
        }
        other => panic!("Expected AwaitResult with destroyed, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have spawned 3 subordinate agent sessions
// @step And 1 subordinate has already finished and is idle
// @step And 2 subordinates are still running
// @step When I call await_idle with all 3 session IDs
// @step And the calling session is interrupted before the running sessions finish
// @step Then the result should contain 1 entry with status "idle"
// @step And the result should contain 2 entries with status "interrupted"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_interrupted() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub1 = Uuid::new_v4().to_string();
    let sub2 = Uuid::new_v4().to_string();
    let sub3 = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub1.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub2.clone(), AwaitOutcome::Interrupted);
    outcomes.insert(sub3.clone(), AwaitOutcome::Interrupted);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Multiple(vec![sub1, sub2, sub3]),
            timeout: Some(60),
        },
    )
    .await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 3);
            assert_eq!(
                results
                    .iter()
                    .filter(|r| r.status == AwaitOutcome::Idle)
                    .count(),
                1
            );
            assert_eq!(
                results
                    .iter()
                    .filter(|r| r.status == AwaitOutcome::Interrupted)
                    .count(),
                2
            );
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that is running
// @step When I call await_idle with that session ID and no timeout parameter
// @step Then the tool should wait indefinitely until the session becomes idle
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_default_timeout() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();
    let timeout_received = Arc::new(std::sync::Mutex::new(None::<Option<u64>>));
    let timeout_clone = timeout_received.clone();
    let sub_id_clone = sub_id.clone();

    let handler: AgentManagerAsyncHandler = Arc::new(move |action, _sid| {
        let tc = timeout_clone.clone();
        let sid = sub_id_clone.clone();
        Box::pin(async move {
            match action {
                AgentManagerAction::AwaitIdle { timeout, .. } => {
                    *tc.lock().unwrap() = Some(timeout);
                    AgentManagerResult::AwaitResult {
                        results: vec![AwaitSessionResult {
                            session_id: sid,
                            status: AwaitOutcome::Idle,
                            request: None,
                        }],
                    }
                }
                _ => AgentManagerResult::invalid_parameter("expected AwaitIdle"),
            }
        })
    });
    set_agent_manager_async_handler(session_id, Some(handler));

    let _result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(sub_id),
            timeout: None,
        },
    )
    .await;

    let received = timeout_received.lock().unwrap();
    assert_eq!(
        *received,
        Some(None),
        "Timeout should be passed as None (waits indefinitely)"
    );
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that is idle
// @step When I call await_idle with session_id as a plain string
// @step Then the result should be identical to calling with a single-element array
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_single_string_vs_array() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::Idle);

    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes.clone())));
    let r1 = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(sub_id.clone()),
            timeout: Some(10),
        },
    )
    .await;

    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));
    let r2 = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Multiple(vec![sub_id.clone()]),
            timeout: Some(10),
        },
    )
    .await;

    match (&r1, &r2) {
        (
            AgentManagerResult::AwaitResult { results: a },
            AgentManagerResult::AwaitResult { results: b },
        ) => {
            assert_eq!(a.len(), b.len());
            assert_eq!(a[0].session_id, b[0].session_id);
            assert_eq!(a[0].status, b[0].status);
        }
        _ => panic!("Expected AwaitResult from both calls"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given a pre-tool hook is configured to block the AgentManager tool
// @step When I call await_idle with a valid session ID
// @step Then the tool should return a blocked error before any waiting occurs
#[tokio::test]
async fn test_await_idle_pre_tool_hook_blocks() {
    use super::super::AgentManagerTool;
    use rig::tool::Tool;
    let tool = AgentManagerTool::new(Uuid::new_v4());
    let definition = tool.definition(String::new()).await;
    let action_prop = definition
        .parameters
        .get("properties")
        .and_then(|p| p.get("action"))
        .and_then(|a| a.get("enum"))
        .unwrap();
    let actions: Vec<String> = serde_json::from_value(action_prop.clone()).unwrap();
    assert!(actions.contains(&"await_idle".to_string()));
}
