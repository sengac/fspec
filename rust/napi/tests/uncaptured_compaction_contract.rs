//! Feature: spec/features/runtime-compaction-no-substitute-success.feature
//! Public NAPI Rust entrypoints must agree with the canonical durable engine.
use codelet_cli::session::Session;
use codelet_providers::{ProviderManager, ProviderType};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
const DAG: &str =
    "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Uncaptured\">raw candidate</dag-node>";
#[test]
fn native_handler_does_not_acknowledge_uncaptured_dag_as_success() -> TestResult {
    let pending = Arc::new(Mutex::new(None));
    let busy = Arc::new(AtomicBool::new(true));
    let called = Arc::new(AtomicBool::new(false));
    let observed = called.clone();
    let handler = codelet_napi::inject_summary_handler::create_handler(
        pending.clone(),
        200_000,
        busy.clone(),
        Some(Arc::new(move |_| observed.store(true, Ordering::SeqCst))),
    );
    assert!(
        handler(uuid::Uuid::new_v4(), DAG.into()).is_err(),
        "uncaptured raw submission cannot report success"
    );
    assert!(pending.lock().map_err(|error| error.to_string())?.is_none());
    assert!(busy.load(Ordering::SeqCst));
    assert!(
        !called.load(Ordering::SeqCst),
        "completion side effects need durable commitment"
    );
    Ok(())
}
#[test]
fn native_pending_application_cannot_replace_memory_or_emit_completion() -> TestResult {
    let manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
    let mut session = Session::from_provider_manager(manager);
    session.messages.push(rig::message::Message::user(
        "committed original must survive",
    ));
    session.token_tracker.input_tokens = 42_000;
    let before = serde_json::to_value(&session.messages)?;
    let pending = Arc::new(Mutex::new(Some(DAG.into())));
    let emitted = Mutex::new(Vec::new());
    let applied = codelet_napi::inject_summary_handler::apply_pending_dag_and_emit(
        &mut session,
        &pending,
        42_000,
        &|event| {
            emitted
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(event);
        },
    );
    assert!(
        applied.is_none(),
        "legacy pending content cannot bypass durable promotion"
    );
    assert_eq!(serde_json::to_value(&session.messages)?, before);
    assert_eq!(session.token_tracker.input_tokens, 42_000);
    assert!(session.ensure_compaction_inference_ready().is_err());
    assert!(emitted
        .lock()
        .map_err(|error| error.to_string())?
        .is_empty());
    Ok(())
}
#[test]
fn canonical_uncaptured_handler_is_a_fail_closed_control() {
    let pending = Arc::new(Mutex::new(None));
    let busy = Arc::new(AtomicBool::new(true));
    let handler =
        codelet_agent_loop::inject_summary_handler::create_handler(pending, 200_000, busy, None);
    assert!(handler(uuid::Uuid::new_v4(), DAG.into()).is_err());
}
