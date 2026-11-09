//! Feature: spec/features/a-panic-in-one-session-never-stops-other-sessions-persistence.feature
//!
//! BUG-610: applying a self-compaction re-validates the Codex session's
//! whole saved history inside the manifest update. Saved thinking blocks
//! replay as reasoning with no provider id, and the OpenAI Responses
//! conversion panicked on them ("An OpenAI-generated ID is required when
//! using OpenAI reasoning items") while the session store lock was held.
//! Reproduced first on a copy of the real session's data; this test builds
//! the same shape from a synthetic Codex session.
#[path = "fixtures/bug278.rs"]
#[allow(dead_code)]
mod fixture;
use codelet_core::persistence::*;
use fixture::*;
use rig::message::{AssistantContent, Message, Reasoning};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
#[serial_test::serial]
fn a_codex_session_whose_saved_history_holds_reasoning_without_an_id_compacts() -> TestResult {
    // @step Given a Codex session whose saved history holds an assistant thinking block with no provider id
    let mut fixture = fixture()?;
    append_message_with_metadata(
        &mut fixture.manifest,
        "assistant",
        "",
        HashMap::from([(
            "message".into(),
            json!({"role":"assistant","content":[
                {"type":"thinking","thinking":"**Inspecting root file**"},
                {"type":"text","text":"The root file lists the workspace members."}]}),
        )]),
    )?;
    let request = wire(&fixture.session.messages)?;
    history(&fixture.session)?.commit(
        &request,
        &[
            json!({"type":"reasoning","id":"rs_bug610","summary":[
                {"type":"summary_text","text":"**Inspecting root file**"}]}),
            json!({"type":"message","role":"assistant","content":[
                {"type":"output_text","text":"The root file lists the workspace members."}]}),
        ],
    )?;
    fixture.session.messages.push(Message::Assistant {
        id: None,
        content: rig::OneOrMany::many(vec![
            AssistantContent::Reasoning(
                Reasoning::new("**Inspecting root file**").with_id("rs_bug610".into()),
            ),
            AssistantContent::text("The root file lists the workspace members."),
        ])?,
    });
    let before = load_session(fixture.manifest.id)?;

    // @step When the session applies a self-compaction of its history
    let source = fixture
        .session
        .capture_durable_compaction(fixture.manifest.id)?;
    let dag = format!(
        "<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Work\">original task; the root file lists the workspace members</dag-node>",
        source.prefix.len() - 1
    );
    let session = &mut fixture.session;
    let applied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.commit_selected_compaction(source, Uuid::new_v4(), &dag, |_| Ok(()))
    }))
    .map_err(|_| "applying the compaction panicked")?;

    // @step Then the compaction commits a new compaction generation
    let committed = applied?;
    let boundary = committed
        .compaction_commit
        .as_ref()
        .ok_or("no committed compaction generation")?;
    assert_eq!(boundary.identity.generation, 1);
    assert!(committed.revision > before.revision);

    // @step And the session's manifest can still be written
    let mut current = load_session(fixture.manifest.id)?;
    append_message(&mut current, "user", "continue")?;
    assert_eq!(
        load_session(fixture.manifest.id)?.revision,
        current.revision
    );
    Ok(())
}
