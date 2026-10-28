//! Feature: spec/features/agent-loop-interrupt-cascade.feature

#[path = "support/prompt_admission.rs"]
pub mod support;

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_rpc_types::{InterruptionReason, StreamChunk};
use std::{path::Path, sync::atomic::Ordering, time::Duration};
#[cfg(feature = "test-support")]
use support::{done, fixture, StubGuard};
use support::{new_session, originals, session_fixture, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn background_session_owns_independent_interrupt_handles() -> TestResult {
    let (directory, session, _receiver) = session_fixture()?;
    let (independent, _other_receiver) = new_session(directory.path())?;
    assert!(!std::sync::Arc::ptr_eq(
        &session.is_interrupted,
        &independent.is_interrupted
    ));
    assert!(!std::sync::Arc::ptr_eq(
        &session.interrupt_notify,
        &independent.interrupt_notify
    ));
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    let other = session.interrupt_notify.clone();
    session.interrupt();
    assert!(session.is_interrupted.load(Ordering::Acquire));
    assert!(!independent.is_interrupted.load(Ordering::Acquire));
    assert_eq!(independent.compaction_epoch(), 0);
    assert_eq!(independent.interruption_reason(), None);
    tokio::time::timeout(Duration::from_millis(100), other.notified()).await?;
    assert!(tokio::time::timeout(
        Duration::from_millis(10),
        independent.interrupt_notify.notified()
    )
    .await
    .is_err());
    assert_eq!(
        session.interruption_reason(),
        Some(InterruptionReason::User)
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn background_session_interrupt_flips_flag_and_wakes_notifier() -> TestResult {
    let (_directory, session, _receiver) = session_fixture()?;
    let notified = session.interrupt_notify.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();
    session.interrupt();
    assert!(session.is_interrupted.load(Ordering::Acquire));
    tokio::time::timeout(Duration::from_millis(100), notified).await?;
    session.reset_interrupt();
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    assert_eq!(session.interruption_reason(), None);
    Ok(())
}

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn new_turn_admission_never_resets_an_older_inputs_cancellation() -> TestResult {
    let (_directory, session, receiver, polls) = fixture()?;
    let _stub = StubGuard;
    let mut events = session.subscribe_to_stream();
    session.send_input("cancelled".into(), None)?;
    session.interrupt();
    session.send_input("fresh".into(), None)?;
    assert!(session.is_interrupted.load(Ordering::Acquire));
    let worker = tokio::spawn(codelet_agent_loop::agent_loop(
        session.clone(),
        receiver,
        tokio::sync::mpsc::channel(1).1,
    ));
    let result = done(&mut events, 2).await;
    worker.abort();
    let _ = worker.await;
    result?;
    assert_eq!(polls.load(Ordering::Acquire), 1);
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    assert!(session
        .get_buffered_output(100)
        .iter()
        .any(|chunk| matches!(
            chunk,
            StreamChunk::InterruptedWithReason {
                reason: InterruptionReason::User,
                ..
            }
        )));
    Ok(())
}

fn source(file: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file))
}

fn assert_interrupt_forwarding(call: &str, receiver: &str) -> TestResult {
    let flag = call
        .find(&format!("{receiver}.is_interrupted.clone()"))
        .ok_or("stream call missing interruption flag")?;
    let compaction = call
        .find(&format!("{receiver}.compaction_in_progress.clone()"))
        .ok_or("stream call missing compaction flag")?;
    let notify = call
        .find(&format!("{receiver}.interrupt_notify.clone()"))
        .ok_or("stream call missing interruption notifier")?;
    assert!(flag < compaction && compaction < notify);
    Ok(())
}

#[test]
fn run_with_provider_macro_forwards_both_interrupt_handles() -> TestResult {
    let dispatch = source("dispatch.rs")?;
    let call = dispatch
        .split("codelet_cli::interactive::run_agent_stream_with_images(")
        .nth(1)
        .ok_or("dispatch macro stream call missing")?
        .split(").await")
        .next()
        .ok_or("stream call missing")?;
    assert_interrupt_forwarding(call, "$session")
}

#[test]
fn openai_provider_forwards_both_interrupt_handles() -> TestResult {
    let provider = source("agent_loop/provider_openai.rs")?;
    let call = provider
        .split("run_agent_stream_with_images(")
        .nth(1)
        .ok_or("OpenAI stream call missing")?;
    assert_interrupt_forwarding(call, "session")
}

#[test]
fn custom_provider_forwards_both_interrupt_handles() -> TestResult {
    let provider = source("agent_loop/provider_custom.rs")?;
    let call = provider
        .split("run_agent_stream_with_images(")
        .nth(1)
        .ok_or("custom stream call missing")?;
    assert_interrupt_forwarding(call, "session")
}

#[tokio::test]
#[serial_test::serial]
async fn background_output_interruption_persists_partial_content_and_typed_reason() -> TestResult {
    let (_directory, session, _receiver) = session_fixture()?;
    let output = BackgroundOutput::with_provider(session.clone(), "stub".into(), None);
    output.emit(StreamEvent::Text("partial response".into()));
    session.interrupt();
    output.emit(StreamEvent::Interrupted(vec!["queued".into()]));
    let archived = originals(session.id)?;
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0]["interruptionReason"], "user");
    assert!(serde_json::to_string(&archived[0])?.contains("partial response"));
    assert!(session
        .get_buffered_output(100)
        .iter()
        .any(|chunk| matches!(
            chunk,
            StreamChunk::InterruptedWithReason { queued_inputs, reason: InterruptionReason::User }
                if queued_inputs == &["queued"]
        )));
    Ok(())
}

#[test]
fn stream_chunk_interrupted_constructor_preserves_queued_inputs() -> TestResult {
    let chunk = StreamChunk::interrupted(vec!["queued".into()]);
    assert!(
        matches!(&chunk, StreamChunk::Interrupted { queued_inputs } if queued_inputs == &["queued"])
    );
    let encoded = codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(&chunk);
    assert_eq!(encoded["type"], "interrupted");
    assert_eq!(encoded["queuedInputs"], serde_json::json!(["queued"]));
    Ok(())
}
