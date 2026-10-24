//! Feature: spec/features/typed-durable-summary-identity.feature

#[path = "cmpct060/support.rs"]
mod support;

use codelet_core::persistence::*;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::StreamChunk;
use serde_json::json;
use support::{cold_resume, envelope, manager, next_anthropic_wire, next_wire, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn marker_wrapped_durable_summary_survives_cold_restore() -> TestResult {
    if let Some(root) = std::env::var_os("CMPCT060_COLD_ROOT") {
        codelet_common::set_data_directory(root.clone().into())?;
        std::env::set_var("FSPEC_USER_DIR", &root);
        let session_id = codelet_rpc_types::SessionId::new(std::env::var("CMPCT060_SESSION")?);
        let baseline = std::env::var("CMPCT060_BASELINE")?.parse()?;
        let manager = cold_resume(&session_id)?;
        let wire = next_wire(&manager, &session_id, baseline).await?;
        std::fs::write(
            std::path::Path::new(&root).join("cold060.json"),
            serde_json::to_vec(&wire)?,
        )?;
        return Ok(());
    }
    // @step Given a persisted DAG summary containing system reminder markers
    let (directory, manager, session_id, baseline) = manager().await?;
    let summary =
        "<system-reminder>\n<!-- type:compaction-dag -->\nunique durable DAG\n</system-reminder>";
    let mut manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    set_compaction_state(&mut manifest, summary.into(), 0)?;
    drop(manager);
    // @step When cold restore prepares the next model history
    reset_stores_for_tests();
    let mut child = tokio::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "marker_wrapped_durable_summary_survives_cold_restore",
            "--nocapture",
        ])
        .env("CMPCT060_COLD_ROOT", directory.path())
        .env("CMPCT060_SESSION", &session_id.value)
        .env("CMPCT060_BASELINE", baseline.to_string())
        .kill_on_drop(true)
        .spawn()?;
    let status = tokio::time::timeout(std::time::Duration::from_secs(30), child.wait()).await??;
    assert!(status.success(), "cold process restore failed: {status}");
    let wire: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("cold060.json"))?)?;
    // @step Then the exact durable summary occurs once in model history
    let encoded = wire.to_string();
    assert_eq!(encoded.matches("unique durable DAG").count(), 1, "{wire}");
    assert!(encoded.contains(
        &serde_json::to_string(summary)?
            .trim_matches('"')
            .to_string()
    ));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn quoted_and_split_markers_remain_legitimate_user_content() -> TestResult {
    // @step Given string and split text user envelopes quoting reminder markers
    let (_directory, manager, session_id, baseline) = manager().await?;
    let envelopes = vec![
        envelope(json!(
            "quoted <system-reminder> <!-- type:user-quote --> string"
        )),
        envelope(json!([
            {"type": "text", "text": "split <system-reminder>"},
            {"type": "text", "text": " <!-- type:user-quote --> retained"}
        ])),
    ]
    .into_iter()
    .map(|value| value.to_string())
    .collect();
    // @step When their persisted history is restored
    manager.restore_session_messages(&session_id, envelopes)?;
    let wire = next_wire(&manager, &session_id, baseline).await?;
    // @step Then every quoted marker and user text remains in model history
    let encoded = wire.to_string();
    assert!(encoded.contains("quoted <system-reminder>"), "{wire}");
    assert!(encoded.contains("split <system-reminder>"), "{wire}");
    assert!(encoded.contains("retained"), "{wire}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn reminder_text_cannot_discard_a_mixed_tool_result_envelope() -> TestResult {
    // @step Given a persisted user envelope with quoted reminder text and a tool result
    let (_directory, manager, session_id, baseline) = manager().await?;
    let mut output = manager.chunks_tx().subscribe();
    let envelope = envelope(json!([
        {"type": "text", "text": "quoted <system-reminder> <!-- type:quote -->"},
        {"type": "tool_result", "tool_use_id": "call_saved", "content": "saved result"}
    ]));
    // @step When its history is restored
    manager.restore_session_messages(
        &session_id,
        vec![
            json!({"message":{"role":"assistant","content":[
                {"type":"tool_use","id":"call_saved","name":"lookup","input":{}}
            ]}})
            .to_string(),
            envelope.to_string(),
        ],
    )?;
    let wire = next_anthropic_wire(&manager, &session_id, baseline).await?;
    // @step Then its text remains in model history and its tool result remains in UI replay
    assert!(
        wire.to_string().contains("quoted <system-reminder>"),
        "{wire}"
    );
    let mut found = false;
    while let Ok((_, chunk)) = output.try_recv() {
        if let StreamChunk::ToolResult {
            tool_result: result,
            ..
        } = chunk
        {
            found |= result.tool_call_id == "call_saved" && result.content == "saved result";
        }
    }
    assert!(found, "mixed envelope lost its saved tool result");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn only_explicitly_replaceable_runtime_reminders_are_omitted() -> TestResult {
    // @step Given a version one replaceable runtime reminder envelope
    let (_directory, manager, session_id, baseline) = manager().await?;
    let mut reminder = envelope(json!("replaceable runtime marker"));
    reminder["_runtimeReminder"] = json!({"version": 1, "replaceable": true});
    // @step When its history is restored
    manager.restore_session_messages(
        &session_id,
        vec![
            envelope(json!("durable before")).to_string(),
            reminder.to_string(),
            envelope(json!("durable after")).to_string(),
        ],
    )?;
    let wire = next_wire(&manager, &session_id, baseline).await?;
    // @step Then that reminder is omitted without removing surrounding durable content
    let encoded = wire.to_string();
    assert!(!encoded.contains("replaceable runtime marker"), "{wire}");
    assert!(encoded.contains("durable before") && encoded.contains("durable after"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn incompatible_reminder_identity_fails_before_restore_mutation() -> TestResult {
    // @step Given an envelope with ambiguous or incompatible typed reminder identity
    let (_directory, manager, session_id, baseline) = manager().await?;
    for identity in [json!(false), json!("true")] {
        let mut invalid = envelope(json!("invalid summary"));
        invalid["_compactionSummary"] = identity;
        // @step When its history is restored
        let result = manager.restore_session_messages(
            &session_id,
            vec![
                envelope(json!("must not install partial prefix")).to_string(),
                invalid.to_string(),
            ],
        );
        // @step Then restore fails explicitly without installing partial model history
        assert!(result.is_err(), "invalid identity must not restore");
        assert_eq!(
            manager
                .get_session(&session_id.value)?
                .inner
                .lock()
                .await
                .messages
                .len(),
            baseline
        );
    }
    for identity in [
        json!({"version": 2, "replaceable": true}),
        json!({"version": 1, "replaceable": false}),
    ] {
        let mut invalid = envelope(json!("invalid reminder"));
        invalid["_runtimeReminder"] = identity;
        assert!(manager
            .restore_session_messages(&session_id, vec![invalid.to_string()])
            .is_err());
    }
    Ok(())
}
