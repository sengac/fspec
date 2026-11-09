//! Feature: spec/features/agent-manager-await-idle.feature
//!
//! AMGR-015 await_idle completion, timeout and errors tests, moved out of tests.rs (COV-075: one test file per feature).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use super::handler::*;
use super::handler::{execute_agent_manager_async, set_agent_manager_async_handler};
use super::test_support::*;
use super::types::*;
use super::types::{AwaitOutcome, SessionIdParam};
use serial_test::serial;
use uuid::Uuid;

// ============================================================
// AMGR-015: AgentManager await_idle tests
// Feature: spec/features/agent-manager-await-idle.feature
// ============================================================

// @step Given I have spawned 3 subordinate agent sessions
// @step And each subordinate has been sent a task and is running
// @step When I call await_idle with all 3 session IDs
// @step Then the tool should block until all 3 sessions become idle
// @step And the result should contain 3 entries each with status "idle"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_all_complete() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub1 = Uuid::new_v4().to_string();
    let sub2 = Uuid::new_v4().to_string();
    let sub3 = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub1.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub2.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub3.clone(), AwaitOutcome::Idle);
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
            for r in &results {
                assert_eq!(r.status, AwaitOutcome::Idle);
            }
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that has finished its task
// @step And the subordinate session status is "idle"
// @step When I call await_idle with that session ID
// @step Then the tool should return immediately without waiting
// @step And the result should contain 1 entry with status "idle"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_already_idle() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::Idle);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(sub_id.clone()),
            timeout: None,
        },
    )
    .await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].session_id, sub_id);
            assert_eq!(results[0].status, AwaitOutcome::Idle);
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a subordinate agent session that is actively running a long task
// @step When I call await_idle with that session ID and timeout of 10 seconds
// @step And the subordinate does not finish within 10 seconds
// @step Then the result should contain 1 entry with status "timed_out"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_timeout() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub_id = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub_id.clone(), AwaitOutcome::TimedOut);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(sub_id.clone()),
            timeout: Some(10),
        },
    )
    .await;

    match result {
        AgentManagerResult::AwaitResult { results } => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].session_id, sub_id);
            assert_eq!(results[0].status, AwaitOutcome::TimedOut);
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have spawned 3 subordinate agent sessions
// @step And 2 subordinates will finish quickly
// @step And 1 subordinate is running a long task
// @step When I call await_idle with all 3 session IDs and timeout of 10 seconds
// @step Then the result should contain 2 entries with status "idle"
// @step And the result should contain 1 entry with status "timed_out"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_mixed_results() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let sub1 = Uuid::new_v4().to_string();
    let sub2 = Uuid::new_v4().to_string();
    let sub3 = Uuid::new_v4().to_string();

    let mut outcomes = std::collections::HashMap::new();
    outcomes.insert(sub1.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub2.clone(), AwaitOutcome::Idle);
    outcomes.insert(sub3.clone(), AwaitOutcome::TimedOut);
    set_agent_manager_async_handler(session_id, Some(mock_async_handler(outcomes)));

    let result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Multiple(vec![sub1, sub2, sub3]),
            timeout: Some(10),
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
                2
            );
            assert_eq!(
                results
                    .iter()
                    .filter(|r| r.status == AwaitOutcome::TimedOut)
                    .count(),
                1
            );
        }
        other => panic!("Expected AwaitResult, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}

// @step Given I have a session ID that does not correspond to any active session
// @step When I call await_idle with that non-existent session ID
// @step Then the tool should return immediately with an error
// @step And the error code should be "session_not_found"
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_await_idle_nonexistent_session() {
    clear_all_agent_manager_handlers();
    let session_id = Uuid::new_v4();
    let nonexistent_id = Uuid::new_v4().to_string();

    set_agent_manager_async_handler(
        session_id,
        Some(mock_async_handler(std::collections::HashMap::new())),
    );

    let result = execute_agent_manager_async(
        session_id,
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(nonexistent_id.clone()),
            timeout: Some(10),
        },
    )
    .await;

    match result {
        AgentManagerResult::Error {
            error,
            code,
            message,
        } => {
            assert!(error);
            assert_eq!(code, "session_not_found");
            assert!(message.contains(&nonexistent_id));
        }
        other => panic!("Expected Error with session_not_found, got: {other:?}"),
    }
    clear_all_agent_manager_handlers();
}
