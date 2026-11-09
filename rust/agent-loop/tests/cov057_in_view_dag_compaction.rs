#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/in-view-dag-compaction.feature
//!
//! COV-057: the in-view DAG flow end to end on production objects. The real
//! `execute_compaction` installs the compaction instruction and raises the
//! session's compaction flag; the production SessionSearch handler trims the
//! persisted history through Layer 0; the agent-built D0/D1/D2 DAG goes
//! through the registered inject_summary handler into the operation's
//! `TurnCompaction`, whose durable application leaves only system reminders
//! and the pinned DAG in context.

#[path = "cov057_in_view_dag_compaction/fixture.rs"]
mod fixture;

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_agent_loop::session_search_handler::create_handler;
use codelet_cli::session::system_reminders::is_system_reminder;
use codelet_core::compaction::{validate_dag_structure, DagDepth};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_tools::session_search::types::{SessionSearchAction, SessionSearchResult};
use fixture::{dag_from, text, Fixture, RetainedLoop, TestResult};
use rig::message::Message;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_agent_builds_dag_via_session_search_and_calls_inject_summary() -> TestResult {
    // @step Given a session where compaction system instruction has been injected
    let fixture = Fixture::new()?;
    let id = fixture.session.id;
    let mut inner = fixture.session.inner.lock().await;
    let operation = register_generate_compaction_handler(
        id,
        &inner,
        fixture.directory.path().to_owned(),
        fixture.session.clone(),
        None,
    );
    codelet_cli::interactive_helpers::execute_compaction(
        &mut inner,
        fixture.session.compaction_in_progress.clone(),
        Some("finish the entry point review"),
    )
    .await?;
    let instruction = text(inner.inference_messages().last().ok_or("instruction")?);
    assert!(instruction.contains("SessionSearch") && instruction.contains("inject_summary"));
    // @step And compaction_in_progress flag is true
    assert!(fixture
        .session
        .compaction_in_progress
        .load(Ordering::Acquire));
    // @step When the agent calls SessionSearch to retrieve session history
    let search = create_handler(
        fixture.directory.path().to_owned(),
        fixture.session.compaction_in_progress.clone(),
    );
    let show = SessionSearchAction::Show {
        session_id: None,
        user_only: None,
        max_turns: None,
        start_turn: None,
        end_turn: None,
    };
    let SessionSearchResult::Session { messages, .. } = search(show, id) else {
        return Err("SessionSearch show must return the current session".into());
    };
    // @step Then SessionSearch results should be trimmed via Layer 0 trimming
    assert_eq!(messages.len(), 4);
    let reference = &messages[2].content;
    assert!(reference.starts_with("[file: src/main.rs,"), "{reference}");
    assert!(reference.contains("500 lines") && reference.contains("use Read to retrieve"));
    assert!(!reference.contains("line 250 content"));
    // @step And the agent can build a hierarchical D0/D1/D2 DAG from trimmed results
    let dag = dag_from(&messages, reference);
    let depths: Vec<DagDepth> = validate_dag_structure(&dag)?
        .into_iter()
        .map(|node| node.depth)
        .collect();
    for depth in [DagDepth::D0, DagDepth::D1, DagDepth::D2] {
        assert!(depths.contains(&depth), "{depth:?} missing from {depths:?}");
    }
    // @step And the agent can call inject_summary with the DAG content
    let injected = codelet_tools::inject_summary::execute_inject_summary(id, dag)?;
    assert!(injected.injected_tokens > 0);
    // @step And after the submitted summary is applied the context contains only system reminders and the pinned DAG
    let applied = operation.apply(&mut inner, &fixture.session);
    codelet_tools::set_inject_summary_handler(id, None);
    codelet_tools::set_generate_compaction_handler(id, None);
    assert!(applied?);
    assert!(!inner.has_compaction_working_view());
    let (reminders, rest): (Vec<_>, Vec<_>) =
        inner.messages.iter().partition(|m| is_system_reminder(m));
    assert!(
        !reminders.is_empty(),
        "the environment reminder is retained"
    );
    assert_eq!(
        rest.len(),
        1,
        "only the pinned DAG accompanies the reminders"
    );
    let pinned = text(rest[0]);
    assert!(pinned.contains("[Previous conversation summary]"));
    assert!(pinned.contains("Entry point review") && pinned.contains("Read evidence"));
    assert!(!inner
        .messages
        .iter()
        .any(|m| text(m).contains("line 250 content")));
    let manifest = codelet_core::persistence::load_session(id)?;
    let summary = manifest.compaction.ok_or("durable summary")?.summary;
    assert!(summary.contains("Entry point review"));
    Ok(())
}

/// Scenario: session_compact NAPI binding queues the compactor agent instead of the in-view flow
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_session_compact_napi_binding_queues_the_compactor_agent() -> TestResult {
    // @step Given a session accessible via the session_compact NAPI binding
    // (the retired NAPI `session_compact` is served by the handle's `compact_session`)
    let fixture = Fixture::new()?;
    // BUG-448: stub credentials, never the machine's own (no request is sent).
    std::env::set_var("ANTHROPIC_API_KEY", "cov057-in-view-no-network");
    let manager = codelet_sessions::SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    let retained = Arc::new(RetainedLoop::default());
    manager.set_hooks(retained.clone());
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = &*manager;
    let id = handle.create_session_checked(None)?;
    let session = manager.get_session(&id.value)?;
    let history = fixture.session.inner.lock().await.messages.clone();
    session.inner.lock().await.messages = history.clone();
    // @step When session_compact is called
    let result = handle.compact_session(&id)?;
    // @step Then no in-view compaction working view is installed and ordinary inference is not blocked
    let inner = session.inner.lock().await;
    assert!(!inner.has_compaction_working_view());
    assert!(inner.ensure_compaction_inference_ready().is_ok());
    // @step And no batch LLM pipeline should be used
    // Only the start is acknowledged; the conversation is untouched until
    // the compactor agent's summary is committed by the agent loop.
    assert_eq!((result.compacted_tokens, result.turns_summarized), (0, 0));
    assert_eq!(text_of(&inner.messages), text_of(&history));
    let manifest = codelet_core::persistence::load_session(session.id)?;
    assert!(manifest.compaction.is_none(), "no summary was produced");
    // @step And one internal manual-compaction request is queued for the agent loop
    let queued = retained.take_queued();
    assert_eq!(queued.len(), 1, "exactly one queued request");
    assert!(
        queued[0].compaction_epoch.is_some(),
        "it is a manual-compaction request"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_inject_summary_leaves_the_flag_to_the_turn_end() -> TestResult {
    // @step Given a session with compaction_in_progress flag set to true
    let fixture = Fixture::new()?;
    let id = fixture.session.id;
    let mut inner = fixture.session.inner.lock().await;
    let operation = register_generate_compaction_handler(
        id,
        &inner,
        fixture.directory.path().to_owned(),
        fixture.session.clone(),
        None,
    );
    codelet_cli::interactive_helpers::execute_compaction(
        &mut inner,
        fixture.session.compaction_in_progress.clone(),
        Some("finish the entry point review"),
    )
    .await?;
    let flag = fixture.session.compaction_in_progress.clone();
    assert!(flag.load(Ordering::Acquire));
    let search = create_handler(fixture.directory.path().to_owned(), flag.clone());
    let show = || SessionSearchAction::Show {
        session_id: None,
        user_only: None,
        max_turns: None,
        start_turn: None,
        end_turn: None,
    };
    let SessionSearchResult::Session { messages, .. } = search(show(), id) else {
        return Err("SessionSearch show must return the current session".into());
    };
    let dag = dag_from(&messages, &messages[2].content);
    // @step When the agent calls inject_summary with DAG content
    codelet_tools::inject_summary::execute_inject_summary(id, dag)?;
    // @step Then the compaction_in_progress flag stays set until the turn that applies the summary ends
    assert!(
        flag.load(Ordering::Acquire),
        "the handler must not clear the flag"
    );
    // @step And SessionSearch calls keep returning Layer 0 trimmed content while the flag is set
    let SessionSearchResult::Session { messages, .. } = search(show(), id) else {
        return Err("SessionSearch show must return the current session".into());
    };
    assert!(messages[2].content.starts_with("[file: src/main.rs,"));
    assert!(!messages[2].content.contains("line 250 content"));
    // @step And the DAG is applied when the turn's compaction operation is applied
    let applied = operation.apply(&mut inner, &fixture.session);
    codelet_tools::set_inject_summary_handler(id, None);
    codelet_tools::set_generate_compaction_handler(id, None);
    assert!(applied?);
    let manifest = codelet_core::persistence::load_session(id)?;
    assert!(manifest
        .compaction
        .ok_or("durable summary")?
        .summary
        .contains("Entry point review"));
    Ok(())
}

fn text_of(messages: &[Message]) -> Vec<String> {
    messages.iter().map(text).collect()
}
