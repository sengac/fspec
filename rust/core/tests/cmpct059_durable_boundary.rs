//! Feature: spec/features/durable-versioned-dag-boundary.feature

#[path = "support/compaction_boundary.rs"]
mod support;

use codelet_core::persistence::compaction_commit::*;
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use std::process::Command;
use support::candidate;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn scenario_cold_resume_restores_the_exact_committed_dag_and_suffix() -> TestResult {
    if let Ok(data) = std::env::var("CMPCT059_COLD_STORE") {
        codelet_common::set_data_directory(data.into())?;
        reset_stores_for_tests();
        let session_id = std::env::var("CMPCT059_COLD_SESSION")?.parse()?;
        let root = std::path::PathBuf::from(std::env::var("CMPCT059_COLD_STORE")?);
        let expected: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("expected.json"))?)?;
        let restored = load_session(session_id)?;
        assert_eq!(serde_json::to_value(&restored)?, expected["manifest"]);
        let boundary = restored.compaction_commit.as_ref().ok_or("boundary")?;
        assert_eq!(boundary.identity.generation, 1);
        assert_eq!(boundary.checkpoint.payload["lineage"], "explicit-full-DAG");
        assert_eq!(boundary.input_tokens, 23);
        assert_eq!(restored.token_usage.current_context_tokens, 23);
        let replay = get_session_message_envelopes(session_id)?;
        assert_eq!(replay.len(), 3);
        assert!(!replay[0].contains("covered original"));
        let summary: serde_json::Value = serde_json::from_str(&replay[0])?;
        assert_eq!(
            summary["message"]["content"],
            serde_json::json!([{"type":"text","text":format!(
                "[Previous conversation summary]\n\n{}",
                expected["dag"].as_str().ok_or("expected DAG text")?
            )}])
        );
        assert_eq!(summary["_committedCompaction"]["generation"], 1);
        assert_eq!(summary["_providerCheckpoint"], boundary.checkpoint.payload);
        let suffix = replay[1..]
            .iter()
            .map(|envelope| serde_json::from_str::<serde_json::Value>(envelope))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(serde_json::to_value(suffix)?, expected["suffix"]);
        assert_eq!(
            serde_json::to_value(std::fs::read(root.join("messages/messages.jsonl"))?)?,
            expected["original_records"]
        );
        return Ok(());
    }
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    // @step Given an admitted DAG and structured work persisted after its source snapshot
    let mut manifest = create_session_with_provider("cold process", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let candidate = candidate(&manifest)?;
    let exact_dag = candidate.dag.content.clone();
    append_message_with_metadata(
        &mut manifest,
        "assistant",
        "tool invocation",
        [(
            "message".into(),
            serde_json::json!({"role":"assistant","content":[
            {"type":"text","text":"checking 雪"},
            {"type":"tool_use","id":"call-1","name":"read_file",
             "input":{"path":"src/lib.rs","offset":7}}]}),
        )]
        .into_iter()
        .collect(),
    )?;
    append_message_with_metadata(
        &mut manifest,
        "user",
        "tool output",
        [(
            "message".into(),
            serde_json::json!({"role":"user","content":[
            {"type":"tool_result","tool_use_id":"call-1","is_error":false,
             "content":"tool output\n雪"},
            {"type":"text","text":"retained suffix trailer"}]}),
        )]
        .into_iter()
        .collect(),
    )?;
    let original_records = std::fs::read(directory.path().join("messages/messages.jsonl"))?;
    let mut expected_suffix = get_session_message_envelopes(manifest.id)?[1..]
        .iter()
        .map(|envelope| serde_json::from_str::<serde_json::Value>(envelope))
        .collect::<Result<Vec<_>, _>>()?;
    let committed = commit_compaction(candidate, "model-a", |_, _| Ok(()))?;
    let published = publish_compaction(&committed, |_| Ok(()))?;
    for envelope in &mut expected_suffix {
        envelope["_sessionReplay"]["revision"] = published.revision.into();
    }
    std::fs::write(
        directory.path().join("expected.json"),
        serde_json::to_vec(&serde_json::json!({"manifest":published,"dag":exact_dag,
            "suffix":expected_suffix,"original_records":original_records}))?,
    )?;
    // @step When a fresh process opens the promoted session boundary
    let status = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "scenario_cold_resume_restores_the_exact_committed_dag_and_suffix",
            "--nocapture",
        ])
        .env("CMPCT059_COLD_STORE", directory.path())
        .env("CMPCT059_COLD_SESSION", manifest.id.to_string())
        .status()?;
    // @step Then it restores the same generation checkpoint tokens exact DAG and complete structured suffix payload
    // @step And every original record remains unchanged and covered originals are not replayed
    assert!(
        status.success(),
        "fresh-process restore must verify all committed fields"
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_incompatible_durable_coverage_blocks_cold_replay() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("bad lineage", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let committed = commit_compaction(candidate(&manifest)?, "model-a", |_, _| Ok(()))?;
    publish_compaction(&committed, |_| Ok(()))?;
    // @step Given a promoted boundary whose covered message identity is incompatible
    update_session_manifest(manifest.id, |current| {
        current.messages[0].message_id = uuid::Uuid::new_v4();
        Ok(())
    })?;
    reset_stores_for_tests();
    // @step When a cold reader loads the model replay
    let result = get_session_message_envelopes(manifest.id);
    // @step Then replay fails explicitly without raw original fallback
    assert!(
        result.is_err(),
        "incompatible covered prefix must not replay a guessed summary: {result:?}"
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_failure_before_promotion_preserves_the_old_generation() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    // @step Given an existing committed generation and a candidate whose promotion cannot complete
    let mut manifest =
        create_session_with_provider("promotion failure", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let initial = commit_compaction(candidate(&manifest)?, "model-a", |_, _| Ok(()))?;
    let current = publish_compaction(&initial, |_| Ok(()))?;
    let next = candidate(&current)?;
    let before = std::fs::read(
        directory
            .path()
            .join(format!("sessions/{}.json", manifest.id)),
    )?;
    let sessions = directory.path().join("sessions");
    let original_permissions = std::fs::metadata(&sessions)?.permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(&sessions, readonly)?;
    // @step When persistence rejects the candidate before promotion
    let result = commit_compaction(next, "model-a", |_, _| Ok(()));
    std::fs::set_permissions(&sessions, original_permissions)?;
    assert!(
        matches!(result, Err(CompactionCommitError::Persistence(_))),
        "stage write must fail before rename: {result:?}"
    );
    reset_stores_for_tests();
    // @step Then cold resume still restores the old generation without a successful acknowledgement
    assert_eq!(
        load_session(manifest.id)?
            .compaction_commit
            .ok_or("boundary")?
            .identity
            .generation,
        1
    );
    assert_eq!(
        std::fs::read(
            directory
                .path()
                .join(format!("sessions/{}.json", manifest.id))
        )?,
        before
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_failed_publication_retains_the_promoted_identity_until_reconciliation() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("publication failure", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    // @step Given a candidate durably promoted as a new generation
    let committed = commit_compaction(candidate(&manifest)?, "model-a", |_, _| Ok(()))?;
    let identity = committed
        .compaction_commit
        .as_ref()
        .ok_or("boundary")?
        .identity
        .clone();
    // @step When its completion publication fails
    let result = publish_compaction(&committed, |_| Err("subscriber disconnected".into()));
    // @step Then the failure reports the new durable commit identity and inference remains blocked
    assert!(
        matches!(result, Err(CompactionCommitError::Publication { identity: actual, .. }) if actual == identity)
    );
    reset_stores_for_tests();
    let restored = load_session(manifest.id)?;
    assert!(
        restored
            .compaction_commit
            .as_ref()
            .ok_or("boundary")?
            .publication_pending
    );
    assert!(
        codelet_core::persistence::compaction_boundary::capture_compaction_source(
            &restored, "model-a", "dag-v1"
        )
        .is_err()
    );
    let mut wrong = identity.clone();
    wrong.operation_id = uuid::Uuid::new_v4();
    assert!(reconcile_compaction(&wrong).is_err());
    // @step And reconciling that exact identity permits inference without a second promotion
    let reconciled = reconcile_compaction(&identity)?;
    let boundary = reconciled.compaction_commit.as_ref().ok_or("boundary")?;
    assert_eq!(boundary.identity, identity);
    assert!(!boundary.publication_pending);
    Ok(())
}
