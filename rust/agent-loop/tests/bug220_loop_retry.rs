//! Feature: spec/features/loop-interruption-retry-admission.feature
#![cfg(feature = "test-support")]

#[path = "support/bug220.rs"]
mod support;

use codelet_agent_loop::{background_output::LoopDetectionWiring, FspecAgentHooks};
use codelet_rpc_types::StreamChunk;
use codelet_sessions::SessionManager;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use support::{detector_abort, fixture, interruptions, TestResult};

struct StreamHook;
impl Drop for StreamHook {
    fn drop(&mut self) {
        codelet_providers::stub_model::clear_looping_stream_hook();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_genuine_loop_recovery_remains_typed_and_bounded() -> TestResult {
    // @step Given a deterministic provider repeatedly producing a genuine sustained loop
    let (directory, _, _) = fixture().await?;
    let polls = Arc::new(AtomicUsize::new(0));
    codelet_providers::stub_model::set_looping_stream_hook(
        vec!["repeated ".into(); 300],
        polls.clone(),
    );
    let _hook = StreamHook;
    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    let id = manager
        .create_session("stub/canned", &directory.path().to_string_lossy())
        .await?;
    let session = manager.get_session(&id)?;
    let mut chunks = manager.chunks_tx().subscribe();
    // @step When the production agent loop exhausts its configured recovery allowance
    session.send_input("explain the architecture".into(), None)?;
    let attempts = LoopDetectionWiring::default().max_retries as usize + 1;
    tokio::time::timeout(Duration::from_secs(60), async {
        let mut done = 0;
        while done < attempts {
            let (_, chunk) = chunks.recv().await?;
            if matches!(chunk, StreamChunk::Done) {
                done += 1;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    // @step Then every interruption is detector initiated and the corrective retries stop at the limit
    let events = interruptions(&session);
    assert_eq!(events.len(), attempts);
    assert!(events.iter().all(|event| event["reason"] == "loopDetector"));
    let polled = polls.load(Ordering::Acquire);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(polls.load(Ordering::Acquire), polled);
    assert!(polled < 300 * attempts);
    assert!(!session.has_pending_loop_abort_note());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_user_stop_invalidates_staged_loop_recovery_and_stale_epochs() -> TestResult {
    // @step Given a genuine detector abort with a staged corrective retry
    let (_directory, session, output) = fixture().await?;
    detector_abort(&output);
    assert!(session.has_pending_loop_abort_note());
    let epoch = session.compaction_epoch();
    let polls = Arc::new(AtomicUsize::new(0));
    codelet_providers::stub_model::set_looping_stream_hook(
        vec!["unexpected".into()],
        polls.clone(),
    );
    let _hook = StreamHook;
    // @step When a user stop arrives before that retry is admitted
    session.interrupt();
    // @step Then the pending recovery is discarded and no new provider request is made
    assert!(!session.admit_loop_retry(epoch));
    assert!(!session.has_pending_loop_abort_note());
    assert!(session.compaction_cancelled(epoch));
    assert!(session.is_interrupted.load(Ordering::Acquire));
    assert_eq!(polls.load(Ordering::Acquire), 0);
    session.reset_interrupt();
    session.interrupt_loop_detector("retry".into());
    let admitted_epoch = session.compaction_epoch();
    assert!(session.admit_loop_retry(admitted_epoch));
    session.interrupt();
    assert!(!session.admit_loop_retry(admitted_epoch));
    let agent =
        rig::agent::AgentBuilder::new(codelet_providers::stub_model::StubModel::new()).build();
    codelet_cli::interactive::run_agent_stream(
        codelet_core::RigAgent::with_default_depth(agent),
        "Continue",
        &mut *session.inner.lock().await,
        session.is_interrupted.clone(),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        session.interrupt_notify.clone(),
        &*output,
        session.id,
    )
    .await?;
    assert_eq!(polls.load(Ordering::Acquire), 0);
    Ok(())
}
