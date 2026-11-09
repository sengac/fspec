//! Feature: spec/features/agent-manager-busy-message-guard.feature

#[path = "support/busy_messages.rs"]
mod support;

use codelet_rpc_types::{SessionState, SessionStatus, StreamChunk};
use codelet_sessions::background_session::IncomingMessage;
use codelet_tools::agent_manager::{
    types::ContextReference, AgentManagerAction, AgentManagerResult, AwaitOutcome, SessionIdParam,
};
use serial_test::serial;
use std::sync::Arc;
use support::{assert_busy, Fixture, TestResult};

#[tokio::test]
#[serial]
async fn scenario_reject_messages_to_non_idle_recipients() -> TestResult {
    let fixture = Fixture::new().await?;
    // @step Given a recipient that is running, paused, compacting, interrupted, or cleared
    for status in [
        SessionStatus::Running,
        SessionStatus::Paused,
        SessionStatus::Compacting,
        SessionStatus::Interrupted,
        SessionStatus::Cleared,
    ] {
        fixture.target.set_status(status);
        // @step When an agent sends the recipient a message with optional quoted context
        for context in [
            None,
            Some(vec![ContextReference::Turns {
                session_id: "missing-history".to_string(),
                turns: vec![0],
            }]),
        ] {
            let result = (fixture.handler)(
                AgentManagerAction::Message {
                    session_id: fixture.target_id.clone(),
                    message: "obsolete follow-up".to_string(),
                    context,
                },
                fixture.caller,
            );
            // @step Then the response has error code "session_busy"
            // @step And the response identifies the recipient and instructs the caller to use "await_idle" before sending again
            // @step And the response advises returning results instead of waiting on a recipient that is waiting for the caller
            assert_busy(result, &fixture.target_id)?;
            // @step And no incoming message is queued and the recipient status is unchanged
            assert_eq!(fixture.target.pending_incoming_message_count(), 0);
            assert!(fixture
                .target
                .incoming_message_rx
                .lock()
                .await
                .try_recv()
                .is_err());
            assert_eq!(fixture.target.get_status(), status);
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_tool_guidance_explains_how_to_message_busy_recipients() {
    use codelet_tools::agent_manager::AgentManagerTool;
    use rig::tool::Tool;

    // @step Given an agent has access to the AgentManager tool description
    // @step When it reads the message action guidance
    let definition = AgentManagerTool::new(uuid::Uuid::new_v4())
        .definition(String::new())
        .await;
    assert!(definition.description.contains("session_busy"));
    // @step Then the guidance states that messages require idle recipients
    assert!(definition.description.contains("idle recipients"));
    // @step And it instructs the caller to use "await_idle" before sending to a busy recipient
    assert!(definition.description.contains("await_idle"));
}

#[tokio::test]
#[serial]
async fn scenario_accept_one_message_and_immediately_mark_the_recipient_running() -> TestResult {
    // @step Given an idle recipient
    let fixture = Fixture::new().await?;
    let mut stream = fixture.target.subscribe_to_stream();
    let mut status_changes = fixture.status_changes();
    // @step When an agent sends the recipient a message
    assert!(matches!(
        fixture.send("first task"),
        AgentManagerResult::MessageDelivered {
            delivered: true,
            ..
        }
    ));
    // @step And the recipient immediately reports running through its status and broadcast channels
    assert_eq!(fixture.target.get_status(), SessionStatus::Running);
    assert_eq!(fixture.target.pending_incoming_message_count(), 1);
    let (session_id, status) = status_changes.try_recv()?;
    assert_eq!(session_id.value, fixture.target_id);
    assert_eq!(status, SessionStatus::Running);
    assert!(matches!(
        stream.try_recv()?,
        StreamChunk::SessionStateChange {
            state: SessionState::Running
        }
    ));
    // @step And await_idle does not report the recipient idle before it finishes
    let result = (fixture.async_handler)(
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(fixture.target_id.clone()),
            timeout: Some(0),
        },
        fixture.caller,
    )
    .await;
    assert!(
        matches!(result, AgentManagerResult::AwaitResult { results } if results[0].status == AwaitOutcome::TimedOut)
    );
    // @step Then the message is delivered with its sender identity intact
    let incoming = fixture.target.incoming_message_rx.lock().await.try_recv()?;
    assert_eq!(incoming.message, "first task");
    assert_eq!(incoming.source_session_id, fixture.caller.to_string());
    assert_eq!(incoming.role_name, "");
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_reject_a_burst_after_the_first_accepted_message() -> TestResult {
    // @step Given an idle recipient
    let fixture = Fixture::new().await?;
    // @step When an agent sends one task followed by hundreds of redundant messages
    assert!(matches!(
        fixture.send("first task"),
        AgentManagerResult::MessageDelivered {
            delivered: true,
            ..
        }
    ));
    for _attempt in 0..300 {
        // @step And all subsequent messages return "session_busy"
        assert_busy(fixture.send("redundant task"), &fixture.target_id)?;
    }
    // @step Then only the first task is queued
    assert_eq!(fixture.target.pending_incoming_message_count(), 1);
    let mut incoming = fixture.target.incoming_message_rx.lock().await;
    assert_eq!(incoming.try_recv()?.message, "first task");
    assert!(incoming.try_recv().is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn scenario_concurrent_senders_cannot_queue_multiple_tasks_to_an_idle_recipient() -> TestResult
{
    // @step Given an idle recipient
    let fixture = Fixture::new().await?;
    // @step When multiple agents send messages concurrently
    let barrier = Arc::new(tokio::sync::Barrier::new(32));
    let mut senders = tokio::task::JoinSet::new();
    for _sender in 0..32 {
        let handler = fixture.handler.clone();
        let target_id = fixture.target_id.clone();
        let barrier = barrier.clone();
        senders.spawn(async move {
            barrier.wait().await;
            handler(
                AgentManagerAction::Message {
                    session_id: target_id,
                    message: "concurrent task".to_string(),
                    context: None,
                },
                uuid::Uuid::new_v4(),
            )
        });
    }
    let mut accepted = 0;
    while let Some(result) = senders.join_next().await {
        let result = result?;
        if matches!(
            result,
            AgentManagerResult::MessageDelivered {
                delivered: true,
                ..
            }
        ) {
            accepted += 1;
        } else {
            // @step And all other sends return "session_busy"
            assert_busy(result, &fixture.target_id)?;
        }
    }
    // @step Then exactly one message is accepted
    assert_eq!(accepted, 1);
    assert_eq!(fixture.target.pending_incoming_message_count(), 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_await_completion_before_sending_a_fresh_message() -> TestResult {
    // @step Given a recipient processing an accepted task
    let fixture = Fixture::new().await?;
    fixture.send("first task");
    // @step And a follow-up message has been rejected as busy
    assert_busy(fixture.send("stale follow-up"), &fixture.target_id)?;
    // @step When the caller awaits the recipient completing its task
    let await_completion = (fixture.async_handler)(
        AgentManagerAction::AwaitIdle {
            session_id: SessionIdParam::Single(fixture.target_id.clone()),
            timeout: Some(2),
        },
        fixture.caller,
    );
    let complete_task = async {
        tokio::task::yield_now().await;
        fixture.finish().await
    };
    let (result, finished) = tokio::join!(await_completion, complete_task);
    finished?;
    assert!(
        matches!(result, AgentManagerResult::AwaitResult { results } if results[0].status == AwaitOutcome::Idle)
    );
    // @step And sends a fresh follow-up message after the recipient becomes idle
    assert!(matches!(
        fixture.send("fresh follow-up"),
        AgentManagerResult::MessageDelivered {
            delivered: true,
            ..
        }
    ));
    // @step Then only the fresh follow-up message is delivered
    let mut incoming = fixture.target.incoming_message_rx.lock().await;
    assert_eq!(incoming.try_recv()?.message, "fresh follow-up");
    assert!(incoming.try_recv().is_err());
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_failed_delivery_leaves_an_idle_recipient_idle() -> TestResult {
    // @step Given an idle recipient whose incoming message channel is full or closed
    let fixture = Fixture::new().await?;
    for _message in 0..16 {
        fixture.target.receive_incoming_message(IncomingMessage {
            source_session_id: fixture.caller.to_string(),
            role_name: String::new(),
            message: "existing external input".to_string(),
            images: None,
            interrupt_epoch: None,
            submission_id: None,
        })?;
    }
    // @step When an agent sends the recipient a message
    let result = fixture.send("cannot queue");
    // @step Then the response has error code "delivery_failed"
    assert!(matches!(result, AgentManagerResult::Error { code, .. } if code == "delivery_failed"));
    // @step And the recipient remains idle with its pending message count unchanged
    assert_eq!(fixture.target.get_status(), SessionStatus::Idle);
    assert_eq!(fixture.target.pending_incoming_message_count(), 16);
    let mut incoming = fixture.target.incoming_message_rx.lock().await;
    while incoming.try_recv().is_ok() {}
    fixture
        .target
        .incoming_message_pending
        .store(0, std::sync::atomic::Ordering::Release);
    incoming.close();
    drop(incoming);
    let result = fixture.send("closed channel");
    assert!(matches!(result, AgentManagerResult::Error { code, .. } if code == "delivery_failed"));
    assert_eq!(fixture.target.get_status(), SessionStatus::Idle);
    assert_eq!(fixture.target.pending_incoming_message_count(), 0);
    Ok(())
}

#[tokio::test]
#[serial]
async fn scenario_active_agents_cannot_send_messages_to_themselves() -> TestResult {
    // @step Given an actively running agent
    let fixture = Fixture::new().await?;
    fixture.target.set_status(SessionStatus::Running);
    // @step When it sends a message to its own session
    let result = (fixture.handler)(
        AgentManagerAction::Message {
            session_id: fixture.target_id.clone(),
            message: "note to self".to_string(),
            context: None,
        },
        uuid::Uuid::parse_str(&fixture.target_id)?,
    );
    // @step Then the response has error code "session_busy"
    assert_busy(result, &fixture.target_id)?;
    // @step And no self-addressed message is queued
    assert_eq!(fixture.target.pending_incoming_message_count(), 0);
    Ok(())
}
