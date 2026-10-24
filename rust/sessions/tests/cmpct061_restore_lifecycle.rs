#[path = "cmpct060/support.rs"]
mod support;

use codelet_core::persistence::{
    append_message, get_session_message_envelopes, load_session, set_compaction_state,
    set_session_tokens, TokenUsage,
};
use codelet_core::SessionManagerHandle;
use rig::message::Message;
use support::TestResult;

async fn reconcile_suffix(persisted: &[&str], runtime: &[&str]) -> TestResult {
    let (_directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "persisted H")?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(background.id)?)?;
    let original = background.inner.lock().await.messages.clone();
    for text in persisted {
        append_message(&mut manifest, "user", text)?;
    }
    {
        let mut inner = background.inner.lock().await;
        inner
            .messages
            .extend(runtime.iter().copied().map(Message::user));
    }
    let envelopes = get_session_message_envelopes(background.id)?;
    manager.restore_session_messages(&session_id, envelopes.clone())?;
    let mut expected = original;
    expected.extend(persisted.iter().copied().map(Message::user));
    expected.extend(
        runtime
            .iter()
            .skip(persisted.len())
            .copied()
            .map(Message::user),
    );
    {
        let inner = background.inner.lock().await;
        assert_eq!(inner.messages, expected, "persisted suffix was duplicated");
        let installation = inner.restored_replay.as_ref().ok_or("installed replay")?;
        assert_eq!(installation.replay_len, 1 + persisted.len());
        assert_eq!(installation.revision, manifest.revision);
    }
    manager.restore_session_messages(&session_id, envelopes)?;
    assert_eq!(background.inner.lock().await.messages, expected);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn rewritten_restore_without_live_suffix_replaces_history() -> TestResult {
    let (_directory, manager, session_id, baseline) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "persisted H")?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(background.id)?)?;
    let prefix = background.inner.lock().await.messages[..baseline].to_vec();
    set_compaction_state(&mut manifest, "replacement summary".into(), 1)?;
    append_message(&mut manifest, "user", "persisted T")?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(background.id)?)?;
    let inner = background.inner.lock().await;
    assert_eq!(inner.messages.len(), baseline + 2);
    assert_eq!(inner.messages[..baseline], prefix);
    assert_eq!(inner.messages.last(), Some(&Message::user("persisted T")));
    assert!(inner.ensure_compaction_inference_ready().is_ok());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn advanced_restore_consumes_already_persisted_runtime_suffix() -> TestResult {
    reconcile_suffix(&["persisted T"], &["persisted T"]).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn advanced_restore_preserves_trailing_runtime_reminder() -> TestResult {
    reconcile_suffix(
        &["persisted T", "persisted U"],
        &["persisted T", "persisted U", "runtime reminder"],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn advanced_restore_consumes_partially_installed_persisted_tail() -> TestResult {
    reconcile_suffix(&["persisted T", "persisted U"], &["persisted T"]).await
}

async fn reject_ambiguous_suffix(runtime: &[&str], compact: bool) -> TestResult {
    let (_directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "persisted H")?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(background.id)?)?;
    {
        let mut inner = background.inner.lock().await;
        inner
            .messages
            .extend(runtime.iter().copied().map(Message::user));
    }
    if compact {
        set_compaction_state(&mut manifest, "replacement summary".into(), 1)?;
    }
    append_message(&mut manifest, "user", "persisted T")?;
    append_message(&mut manifest, "user", "persisted U")?;
    let before_messages = background.inner.lock().await.messages.clone();
    let before_installation = format!("{:?}", background.inner.lock().await.restored_replay);
    let before_output = serde_json::to_value(background.get_buffered_output(100))?;
    let mut events = background.subscribe_to_stream();
    let result = manager
        .restore_session_messages(&session_id, get_session_message_envelopes(background.id)?);
    assert!(result.is_err(), "ambiguous live suffix must conflict");
    assert!(
        result.err().ok_or("restore conflict")?.contains("suffix"),
        "restore conflict must identify the runtime suffix"
    );
    let inner = background.inner.lock().await;
    assert_eq!(inner.messages, before_messages);
    assert_eq!(format!("{:?}", inner.restored_replay), before_installation);
    assert!(
        inner.ensure_compaction_inference_ready().is_err(),
        "ambiguous restore must block inference until history is reconciled"
    );
    assert_eq!(
        serde_json::to_value(background.get_buffered_output(100))?,
        before_output
    );
    assert!(
        events.try_recv().is_err(),
        "failed restore published output"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn advanced_restore_rejects_unpersisted_runtime_suffix_without_mutation() -> TestResult {
    reject_ambiguous_suffix(&["unpersisted user turn"], false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn advanced_restore_rejects_interleaved_runtime_reminder_without_dropping_it() -> TestResult {
    reject_ambiguous_suffix(&["persisted T", "runtime reminder", "persisted U"], false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn rewritten_restore_rejects_live_suffix_without_dropping_it() -> TestResult {
    reject_ambiguous_suffix(&["runtime reminder"], true).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn metadata_only_revision_preserves_runtime_suffix() -> TestResult {
    let (_directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "persisted H")?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(background.id)?)?;
    {
        let mut inner = background.inner.lock().await;
        inner.messages.push(Message::user("runtime reminder"));
    }
    let before = background.inner.lock().await.messages.clone();
    let old_revision = manifest.revision;
    set_session_tokens(&mut manifest, &TokenUsage::default())?;
    assert!(manifest.revision > old_revision);
    manager.restore_session_messages(&session_id, get_session_message_envelopes(background.id)?)?;
    assert_eq!(background.inner.lock().await.messages, before);
    Ok(())
}
