//! Feature: spec/features/durable-checkpoint-envelope-preservation.feature

use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::session::{ExtensionError, ProviderSession};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use std::{process::Command, sync::Arc};

#[path = "support/compaction_boundary.rs"]
mod support;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn provider_checkpoints_survive_captured_prefix_promotion_and_cold_replay() -> TestResult {
    if let Ok(data) = std::env::var("CMPCT055_CHECKPOINT_STORE") {
        let data = std::path::PathBuf::from(data);
        codelet_common::set_data_directory(data.clone())?;
        let session_id = std::env::var("CMPCT055_CHECKPOINT_SESSION")?.parse()?;
        let manifest = load_session(session_id)?;
        let expected: Value =
            serde_json::from_slice(&std::fs::read(data.join("expected-originals.json"))?)?;
        let replay = get_session_message_envelopes(session_id)?
            .iter()
            .map(|envelope| serde_json::from_str::<Value>(envelope))
            .collect::<Result<Vec<_>, _>>()?;
        let boundary = manifest
            .compaction_commit
            .as_ref()
            .ok_or("missing boundary")?;
        // @step Then replay starts with the committed checkpoint followed by the exact ordered conversational suffix
        assert_eq!(boundary.identity.generation, 1);
        assert_eq!(boundary.covered_prefix.len(), 2);
        assert_eq!(boundary.verified_suffix.len(), 3);
        assert_eq!(replay.len(), 3);
        assert_eq!(replay[0]["_committedCompaction"]["generation"], 1);
        assert_eq!(
            replay[0]["_providerCheckpoint"],
            boundary.checkpoint.payload
        );
        assert_eq!(replay[0]["codexContinuation"]["generation"], 1);
        assert!(replay
            .iter()
            .skip(1)
            .all(|envelope| envelope.get("codexContinuation").is_none()));
        for (envelope, original) in replay
            .iter()
            .skip(1)
            .zip(expected.as_array().ok_or("original array")?.iter().skip(3))
        {
            assert_eq!(envelope["uuid"], original["id"]);
            assert_eq!(envelope["message"], original["metadata"]["message"]);
        }
        assert_eq!(replay[1]["message"]["content"][0]["id"], "item-exact");
        assert_eq!(replay[1]["message"]["content"][0]["call_id"], "call-exact");
        assert_eq!(
            replay[2]["message"]["content"][0]["tool_use_id"],
            "item-exact"
        );
        assert_eq!(replay[2]["message"]["content"][0]["call_id"], "call-exact");
        // @step And checkpoint positions remain non-conversational and every immutable original remains retrievable
        let page = originals::retrieve_originals(
            codelet_tools::session_search::originals::OriginalsRequest {
                session_id: session_id.to_string(),
                start_turn: Some(1),
                end_turn: Some(2),
                next_turn: None,
                limit: Some(2),
                max_bytes: Some(61440),
            },
        )?;
        assert_eq!(page.records.len(), 2);
        for (record, position) in page.records.iter().zip(1..=2) {
            assert_eq!(record.turn_index, position);
            assert_eq!(record.message_id, manifest.messages[position].message_id);
            assert_eq!(record.envelope["recordKind"], "provider_state");
            assert!(record.envelope.get("message").is_none());
        }
        assert_eq!(expected[1]["metadata"]["message"]["content"], json!([]));
        assert_eq!(
            serde_json::to_value(get_session_messages_full(&manifest)?)?,
            expected
        );
        for reference in &manifest.messages {
            assert!(get_message(reference.message_id)?.is_some());
        }
        return Ok(());
    }

    // @step Given genuine provider checkpoints in a captured prefix and an appended tool suffix
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("checkpoint preservation", directory.path(), "codex/model-a")?;
    append_message(&mut manifest, "user", "covered original")?;
    let session_id = manifest.id;
    let provider = ProviderSession::for_provider("codex");
    provider.bind_writers(|name| {
        let name = name.to_owned();
        Arc::new(move |mut metadata| {
            let mut current = load_session(session_id).map_err(ExtensionError::Persistence)?;
            metadata.insert("provider".into(), json!(name));
            metadata.insert("message".into(), json!({"role":"assistant","content":[]}));
            append_message_with_metadata(&mut current, "assistant", "", metadata)
                .map_err(ExtensionError::Persistence)?;
            Ok(())
        })
    })?;
    let history = &provider
        .extension::<CodexSession>("codex")
        .ok_or("Codex extension")?
        .history;
    let request = |text: &str| {
        json!({"model":"model-a","input":[{
            "role":"user","content":[{"type":"input_text","text":text}]
        }]})
    };
    history.commit(&request("captured checkpoint"), &[])?;
    manifest = load_session(session_id)?;
    let mut candidate = support::candidate(&manifest)?;
    candidate.dag = validate_dag_candidate(
        "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">checkpoint-preserving DAG</dag-node>",
        &DagValidationContext {
            persisted_turn_count: 2,
            required_coverage_end: Some(1),
            ..Default::default()
        },
    )?;
    candidate.checkpoint.payload =
        serde_json::to_value(provider.prepare_generation(1, "model-a")?)?;
    history.commit(&request("retained checkpoint"), &[])?;
    manifest = load_session(session_id)?;
    for (role, content) in [
        (
            "assistant",
            json!({"type":"tool_use","id":"item-exact","call_id":"call-exact","name":"read","input":{"path":"original.txt"}}),
        ),
        (
            "user",
            json!({"type":"tool_result","tool_use_id":"item-exact","call_id":"call-exact","content":"exact tool output"}),
        ),
    ] {
        append_message_with_metadata(
            &mut manifest,
            role,
            "tool suffix",
            [("message".into(), json!({"role":role,"content":[content]}))].into(),
        )?;
    }
    std::fs::write(
        directory.path().join("expected-originals.json"),
        serde_json::to_vec(&get_session_messages_full(&manifest)?)?,
    )?;
    // @step When the candidate commits and a fresh process reads the durable replay
    let committed = commit_compaction(candidate, "model-a", |current, _| {
        assert_eq!(current.messages.len(), 5);
        Ok(())
    })?;
    publish_compaction(&committed, |_| Ok(()))?;
    let status = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "provider_checkpoints_survive_captured_prefix_promotion_and_cold_replay",
            "--nocapture",
        ])
        .env("CMPCT055_CHECKPOINT_STORE", directory.path())
        .env("CMPCT055_CHECKPOINT_SESSION", session_id.to_string())
        .status()?;
    assert!(
        status.success(),
        "fresh-process checkpoint preservation failed"
    );
    Ok(())
}
