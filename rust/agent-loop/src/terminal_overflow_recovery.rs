//! CMPCT-044: terminal-error-arm overflow recovery for background sessions.
//!
//! When a background agent-loop turn dies with a provider
//! context-overflow error the in-loop cascade could not resolve (the
//! stream returned `Err` to the agent loop), the terminal-error arm routes
//! it through the SAME compactor recovery entry point the stream loop
//! uses: `codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round`.
//!
//! The session's context is cleared to reminders plus a DAG (the
//! sub-agent's DAG, or the free fallback DAG on any failure), the session
//! returns to Idle, and the next user message starts a fresh stream on
//! the reduced context — the oversized payload is never replayed.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use codelet_cli::interactive_helpers::convert_messages_to_turns;
use codelet_core::TokenState;
use codelet_rpc_types::SessionStatus;
use codelet_sessions::background_session::BackgroundSession;

/// CMPCT-044: process a terminal stream error on a background session.
///
/// Classifies the error with the robust `is_context_overflow_error`
/// classifier, gates on compactable turns (PROV-010), and on a match runs
/// the shared compactor-sub-agent recovery round under the session's
/// inner lock. Returns `true` when the compactor round ran — the caller
/// must then set the session Idle and emit `Done` itself instead of
/// re-sending the oversized payload.
pub async fn try_terminal_overflow_recovery(
    session: Arc<BackgroundSession>,
    error: &anyhow::Error,
) -> bool {
    // PROV-010: only overflow with actual user/assistant turns to
    // compact — a session with only system-reminder messages follows the
    // existing terminal error handling.
    let (is_overflow, has_compactable_turns) = {
        let inner = session.inner.lock().await;
        (
            codelet_cli::interactive::is_context_overflow_error(error),
            !convert_messages_to_turns(&inner.messages).is_empty(),
        )
    };
    if !is_overflow || !has_compactable_turns {
        return false;
    }

    tracing::info!(
        "[CMPCT-044] terminal overflow on session {} — running the compactor \
         sub-agent recovery round",
        session.id
    );

    // The pre-compaction token basis (CMPCT-038) is the session's own
    // tracker total under the same lock — no shared token state is
    // involved in the terminal path.
    let mut session_inner = session.inner.lock().await;
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session_inner.token_tracker.input_tokens,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: true,
    }));

    let output = crate::background_output::BackgroundOutput::with_provider(
        session.clone(),
        session_inner.current_provider_name().to_string(),
        None,
    );

    codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round(
        &mut session_inner,
        session.id,
        &token_state,
        &session.compaction_in_progress,
        &output,
    )
    .await
    .expect("the compactor round always converges — the session must be reduced");
    drop(session_inner);

    // The session returns to Idle; the next user message starts a fresh
    // stream on the reduced context.
    session.set_status(SessionStatus::Idle);
    session.set_compaction_progress(None);
    session
        .compaction_in_progress
        .store(false, Ordering::SeqCst);
    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    /// Scenario: Compactor sub-agent is bounded by the shared AMGR-016
    /// wall-clock timeout (source-shape guard — the spawner reuses the
    /// deep-search timeout constant).
    #[test]
    fn terminal_path_shares_the_deep_search_wall_clock_timeout() {
        use codelet_cli::interactive::{
            deep_search_wall_clock_timeout, DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS,
        };
        assert_eq!(
            deep_search_wall_clock_timeout().as_secs(),
            DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS,
            "the terminal overflow path must reuse the shared AMGR-016 wall-clock timeout"
        );
    }
}
