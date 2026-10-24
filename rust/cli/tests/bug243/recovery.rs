//! Feature: spec/features/provider-state-recovery.feature

use super::fixtures::*;
use codelet_tools::session_search::originals::OriginalsError;

#[path = "../../../providers/tests/fixtures/prov148.rs"]
#[allow(dead_code)]
mod transport;

pub(super) async fn send_next(cold: &codelet_cli::session::Session, secret: &str) -> TestResult {
    use futures::StreamExt;
    use rig::completion::{CompletionModel, CompletionRequestBuilder};
    let server = transport::Server::start(
        vec![transport::terminal("completed", Some(true), None)],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_history(history(cold)?)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(4_000_000),
            output_reservation: Some(1000),
        });
    let request = CompletionRequestBuilder::new(model.clone(), "next")
        .messages(vec![
            rig::message::Message::user("seed"),
            rig::message::Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(
                    rig::message::AssistantContent::tool_call_with_call_id(
                        "item-exact",
                        "call-exact".into(),
                        "inspect",
                        json!({}),
                    ),
                ),
            },
            rig::message::Message::tool_result_with_call_id(
                "item-exact",
                Some("call-exact".into()),
                "authentic output",
            ),
        ])
        .build();
    let mut response = model.stream(request).await?;
    while let Some(event) = response.next().await {
        event?;
    }
    let requests = server.requests.lock().map_err(|_| "requests poisoned")?;
    let input = requests[0]["input"].as_array().ok_or("input")?;
    for item in native(secret).as_array().ok_or("native")? {
        assert!(input.contains(item), "native item not retained");
    }
    assert_eq!(requests.len(), 1);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn cold_continuation_preserves_native_reasoning_and_tool_identity() -> TestResult {
    use futures::StreamExt;
    use rig::completion::{CompletionModel, CompletionRequestBuilder};
    // @step Given real persisted native reasoning and paired tool invocation outcomes
    let directory = setup()?;
    let (manifest, _) = dialogue(directory.path())?;
    let mut live = session();
    live.bind_provider_persistence(manifest.id)?;
    publish(&history(&live)?, "encrypted-exact")?;
    let expected = serde_json::to_value(history(&live)?.snapshot()?)?;
    // @step When a cold provider restores and sends its next physical request
    reset_stores_for_tests();
    let mut cold = session();
    cold.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    cold.bind_provider_persistence(manifest.id)?;
    let restored = history(&cold)?;
    let server = transport::Server::start(
        vec![transport::terminal("completed", Some(true), None)],
        false,
    )
    .await?;
    let model = server.models()?.0.with_history(restored.clone());
    let request = CompletionRequestBuilder::new(model.clone(), "next")
        .messages(vec![
            rig::message::Message::user("seed"),
            rig::message::Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(
                    rig::message::AssistantContent::tool_call_with_call_id(
                        "item-exact",
                        "call-exact".into(),
                        "inspect",
                        json!({}),
                    ),
                ),
            },
            rig::message::Message::tool_result_with_call_id(
                "item-exact",
                Some("call-exact".into()),
                "authentic output",
            ),
        ])
        .build();
    let mut response = model.stream(request).await?;
    while let Some(event) = response.next().await {
        event?;
    }
    // @step Then model protocol generation reasoning and authentic tool identity survive
    assert_eq!(expected["model"], "shared-model");
    assert_eq!(expected["protocol"], "full-current-dag");
    assert_eq!(expected["generation"], 0);
    let requests = server.requests.lock().map_err(|_| "requests poisoned")?;
    let input = requests[0]["input"].as_array().ok_or("input")?;
    for item in native("encrypted-exact").as_array().ok_or("native")? {
        assert!(input.contains(item), "{input:?}");
    }
    // @step And no lineage reset or alternate request is used
    assert_eq!(requests.len(), 1);
    assert_eq!(load_session(manifest.id)?.messages.len(), 3);
    let mut again = session();
    again.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    assert_eq!(restored.snapshot()?, history(&again)?.snapshot()?);
    Ok(())
}

#[test]
#[serial_test::serial]
fn reject_corrupt_originals_and_incompatible_state_without_fallback() -> TestResult {
    // @step Given malformed genuine dialogue oversized dialogue and a mismatched state anchor
    let directory = setup()?;
    let mut malformed = create_session("malformed", directory.path())?;
    append_message(&mut malformed, "assistant", "not an original")?;
    let mut oversized = create_session("oversized", directory.path())?;
    append_message_with_metadata(
        &mut oversized,
        "user",
        "",
        serde_json::from_value(
            json!({"uuid":Uuid::new_v4(),"message":{"role":"user","content":"large".repeat(20000)}}),
        )?,
    )?;
    let (manifest, _) = dialogue(directory.path())?;
    let mut live = session();
    live.bind_provider_persistence(manifest.id)?;
    publish(&history(&live)?, "exact")?;
    let path = state_path(directory.path(), manifest.id)?;
    let valid_state = std::fs::read(&path)?;
    let mut state: Value = serde_json::from_slice(&valid_state)?;
    state["checkpoints"][0]["prefixHash"] = json!("wrong-prefix");
    std::fs::write(&path, serde_json::to_vec(&state)?)?;
    // @step When originals and provider replay are requested
    let malformed_result = originals::retrieve_originals(request(malformed.id, None));
    let oversized_result = originals::retrieve_originals(request(oversized.id, None));
    let replay = get_session_message_envelopes(manifest.id);
    // @step Then genuine dialogue returns its exact typed missing or oversized failure
    assert!(matches!(
        malformed_result,
        Err(OriginalsError::MissingEnvelope { .. })
    ));
    assert!(matches!(
        oversized_result,
        Err(OriginalsError::OversizedEnvelope { turn_index: 0, .. })
    ));
    // @step And incompatible state prevents replay without legacy lineage fallback
    assert!(replay.is_err());
    std::fs::write(&path, &valid_state)?;
    let reference = load_session(manifest.id)?
        .provider_state
        .ok_or("state reference")?;
    let incompatible = serde_json::to_vec(&state)?;
    let incompatible_id = compute_hash(&incompatible);
    std::fs::write(
        path.with_file_name(format!("{incompatible_id}.json")),
        incompatible,
    )?;
    update_session_manifest(manifest.id, |current| {
        current.provider_state = Some(ProviderStateRef {
            version: 1,
            state_id: incompatible_id,
        });
        Ok(())
    })?;
    assert!(get_session_message_envelopes(manifest.id).is_err());
    update_session_manifest(manifest.id, |current| {
        current.provider_state = Some(reference);
        Ok(())
    })?;
    std::fs::remove_file(&path)?;
    assert!(get_session_message_envelopes(manifest.id).is_err());
    let (mixed, _) = legacy(directory.path())?;
    let mut mixed_live = session();
    mixed_live.bind_provider_persistence(mixed.id)?;
    publish(&history(&mixed_live)?, "new state beyond legacy checkpoint")?;
    std::fs::remove_file(state_path(directory.path(), mixed.id)?)?;
    assert!(get_session_message_envelopes(mixed.id).is_err());
    Ok(())
}

#[test]
#[serial_test::serial]
fn preserve_prior_state_across_checkpoint_publication_failures() -> TestResult {
    // @step Given a committed checkpoint and an unavailable checkpoint promotion destination
    let directory = setup()?;
    let (manifest, _) = dialogue(directory.path())?;
    let mut live = session();
    live.bind_provider_persistence(manifest.id)?;
    publish(&history(&live)?, "prior")?;
    let path = state_path(directory.path(), manifest.id)?;
    let previous = std::fs::read(&path)?;
    let backup = path.with_extension("backup");
    std::fs::rename(&path, &backup)?;
    std::fs::create_dir(&path)?;
    // @step When a new checkpoint publication fails and is retried after recovery
    let failure = publish(&history(&live)?, "next");
    std::fs::remove_dir(&path)?;
    std::fs::rename(&backup, &path)?;
    // @step Then the failure is explicit and the previous committed state remains recoverable
    assert!(failure.is_err());
    assert_eq!(std::fs::read(&path)?, previous);
    let mut cold = session();
    cold.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    assert_eq!(
        serde_json::to_value(history(&cold)?.snapshot()?)?["items"][1]["encrypted_content"],
        "prior"
    );
    cold.bind_provider_persistence(manifest.id)?;
    publish(&history(&cold)?, "next")?;
    // @step And retry selects the exact new native checkpoint without adding conversation records
    let mut retry = session();
    retry.restore_provider_state(&get_session_message_envelopes(manifest.id)?)?;
    assert_eq!(
        serde_json::to_value(history(&retry)?.snapshot()?)?["items"][1]["encrypted_content"],
        "next"
    );
    assert_eq!(load_session(manifest.id)?.messages.len(), 3);
    Ok(())
}
