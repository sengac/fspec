//! Feature: spec/features/agent-loop-interrupt-cascade.feature

#[path = "support/prompt_admission.rs"]
pub mod support;

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_rpc_types::{InterruptionReason, StreamChunk};
use std::{path::Path, sync::atomic::Ordering};
#[cfg(feature = "test-support")]
use support::{done, fixture, StubGuard};
use support::{new_session, originals, session_fixture, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn background_session_owns_independent_interrupt_handles() -> TestResult {
    // @step Given a BackgroundSession created via the session test fixture
    let (directory, session, _receiver) = session_fixture()?;
    // @step And a second independent BackgroundSession
    let (independent, _other_receiver) = new_session(directory.path())?;
    // @step Then it declares a `pub is_interrupted: Arc<AtomicBool>` field
    assert!(!std::sync::Arc::ptr_eq(
        &session.is_interrupted,
        &independent.is_interrupted
    ));
    // @step And it declares a `pub interrupt_notify: Arc<Notify>` field
    assert!(!std::sync::Arc::ptr_eq(
        &session.interrupt_notify,
        &independent.interrupt_notify
    ));
    // @step And a fresh session starts un-interrupted with handles not shared with any other session
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    let other = session.interrupt_notify.clone();
    session.interrupt();
    assert!(session.is_interrupted.load(Ordering::Acquire));
    assert!(!independent.is_interrupted.load(Ordering::Acquire));
    assert_eq!(independent.compaction_epoch(), 0);
    assert_eq!(independent.interruption_reason(), None);
    codelet_test_helpers::deadlock_guard("other.notified()", other.notified()).await?;
    // TEST-042: polled once, no waiting window; a notification meant for
    // the independent session would already be stored.
    assert!(futures::FutureExt::now_or_never(independent.interrupt_notify.notified()).is_none());
    assert_eq!(
        session.interruption_reason(),
        Some(InterruptionReason::User)
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn background_session_interrupt_flips_flag_and_wakes_notifier() -> TestResult {
    // @step Given a `codelet_sessions::background_session::BackgroundSession` constructed via test helpers
    let (_directory, session, _receiver) = session_fixture()?;
    // @step And a tokio task awaiting `session.interrupt_notify.notified()` (registered BEFORE interrupt was called) is woken within 100ms
    let notified = session.interrupt_notify.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();
    // @step When I call `session.interrupt()`
    session.interrupt();
    // @step Then `session.is_interrupted.load(Ordering::Acquire)` returns `true`
    assert!(session.is_interrupted.load(Ordering::Acquire));
    codelet_test_helpers::deadlock_guard("notified", notified).await?;
    // @step And calling `session.reset_interrupt()` flips `session.is_interrupted.load(Ordering::Acquire)` back to `false`
    session.reset_interrupt();
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    assert_eq!(session.interruption_reason(), None);
    Ok(())
}

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn new_turn_admission_never_resets_an_older_inputs_cancellation() -> TestResult {
    // @step Given a queued input cancelled before admission
    let (_directory, session, receiver, polls) = fixture()?;
    let _stub = StubGuard;
    let mut events = session.subscribe_to_stream();
    session.send_input("cancelled".into(), None)?;
    session.interrupt();
    // @step When a genuinely new user input is submitted
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
    // @step Then the cancelled input never reaches inference
    assert_eq!(polls.load(Ordering::Acquire), 1);
    // @step And only admission of the new input clears its interruption state
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
    // @step Given the source of `rust/agent-loop/src/dispatch.rs`
    let dispatch = source("dispatch.rs")?;
    // @step When I locate the `run_with_provider!` macro body
    let call = dispatch
        .split("codelet_cli::interactive::run_agent_stream_with_images(")
        .nth(1)
        .ok_or("dispatch macro stream call missing")?
        .split(").await")
        .next()
        .ok_or("stream call missing")?;
    let args: Vec<&str> = call
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect();
    // @step Then the body's call to `codelet_cli::interactive::run_agent_stream_with_images` passes `$session.is_interrupted.clone()` as positional arg 5
    assert_eq!(args.get(4), Some(&"$session.is_interrupted.clone()"));
    // @step And the call passes `$session.interrupt_notify.clone()` as positional arg 7
    assert_eq!(args.get(6), Some(&"$session.interrupt_notify.clone()"));
    assert_interrupt_forwarding(call, "$session")
}

#[test]
fn openai_provider_forwards_both_interrupt_handles() -> TestResult {
    // @step Given the current OpenAI and custom-provider dispatch implementations
    // @step When I locate the OpenAI stream call
    let provider = source("agent_loop/provider_openai.rs")?;
    let call = provider
        .split("run_agent_stream_with_images(")
        .nth(1)
        .ok_or("OpenAI stream call missing")?;
    // @step Then the body's `run_agent_stream_with_images` call passes `session.is_interrupted.clone()` and `session.interrupt_notify.clone()`
    assert_interrupt_forwarding(call, "session")
}

#[test]
fn custom_provider_forwards_both_interrupt_handles() -> TestResult {
    // @step When I locate the custom-provider stream call
    let provider = source("agent_loop/provider_custom.rs")?;
    let call = provider
        .split("run_agent_stream_with_images(")
        .nth(1)
        .ok_or("custom stream call missing")?;
    // @step Then the body's `run_agent_stream_with_images` call also passes `session.is_interrupted.clone()` and `session.interrupt_notify.clone()`
    assert_interrupt_forwarding(call, "session")
}

#[tokio::test]
#[serial_test::serial]
async fn background_output_interruption_persists_partial_content_and_typed_reason() -> TestResult {
    // @step Given a real session with partial assistant content
    let (_directory, session, _receiver) = session_fixture()?;
    let output = BackgroundOutput::with_provider(session.clone(), "stub".into(), None);
    output.emit(StreamEvent::Text("partial response".into()));
    // @step When a user interruption is emitted
    session.interrupt();
    output.emit(StreamEvent::Interrupted(vec!["queued".into()]));
    // @step Then cold reopen retains the partial content and user interruption reason
    let archived = originals(session.id)?;
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0]["interruptionReason"], "user");
    assert!(serde_json::to_string(&archived[0])?.contains("partial response"));
    // @step And the emitted interruption preserves queued input and its typed reason
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
    // @step Given the codelet_rpc_types StreamChunk type
    // @step When I construct StreamChunk::interrupted with one queued input
    // @step And the impl block defines a `pub fn interrupted(queued_inputs: Vec<String>) -> Self` constructor returning `Self::Interrupted { queued_inputs }`
    let chunk = StreamChunk::interrupted(vec!["queued".into()]);
    // @step Then the enum declares a `Interrupted { queued_inputs: Vec<String> }` variant
    assert!(
        matches!(&chunk, StreamChunk::Interrupted { queued_inputs } if queued_inputs == &["queued"])
    );
    let encoded = codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(&chunk);
    assert_eq!(encoded["type"], "interrupted");
    assert_eq!(encoded["queuedInputs"], serde_json::json!(["queued"]));
    Ok(())
}
