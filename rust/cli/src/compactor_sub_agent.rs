//! CMPCT-044: Clean compaction sub-agent — the second stage of overflow
//! recovery.
//!
//! When the API rejects a request with a context-overflow error that the
//! in-loop in-view compaction (Paths A/B/C/D) could not resolve, the
//! stream loop escalates to a DeepSearch-style EPHEMERAL compactor
//! sub-agent:
//!
//! 1. The sub-agent runs in a fresh `Uuid::new_v4()` session with a clean
//!    context, inheriting the parent session's provider/model
//!    (`CompactorSubAgentHandler`, registered by `codelet-agent-loop` —
//!    same pattern as `DeepSearchHandler`). Its only view of the dying
//!    session is `SessionSearch` over the parent's session id.
//! 2. It returns the compaction DAG as its final response (it has NO
//!    `inject_summary` tool — the pin is handler-side, not a tool call).
//! 3. The trigger site performs the `apply_pending_dag` mutation in
//!    production code — see [`pin_dag_to_session`]
//!    (`reset_session_to_reminders` → push `wrap_dag_content` →
//!    `recalculate_token_tracker` → `reset_after_compaction`).
//! 4. On timeout / failure / unparseable DAG / missing handler, the
//!    trigger falls back to a FREE force-inject fallback DAG (CMPCT-020
//!    Level-3 shape) so the session is ALWAYS reduced — see
//!    [`run_compactor_sub_agent_round`].
//!
//! The handler registry lives here (in `codelet-cli`) so the stream loop
//! and `codelet-agent-loop` share one entry point without
//! `codelet-cli` depending on the spawner crate.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::Result;
use codelet_core::compaction::{parse_dag_nodes, wrap_dag_content};
use codelet_core::TokenState;
use uuid::Uuid;

use crate::compaction_dag::{
    build_recovered_or_generic_dag, detect_existing_dag, force_inject_fallback_dag,
};
use crate::interactive::output::StreamOutput;
use crate::interactive_helpers::{compression_ratio, recalculate_token_tracker};
use crate::session::Session;
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use tracing::{debug, warn};

/// Handler function type for the compactor sub-agent run.
///
/// Takes the parent (target) session id and the pre-captured existing DAG
/// `(content, max_turn_end)` — captured via `detect_existing_dag` BEFORE
/// any clear — and returns a future resolving to the DAG text the
/// sub-agent produced. The spawner lives in `codelet-agent-loop`
/// (mirroring `execute_deep_search`): ephemeral session, SessionSearch
/// over the parent id, provider inheritance (BUG-102), AMGR-016
/// wall-clock timeout, 7 read-only tools.
pub type CompactorSubAgentHandler = Arc<
    dyn Fn(
            Uuid,
            Option<(String, usize)>,
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

/// Per-session compactor sub-agent handler registry.
static COMPACTOR_SUB_AGENT_HANDLERS: once_cell::sync::Lazy<
    RwLock<HashMap<Uuid, CompactorSubAgentHandler>>,
> = once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Register (or clear, with `None`) the compactor sub-agent handler for a
/// session. Called by `codelet-agent-loop` at session creation / after
/// `/model` `/provider` changes; cleared in the end-of-turn cleanup block.
pub fn set_compactor_sub_agent_handler(
    session_id: Uuid,
    handler: Option<CompactorSubAgentHandler>,
) {
    if let Ok(mut guard) = COMPACTOR_SUB_AGENT_HANDLERS.write() {
        match handler {
            Some(h) => {
                guard.insert(session_id, h);
            }
            None => {
                guard.remove(&session_id);
            }
        }
    }
}

/// True iff a compactor sub-agent handler is registered for `session_id`.
pub fn has_compactor_sub_agent_handler(session_id: Uuid) -> bool {
    COMPACTOR_SUB_AGENT_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Test helper: drop all registered handlers.
pub fn clear_all_compactor_sub_agent_handlers() {
    if let Ok(mut guard) = COMPACTOR_SUB_AGENT_HANDLERS.write() {
        guard.clear();
    }
}

/// Pin a DAG to a session — the `apply_pending_dag` mutation with a DAG
/// string as the source instead of `pending_dag_content`.
///
/// 1. `reset_session_to_reminders` — in-memory messages become the system
///    reminders only; `session.turns` is cleared.
/// 2. Push the `wrap_dag_content`-wrapped DAG as a User message.
/// 3. `recalculate_token_tracker` — tracker reflects the reduced list.
/// 4. `reset_after_compaction()` — last-turn billing/cache fields reset.
///
/// The persisted session manifest history is intentionally NOT truncated:
/// the manifest is append-only history, so SessionSearch can still drill
/// down into pre-compaction turns later.
pub fn pin_dag_to_session(session: &mut Session, dag_content: &str) {
    let _counts = crate::interactive_helpers::reset_session_to_reminders(session);

    let wrapped = wrap_dag_content(dag_content);
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    });

    recalculate_token_tracker(session);
    session.token_tracker.reset_after_compaction();

    debug!(
        "[pin_dag_to_session] pinned — messages_len={}, tokens={}",
        session.messages.len(),
        session.token_tracker.input_tokens
    );
}

/// CMPCT-044: the compactor sub-agent escalation step, invoked by the
/// stream loop's `in_loop_compaction_restart!` macro for overflow rounds
/// after the in-view compaction (Round 1).
///
/// The caller (the macro) has ALREADY run `begin_compaction_recovery` and
/// the budget check for this round — this function only performs:
///
/// 1. Emit `compaction_progress("Compaction sub-agent working")`.
/// 2. Set the `compaction_in_progress` flag for the sub-agent run so
///    SessionSearch Layer-0 trimming applies to its reads of the parent
///    (CMPCT-044 Rule [5]).
/// 3. Look up the session's `CompactorSubAgentHandler`. On success, pin
///    the returned DAG via [`pin_dag_to_session`]. On any failure —
///    missing handler, timeout, LLM error, unparseable output — pin a
///    FREE fallback DAG (generic auto-recovered node) so the session is
///    ALWAYS reduced (convergence guarantee).
/// 4. Clear the flag after the pin and reset the shared `token_state` in
///    place (the same reset the macro performs after in-view compaction),
///    so the branch classifier and the fresh CompactionHook observe the
///    same Arc.
pub async fn run_compactor_sub_agent_round<O: StreamOutput>(
    session: &mut Session,
    session_id: Uuid,
    token_state: &Arc<Mutex<TokenState>>,
    compaction_in_progress: &Arc<AtomicBool>,
    output: &O,
) -> Result<()> {
    let existing_dag = detect_existing_dag(&session.messages);
    let total_turns = (session.messages.len() as u32).max(1);
    output.emit_compaction_progress("Compaction sub-agent working", 0, total_turns);

    // CMPCT-044 Rule [5]: the target's compaction_in_progress flag is set
    // for the whole sub-agent run so SessionSearch Layer-0 trimming
    // applies to the sub-agent's reads of the parent.
    compaction_in_progress.store(true, std::sync::atomic::Ordering::SeqCst);

    let handler = COMPACTOR_SUB_AGENT_HANDLERS
        .read()
        .map(|guard| guard.get(&session_id).cloned())
        .unwrap_or_default();

    let run_result: Result<String, String> = match handler {
        Some(h) => {
            debug!(
                "[run_compactor_sub_agent_round] running compactor sub-agent for session {session_id}"
            );
            h(session_id, existing_dag).await
        }
        None => {
            warn!(
                "[run_compactor_sub_agent_round] no compactor sub-agent handler registered \
                 for session {session_id} — force-injecting fallback DAG (zero LLM cost)"
            );
            Err("no compactor sub-agent handler registered for session {session_id}".to_string())
        }
    };

    // CMPCT-044 rule [9]: every fallback outcome surfaces the structured
    // `CompactionFailed(reason)` lifecycle event on the session's output
    // BEFORE the honest post-pin `CompactionComplete` — the session IS
    // reduced (the convergence guarantee is the pinned fallback DAG), but
    // the sub-agent path's failure must be visible to frontends (the
    // in-view path emits the same event for its own failures, so the
    // event contract is consistent across both stages).
    let failure_reason: Option<String> = match &run_result {
        Ok(dag_text) if parse_dag_nodes(dag_text, None).is_empty() => Some(
            "compactor sub-agent output had no parseable <dag-node> blocks — \
             reduced with the free fallback DAG"
                .to_string(),
        ),
        Ok(_) => None,
        Err(reason) => Some(format!("compactor sub-agent failed: {reason}")),
    };

    match run_result {
        Ok(dag_text) => {
            if parse_dag_nodes(&dag_text, None).is_empty() {
                warn!(
                    "[run_compactor_sub_agent_round] sub-agent output has no parseable \
                     <dag-node> blocks — falling back to the free force-inject DAG"
                );
                pin_fallback_dag(session, compaction_in_progress, &dag_text);
            } else {
                pin_dag_to_session(session, &dag_text);
            }
        }
        Err(reason) => {
            warn!(
                reason = %reason,
                "[run_compactor_sub_agent_round] sub-agent failed — force-injecting fallback DAG"
            );
            pin_fallback_dag(session, compaction_in_progress, &reason);
        }
    }

    // CMPCT-044 rule [9]: emit the structured compaction-failed event
    // before the CompactionComplete (frontends render the failure and
    // then the honest post-pin basis — the session was still reduced).
    if let Some(reason) = &failure_reason {
        output.emit_compaction_failed(reason);
    }

    // CMPCT-038 measurement rule: emit CompactionComplete with the honest
    // post-pin basis — `compacted_tokens` is the recalculated tracker
    // total after the pin, NOT just the DAG summary size. The
    // pre-compaction basis is the shared token state's input basis,
    // which `begin_compaction_recovery` flushed before this round.
    let original_tokens = token_state
        .lock()
        .map(|state| state.input_tokens)
        .unwrap_or(0);
    emit_compaction_complete(output, original_tokens, session);

    // Clear the flag after the pin completes (Rule [5] symmetry) and reset
    // the shared token state in place — the same reset the
    // `in_loop_compaction_restart!` macro performs after in-view compaction.
    compaction_in_progress.store(false, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut state) = token_state.lock() {
        state.compaction_needed = false;
        state.input_tokens = session.token_tracker.input_tokens;
        state.cache_read_input_tokens = 0;
        state.cache_creation_input_tokens = 0;
        state.output_tokens = 0;
    }

    Ok(())
}

/// CMPCT-044 Level-3 convergence guarantee: pin a recovered/auto-recovered
/// DAG with zero further LLM cost. Used when the compactor sub-agent times
/// out, fails, or returns an unparseable DAG.
///
/// `failed_output` is the sub-agent's final text (Ok with no parseable
/// nodes) or its failure reason (Err) — any complete `<dag-node>` blocks it
/// carries are recovered first; otherwise the generic auto-recovered node
/// is emitted (CMPCT-044 Rule [4] / CMPCT-020 Level-3 shape).
fn pin_fallback_dag(
    session: &mut Session,
    compaction_in_progress: &Arc<AtomicBool>,
    failed_output: &str,
) {
    let total_turns = (session.messages.len() as u32).max(1);
    let fallback_dag = build_recovered_or_generic_dag(
        failed_output,
        "Auto-recovered: context overflow",
        "Session was auto-compacted after a provider context-overflow error.",
        total_turns,
    );
    force_inject_fallback_dag(session, compaction_in_progress, &fallback_dag);
}

/// Emit `CompactionComplete` with the honest post-pin basis (CMPCT-038):
/// `compacted_tokens` is the recalculated tracker total after the pin,
/// NOT the DAG summary size. `original_tokens` is the pre-compaction
/// basis flushed by `begin_compaction_recovery` before this round.
fn emit_compaction_complete<O: StreamOutput>(output: &O, original_tokens: u64, session: &Session) {
    let compacted_tokens = session.token_tracker.input_tokens;
    let ratio = compression_ratio(original_tokens, compacted_tokens).max(0.0);
    output.emit_compaction_complete(
        original_tokens.try_into().unwrap_or(u32::MAX),
        compacted_tokens.try_into().unwrap_or(u32::MAX),
        ratio,
    );
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use codelet_core::TokenState;
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::interactive::output::StreamEvent;

    struct NullOutput;
    impl StreamOutput for NullOutput {
        fn emit(&self, _event: StreamEvent) {}
    }

    fn fresh_session() -> Session {
        Session::new(None).expect("test session must build")
    }

    fn seed_turns(session: &mut Session) {
        session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text("do some work")),
        });
        session.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(rig::message::Text {
                text: "done".to_string(),
            })),
        });
    }

    fn token_state() -> Arc<Mutex<TokenState>> {
        Arc::new(Mutex::new(TokenState {
            input_tokens: 190_000,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            output_tokens: 0,
            compaction_needed: true,
        }))
    }

    // Scenario: Successful sub-agent DAG is pinned to the parent session
    // @step Given a parent session with system reminders and many compactable conversation messages
    #[tokio::test]
    async fn successful_dag_is_pinned_to_the_session() {
        // @step And the compactor sub-agent returns a parseable DAG with at least one dag-node block
        let session_id = Uuid::new_v4();
        set_compactor_sub_agent_handler(
            session_id,
            Some(Arc::new(|_parent, existing| {
                assert!(
                    existing.is_none(),
                    "a session without a prior DAG must get the FRESH instruction"
                );
                Box::pin(async {
                    Ok(r#"<dag-node depth="D1" turns="0-5" label="Work arc">
- completed the task
</dag-node>"#
                        .to_string())
                })
            })),
        );

        let mut session = fresh_session();
        seed_turns(&mut session);
        session.token_tracker.input_tokens = 190_000;

        let compaction_in_progress = Arc::new(AtomicBool::new(false));
        let output = NullOutput;

        // @step When the trigger site pins the sub-agent's DAG to the parent
        run_compactor_sub_agent_round(
            &mut session,
            session_id,
            &token_state(),
            &compaction_in_progress,
            &output,
        )
        .await
        .expect("recovery must succeed");

        // @step Then the parent's in-memory messages are replaced by the system reminders plus the wrapped DAG
        let last = session.messages.last().expect("a message must remain");
        match last {
            Message::User { content } => {
                let text = content
                    .iter()
                    .find_map(|c| match c {
                        UserContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                assert!(
                    text.contains("<!-- type:compaction-dag -->"),
                    "the pinned message must carry the compaction-dag wrapper"
                );
                assert!(
                    text.contains("Work arc"),
                    "the pinned DAG must be the sub-agent's DAG"
                );
            }
            _ => panic!("the pinned message must be a User message"),
        }

        // @step And the parent's token tracker is recalculated from the reduced message list
        assert!(
            session.token_tracker.input_tokens < 190_000,
            "tracker must reflect the reduced context, got {}",
            session.token_tracker.input_tokens
        );

        // @step And the compaction_in_progress flag is false after the pin completes
        assert!(
            !compaction_in_progress.load(Ordering::SeqCst),
            "the flag must be cleared after the pin"
        );

        set_compactor_sub_agent_handler(session_id, None);
    }

    // Scenario: Missing compactor handler still guarantees a reduced session
    // @step Given a session for which no compactor-sub-agent handler is registered
    #[tokio::test]
    async fn missing_handler_pins_a_fallback_dag() {
        let session_id = Uuid::new_v4();
        assert!(!has_compactor_sub_agent_handler(session_id));

        let mut session = fresh_session();
        seed_turns(&mut session);
        session.token_tracker.input_tokens = 190_000;

        let compaction_in_progress = Arc::new(AtomicBool::new(false));
        let output = NullOutput;

        // @step When the trigger site attempts compactor recovery
        run_compactor_sub_agent_round(
            &mut session,
            session_id,
            &token_state(),
            &compaction_in_progress,
            &output,
        )
        .await
        .expect("recovery must succeed even without a handler");

        // @step Then a fallback DAG is force-injected into the session without any LLM call
        let last = session.messages.last().expect("a message must remain");
        match last {
            Message::User { content } => {
                let text = content
                    .iter()
                    .find_map(|c| match c {
                        UserContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                assert!(
                    text.contains("Auto-recovered: context overflow"),
                    "the fallback DAG must be pinned, got: {text}"
                );
            }
            _ => panic!("the pinned message must be a User message"),
        }
        assert!(
            session.token_tracker.input_tokens < 190_000,
            "the session must be reduced"
        );
    }

    // Scenario: Unparseable sub-agent output pins a fallback DAG
    // @step Given the compactor sub-agent returns final text containing no parseable dag-node blocks
    #[tokio::test]
    async fn unparseable_output_pins_a_fallback_dag() {
        let session_id = Uuid::new_v4();
        set_compactor_sub_agent_handler(
            session_id,
            Some(Arc::new(|_parent, _| {
                Box::pin(async { Ok("I could not build a DAG".to_string()) })
            })),
        );

        let mut session = fresh_session();
        seed_turns(&mut session);
        session.token_tracker.input_tokens = 190_000;

        let compaction_in_progress = Arc::new(AtomicBool::new(false));
        let output = NullOutput;

        // @step When the trigger site completes compaction
        run_compactor_sub_agent_round(
            &mut session,
            session_id,
            &token_state(),
            &compaction_in_progress,
            &output,
        )
        .await
        .expect("recovery must succeed");

        // @step Then a fallback DAG is pinned to the parent session
        let last = session.messages.last().expect("a message must remain");
        match last {
            Message::User { content } => {
                let text = content
                    .iter()
                    .find_map(|c| match c {
                        UserContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                assert!(
                    text.contains("Auto-recovered: context overflow"),
                    "unparseable output must fall back to the free DAG, got: {text}"
                );
            }
            _ => panic!("the pinned message must be a User message"),
        }

        set_compactor_sub_agent_handler(session_id, None);
    }
}
