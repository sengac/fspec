//! Feature: spec/features/typed-interruption-provenance.feature

#[path = "support/bug220.rs"]
mod support;
#[path = "../../providers/tests/fixtures/prov148.rs"]
mod transport;

use codelet_cli::interactive::run_agent_stream;
use codelet_core::RigAgent;
use codelet_providers::codex::session::CodexSession;
use rig::tool::Tool;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use support::{detector_abort, fixture, interruptions, TestResult};

async fn teardown(
    session: &codelet_sessions::background_session::BackgroundSession,
    output: &codelet_agent_loop::BackgroundOutput,
) -> TestResult {
    let agent =
        rig::agent::AgentBuilder::new(codelet_providers::stub_model::StubModel::new()).build();
    run_agent_stream(
        RigAgent::with_default_depth(agent),
        "work",
        &mut *session.inner.lock().await,
        session.is_interrupted.clone(),
        Arc::new(AtomicBool::new(false)),
        session.interrupt_notify.clone(),
        output,
        session.id,
    )
    .await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_a_genuine_loop_abort_survives_teardown_and_persistence() -> TestResult {
    // @step Given a session whose streaming detector aborts sustained repetitive output
    let (_directory, session, output) = fixture().await?;
    detector_abort(&output);
    assert!(session.is_interrupted.load(Ordering::Acquire));
    // @step When the streaming consumer tears down the interrupted response
    teardown(&session, &output).await?;
    // @step Then the interruption event and persisted assistant metadata identify the loop detector
    let events = interruptions(&session);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["reason"], "loopDetector");
    let manifest = codelet_core::persistence::load_session(session.id)?;
    let messages = codelet_core::persistence::get_session_messages(&manifest)?;
    assert!(messages
        .iter()
        .any(|message| message.metadata.get("interruptionReason") == Some(&json!("loopDetector"))));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_user_stop_takes_precedence_in_either_arrival_order() -> TestResult {
    // @step Given a session receiving a user stop and a genuine detector abort
    for user_first in [true, false] {
        let (_directory, session, output) = fixture().await?;
        let previous = session.compaction_epoch();
        // @step When both interruption orders are processed before stream teardown
        if user_first {
            session.interrupt();
        }
        detector_abort(&output);
        if !user_first {
            session.interrupt();
        }
        teardown(&session, &output).await?;
        // @step Then the final cause is user and no loop recovery note remains
        assert!(!session.has_pending_loop_abort_note());
        assert_eq!(interruptions(&session)[0]["reason"], "user");
        assert!(session.compaction_cancelled(previous));
        session.reset_interrupt();
        assert!(!session.is_interrupted.load(Ordering::Acquire));
    }
    Ok(())
}

struct InterruptTool {
    session: Arc<codelet_sessions::background_session::BackgroundSession>,
    output: Arc<codelet_agent_loop::BackgroundOutput>,
    user: bool,
    dropped: Arc<AtomicBool>,
    release: Arc<tokio::sync::Notify>,
}

struct Running(Arc<AtomicBool>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl Tool for InterruptTool {
    const NAME: &'static str = "inspect";
    type Error = std::io::Error;
    type Args = Value;
    type Output = Value;
    async fn definition(&self, _: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.into(),
            description: "Wait for interruption".into(),
            parameters: json!({"type":"object"}),
        }
    }
    async fn call(&self, _: Value) -> Result<Value, Self::Error> {
        let _running = Running(self.dropped.clone());
        if self.user {
            self.session.interrupt();
        } else {
            detector_abort(&self.output);
        }
        self.release.notified().await;
        Err(std::io::Error::other("fixture released after interruption"))
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_running_tool_cancellation_preserves_its_initiating_cause() -> TestResult {
    // @step Given a committed Codex tool batch with a running tool
    for user in [false, true] {
        let (_directory, session, output) = fixture().await?;
        let calls: Vec<_> = (1..=2).map(|index| json!({"type":"function_call","id":format!("item-{index}"),"call_id":format!("call-{index}"),"name":"inspect","arguments":"{}"})).collect();
        let server = transport::Server::start(vec![transport::event(json!({"type":"response.completed","response":{"id":"response-1","status":"completed","output":calls,"end_turn":true,"usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}}}))], false).await?;
        let dropped = Arc::new(AtomicBool::new(false));
        let release = Arc::new(tokio::sync::Notify::new());
        let mut inner = session.inner.lock().await;
        *inner = codelet_cli::session::Session::from_provider_manager(
            codelet_providers::ProviderManager::for_testing(
                codelet_providers::ProviderType::Codex,
                Some(272_000),
                Some(4096),
            ),
        );
        let history = inner
            .provider_session
            .extension::<CodexSession>("codex")
            .ok_or("Codex extension missing")?
            .history
            .clone();
        let agent = rig::agent::AgentBuilder::new(server.models()?.0.with_history(history))
            .preamble("Test instructions")
            .tool(InterruptTool {
                session: session.clone(),
                output: output.clone(),
                user,
                dropped: dropped.clone(),
                release: release.clone(),
            })
            .build();
        // @step When user stop or the loop detector interrupts the running tool
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                "inspect",
                &mut inner,
                session.is_interrupted.clone(),
                Arc::new(AtomicBool::new(false)),
                session.interrupt_notify.clone(),
                output.as_ref(),
                session.id,
            ),
        )
        .await??;
        // @step Then unfinished tools are cancelled exactly once with the actual cause before interrupted and done
        let chunks: Vec<_> = session
            .get_buffered_output(10000)
            .iter()
            .map(codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value)
            .collect();
        let interrupted = chunks
            .iter()
            .position(|chunk| chunk["type"] == "interrupted")
            .ok_or("missing interruption")?;
        let results: Vec<_> = chunks[..interrupted]
            .iter()
            .filter(|chunk| chunk["type"] == "toolResult")
            .collect();
        assert_eq!(results.len(), 2);
        let expected = if user { "user" } else { "loopDetector" };
        assert_eq!(chunks[interrupted]["reason"], expected);
        for result in results {
            let content = result["toolResult"]["content"]
                .as_str()
                .ok_or("tool content missing")?;
            assert!(
                content.contains(if user {
                    "user interruption"
                } else {
                    "loop detector"
                }),
                "{content}"
            );
        }
        assert!(chunks[interrupted + 1..]
            .iter()
            .any(|chunk| chunk["type"] == "done"));
        release.notify_one();
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !dropped.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await?;
    }
    Ok(())
}
