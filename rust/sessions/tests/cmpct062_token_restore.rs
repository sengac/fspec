//! Feature: spec/features/committed-token-state-restore.feature

#[path = "cmpct060/support.rs"]
mod support;

use codelet_core::persistence::*;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{StreamChunk, TokenRestoreState};
use support::{cold_resume, manager, TestResult};

fn counters() -> TokenRestoreState {
    TokenRestoreState {
        current_context: 101,
        cumulative_billed_output: 29,
        cache_read: 43,
        cache_creation: 17,
        cumulative_billed_input: 503,
        cumulative_billed_output_second: 29,
    }
}

fn persist(session_id: &str) -> Result<SessionManifest, String> {
    let mut manifest =
        load_session(uuid::Uuid::parse_str(session_id).map_err(|error| error.to_string())?)?;
    set_session_tokens(
        &mut manifest,
        &TokenUsage {
            current_context_tokens: 101,
            cumulative_billed_input: 503,
            cumulative_billed_output: 29,
            cache_read_tokens: 43,
            cache_creation_tokens: 17,
            reasoning_tokens: 11,
        },
    )?;
    Ok(manifest)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn saved_context_spend_cache_and_reasoning_become_live_state() -> TestResult {
    // @step Given a committed manifest with distinct saved context spend cache and reasoning
    let (_directory, manager, session_id, _baseline) = manager().await?;
    persist(&session_id.value)?;
    drop(manager);
    // @step When a fresh session restores the matching saved counters
    let manager = cold_resume(&session_id)?;
    let mut chunks = manager.chunks_tx().subscribe();
    manager.restore_session_token_state(&session_id, counters())?;
    // @step Then live tracking cached fields and the emitted token update match the manifest
    let session = manager.get_session(&session_id.value)?;
    let inner = session.inner.lock().await;
    let tracker = &inner.token_tracker;
    assert_eq!(tracker.input_tokens, 101);
    assert_eq!(tracker.cumulative_billed_input, 503);
    assert_eq!(tracker.cumulative_billed_output, 29);
    assert_eq!(tracker.cache_read_input_tokens, Some(43));
    assert_eq!(tracker.cache_creation_input_tokens, Some(17));
    assert_eq!(tracker.reasoning_tokens, 11);
    assert_eq!(session.get_tokens(), (101, 29, Some(11)));
    let (_, StreamChunk::TokenUpdate { tokens }) = chunks.try_recv()? else {
        return Err("expected observable restored token update".into());
    };
    assert_eq!(tokens.input_tokens, 101);
    assert_eq!(tokens.cumulative_billed_input, Some(503));
    assert_eq!(tokens.cumulative_billed_output, Some(29));
    assert_eq!(tokens.cache_read_input_tokens, Some(43));
    assert_eq!(tokens.cache_creation_input_tokens, Some(17));
    assert_eq!(tokens.reasoning_tokens, Some(11));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn invalid_saved_counters_cannot_publish_restored_success() -> TestResult {
    // @step Given a committed manifest and incompatible or negative restore counters
    let (_directory, manager, session_id, _baseline) = manager().await?;
    persist(&session_id.value)?;
    let mut chunks = manager.chunks_tx().subscribe();
    for invalid in [-1, 100, i64::MAX] {
        let mut state = counters();
        state.current_context = invalid;
        // @step When token restoration is requested
        let result = manager.restore_session_token_state(&session_id, state);
        // @step Then restoration fails without changing live counters or emitting a token update
        assert!(result.is_err(), "incompatible counters must fail");
        assert_eq!(
            manager.get_session(&session_id.value)?.get_tokens(),
            (0, 0, None)
        );
        assert!(
            chunks.try_recv().is_err(),
            "invalid restore emitted usable token state"
        );
    }
    let uuid = uuid::Uuid::parse_str(&session_id.value)?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(uuid)?)?;
    while chunks.try_recv().is_ok() {}
    let mut manifest = load_session(uuid)?;
    append_message(&mut manifest, "user", "new generation suffix")?;
    let result = manager.restore_session_token_state(&session_id, counters());
    assert!(result.is_err(), "model and token revisions must agree");
    assert_eq!(
        manager.get_session(&session_id.value)?.get_tokens(),
        (0, 0, None)
    );
    assert!(
        chunks.try_recv().is_err(),
        "revision conflict emitted token success"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn repeated_token_restoration_does_not_accumulate_billing() -> TestResult {
    // @step Given a committed manifest with saved cumulative billing
    let (_directory, manager, session_id, _baseline) = manager().await?;
    persist(&session_id.value)?;
    // @step When matching token state is restored twice
    manager.restore_session_token_state(&session_id, counters())?;
    manager.restore_session_token_state(&session_id, counters())?;
    let session = manager.get_session(&session_id.value)?;
    let inner = session.inner.lock().await;
    // @step Then billing and context remain equal to one restore
    assert_eq!(inner.token_tracker.input_tokens, 101);
    assert_eq!(inner.token_tracker.cumulative_billed_input, 503);
    assert_eq!(inner.token_tracker.cumulative_billed_output, 29);
    Ok(())
}
