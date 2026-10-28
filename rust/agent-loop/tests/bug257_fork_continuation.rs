#![doc = "Feature: spec/features/fork-native-continuation.feature"]

#[path = "support/bug257.rs"]
pub mod support;

use codelet_core::persistence::messages::durability::observe;
use support::{fixtures::*, inventory, next_request, Environment};

fn parent(environment: &Environment) -> Result<SessionManifest, Box<dyn std::error::Error>> {
    let (manifest, _) = dialogue(environment.directory.path())?;
    let mut live = session();
    live.bind_provider_persistence(manifest.id)?;
    publish(&history(&live)?, "fork-native-exact")?;
    Ok(load_session(manifest.id)?)
}

fn evidence(
    environment: &Environment,
    parent: &SessionManifest,
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    Ok((
        std::fs::read(
            environment
                .directory
                .path()
                .join(format!("sessions/{}.json", parent.id)),
        )?,
        std::fs::read(state_path(environment.directory.path(), parent.id)?)?,
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn final_completed_codex_fork_cold_continues_its_exact_prefix() -> TestResult {
    // @step Given completed tool dialogue with a separate native continuation checkpoint
    let environment = Environment::new()?;
    let parent = parent(&environment)?;
    let before = evidence(&environment, &parent)?;
    // @step When its final message is forked and the child cold sends a physical next request
    let child = fork_session(&parent, parent.messages.len() - 1, "child")?;
    let request = next_request(&child).await?;
    // @step Then native reasoning and tool identities survive without changing parent evidence
    let input = request["input"]
        .as_array()
        .ok_or("physical request input")?;
    for item in native("fork-native-exact")
        .as_array()
        .ok_or("native fixture")?
    {
        assert!(input.contains(item), "missing native item: {item}");
    }
    assert_eq!(request["model"], "shared-model");
    assert!(request.to_string().contains("child-next-unique"));
    assert_ne!(child.provider_state, parent.provider_state);
    assert_eq!(
        child
            .messages
            .iter()
            .map(|message| message.message_id)
            .collect::<Vec<_>>(),
        parent
            .messages
            .iter()
            .map(|message| message.message_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(evidence(&environment, &parent)?, before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn retained_exact_boundary_excludes_later_parent_work() -> TestResult {
    // @step Given a completed checkpoint followed by later parent conversation
    let environment = Environment::new()?;
    let mut parent = parent(&environment)?;
    let boundary = parent.messages.len() - 1;
    append_message_with_metadata(
        &mut parent,
        "user",
        "",
        serde_json::from_value(
            json!({"message":{"role":"user","content":[{"type":"text","text":"later-parent-only"}]}}),
        )?,
    )?;
    let before = evidence(&environment, &parent)?;
    // @step When the checkpoint prefix is forked and cold continued physically
    let child = fork_session(&parent, boundary, "prefix child")?;
    let request = next_request(&child).await?;
    // @step Then the child request excludes all later parent content and originals remain separate
    assert!(request.to_string().contains("fork-native-exact"));
    assert!(!request.to_string().contains("later-parent-only"));
    assert_eq!(child.messages.len(), boundary + 1);
    let originals = originals::retrieve_originals(request_originals(child.id))?;
    assert!(!serde_json::to_string(&originals)?.contains("codexContinuation"));
    assert_eq!(evidence(&environment, &parent)?, before);
    Ok(())
}

fn request_originals(session: Uuid) -> codelet_tools::session_search::originals::OriginalsRequest {
    let mut request = request(session, None);
    request.limit = Some(10);
    request
}

#[test]
#[serial_test::serial]
fn unsupported_older_boundaries_fail_before_publishing_a_child() -> TestResult {
    // @step Given a Codex session whose only checkpoint is later than the selected prefix
    let environment = Environment::new()?;
    let parent = parent(&environment)?;
    let before = evidence(&environment, &parent)?;
    let sessions = inventory()?;
    // @step When that older boundary is forked
    let result = fork_session(&parent, 0, "unsupported child");
    // @step Then an explicit unsupported boundary error leaves parent and session inventory unchanged
    assert!(result.is_err(), "fork accepted unsupported native boundary");
    assert!(result
        .err()
        .ok_or("missing rejection")?
        .contains("boundary"));
    assert_eq!(inventory()?, sessions);
    assert_eq!(evidence(&environment, &parent)?, before);
    let (missing, _) = dialogue(environment.directory.path())?;
    let sessions = inventory()?;
    assert!(fork_session(&missing, missing.messages.len() - 1, "missing checkpoint").is_err());
    assert_eq!(inventory()?, sessions);
    Ok(())
}

#[test]
#[serial_test::serial]
fn corrupt_provider_state_prevents_fork_publication() -> TestResult {
    // @step Given a parent with a corrupt referenced native checkpoint
    let environment = Environment::new()?;
    let parent = parent(&environment)?;
    std::fs::write(
        state_path(environment.directory.path(), parent.id)?,
        b"corrupt native state",
    )?;
    let before = evidence(&environment, &parent)?;
    let sessions = inventory()?;
    // @step When its completed prefix is forked
    let result = fork_session(&parent, parent.messages.len() - 1, "corrupt child");
    // @step Then fork fails explicitly without publishing a child or changing parent evidence
    assert!(result.is_err(), "fork ignored corrupt referenced state");
    assert_eq!(inventory()?, sessions);
    assert_eq!(evidence(&environment, &parent)?, before);
    Ok(())
}

#[test]
#[serial_test::serial]
fn legacy_checkpoints_and_compaction_boundaries_remain_safe() -> TestResult {
    // @step Given a legacy checkpoint session and an existing compacted conversation
    let environment = Environment::new()?;
    let (legacy, _) = legacy(environment.directory.path())?;
    let mut compacted = parent(&environment)?;
    set_compaction_state(&mut compacted, "existing summary".into(), 2)?;
    // @step When a final legacy checkpoint and a forbidden pre-compaction boundary are forked
    let child = fork_session(&legacy, legacy.messages.len() - 1, "legacy child")?;
    let rejected = fork_session(&compacted, 0, "before boundary");
    let unsupported_compacted = fork_session(
        &compacted,
        compacted.messages.len() - 1,
        "native compacted boundary",
    );
    let mut cold = session();
    cold.restore_provider_state(&get_session_message_envelopes(child.id)?)?;
    // @step Then legacy continuation restores and the forbidden boundary remains rejected
    assert!(history(&cold)?.snapshot()?.is_some());
    assert!(rejected.is_err());
    assert!(unsupported_compacted.is_err());
    assert_eq!(
        load_session(legacy.id)?.provider_state,
        legacy.provider_state
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn failed_child_checkpoint_publication_preserves_parent_ownership() -> TestResult {
    // @step Given a completed Codex parent with durable native state
    let environment = Environment::new()?;
    let parent = parent(&environment)?;
    let before = evidence(&environment, &parent)?;
    let sessions = inventory()?;
    // @step When child checkpoint synchronization fails during fork
    let (result, _) = observe(Some("provider_state.sync"), || {
        fork_session(&parent, parent.messages.len() - 1, "failed child")
    });
    // @step Then no child manifest is acknowledged and parent continuation remains recoverable
    assert!(result.is_err(), "fork skipped child checkpoint durability");
    assert_eq!(inventory()?, sessions);
    assert_eq!(evidence(&environment, &parent)?, before);
    let mut cold = session();
    cold.restore_provider_state(&get_session_message_envelopes(parent.id)?)?;
    assert!(history(&cold)?.snapshot()?.is_some());
    Ok(())
}
